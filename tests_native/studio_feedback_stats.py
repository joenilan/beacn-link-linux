#!/usr/bin/env python3
"""Bounded selected-device aggregate completion metadata; never read payloads."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import time

import studio_playback as playback
import usb_feedback_review

MAX_PACKETS = 8
WINDOW_SECONDS = 30


def probe_program(bus, device):
    if type(bus) is not int or not 1 <= bus <= 999 or type(device) is not int or not 1 <= device <= 127:
        raise ValueError("invalid selected USB bus/device metadata")
    selected = ("$u->dev->bus->busnum == %d && $u->dev->devnum == %d && "
                "$u->dev->descriptor.idVendor == 0x33ae && "
                "$u->dev->descriptor.idProduct == 0x4003" % (bus, device))
    return r'''config = { max_map_keys = 4096; }
BEGIN { @start = nsecs; }
kprobe:snd_usb_audio:snd_complete_urb
{
    $u = (struct urb *)arg0;
    if (SELECTED) {
        $ep = $u->ep->desc.bEndpointAddress;
        if (($ep == 2 || $ep == 130) && ($u->ep->desc.bmAttributes & 3) == 1) {
            $second = (nsecs - @start) / 1000000000;
            $status = $u->status;
            $bucket = 6;
            if ($status == 0) { $bucket = 0; }
            else if ($status == -18) { $bucket = 1; }
            else if ($status == -71) { $bucket = 2; }
            else if ($status == -75) { $bucket = 3; }
            else if ($status == -110) { $bucket = 4; }
            else if ($status == -2 || $status == -19 || $status == -104 || $status == -108) { $bucket = 5; }
            @completions[$second, $ep, $bucket] = count();
            $packets = (uint64)$u->number_of_packets;
            if ($bucket == 5) {
                // The ALSA callback returns before considering cancelled packets.
            } else if ($packets < 1 || $packets > PACKET_LIMIT) {
                @unsupported_urbs[$second, $ep] = count();
            } else {
                $i = 0;
                unroll(PACKET_LIMIT) {
                    if ($i < $packets) {
                        $ps = $u->iso_frame_desc[$i].status;
                        $length = $u->iso_frame_desc[$i].actual_length;
                        $requested = $u->iso_frame_desc[$i].length;
                        $pb = 6;
                        if ($ps == 0) { $pb = 0; }
                        else if ($ps == -18) { $pb = 1; }
                        else if ($ps == -71) { $pb = 2; }
                        else if ($ps == -75) { $pb = 3; }
                        else if ($ps == -110) { $pb = 4; }
                        else if ($ps == -2 || $ps == -19 || $ps == -104 || $ps == -108) { $pb = 5; }
                        @packets[$second, $ep, $pb] = count();
                        if ($requested > 0 && $requested <= 416 && $requested % 32 == 0) {
                            @requested_lengths[$ep, $pb, $requested] = count();
                        } else { @invalid_requests[$ep] = count(); }
                        if ($ps == 0) {
                            if ($length == 0) { @empty_packets[$second, $ep] = count(); }
                            else if ($length > 416) { @invalid_lengths[$second, $ep] = count(); }
                            else { @successful_bytes[$second, $ep] = sum($length); }
                        }
                    }
                    $i++;
                }
            }
        }
    }
}
kprobe:snd_usb_audio:snd_usb_handle_sync_urb
{
    $u = (struct urb *)arg2;
    if (SELECTED && $u->ep->desc.bEndpointAddress == 130) {
        @sync_callbacks[(nsecs - @start) / 1000000000] = count();
        $sink = (struct snd_usb_endpoint *)arg0;
        $sender = (struct snd_usb_endpoint *)arg1;
        if ($sink->ep_num == 2 && $sender->ep_num == 130 &&
            ($sender->cur_rate == 48000 || $sender->cur_rate == 96000) &&
            ($sink->cur_rate == 48000 || $sink->cur_rate == 96000) &&
            $sender->maxpacksize == 416 && $sink->maxpacksize == 416 &&
            $sender->curpacksize > 0 && $sender->curpacksize <= 416 && $sender->curpacksize % 32 == 0 &&
            $sink->curpacksize > 0 && $sink->curpacksize <= 416 && $sink->curpacksize % 32 == 0) {
            @sync_configuration[$sender->cur_rate, $sender->curpacksize, $sender->maxpacksize,
                                $sink->cur_rate, $sink->curpacksize, $sink->maxpacksize] = count();
        } else { @invalid_configuration = count(); }
    }
}
kprobe:snd_usb_audio:get_sample_rate_v2v3
{
    $u = (struct snd_usb_audio *)arg0;
    if (SELECTED && arg3 == 41) { @clock_pending[tid] = 1; }
}
kretprobe:snd_usb_audio:get_sample_rate_v2v3
{
    if (@clock_pending[tid]) {
        $rate = (int32)retval;
        if ($rate == 48000 || $rate == 96000) { @device_clock_rates[$rate] = count(); }
        else { @clock_unknown = count(); }
        $removed = delete(@clock_pending, tid);
        if (!$removed) { @clock_unknown = count(); }
    }
}
interval:s:30 { @window_limit = count(); exit(); }
END { clear(@start); clear(@clock_pending); }
'''.replace("SELECTED", selected).replace("PACKET_LIMIT", str(MAX_PACKETS))


def tracer_command(program, audio_child, *, attachment_check=False):
    # Check the identical load options and -c context, including helper warnings.
    command = ["/usr/bin/bpftrace", "-k", "-f", "json", "-B", "line", "-e", program, "-c", audio_child]
    if attachment_check:
        command.append("--dry-run")
    return command


def observer_outcome(exit_code, marker, journal):
    started = "owned_pid" in journal if journal is not None else (None if marker else False)
    return {"probe_exit": exit_code, "audio_started": started,
            "audio_launch_marker_present": marker, "playback_journal_present": journal is not None,
            "error": None if exit_code in (None, 0) else "aggregate observer exited %d; review retained diagnostics" % exit_code}


def child_command(uid, gid, home, source, selected, evidence):
    if type(uid) is not int or uid < 1 or type(gid) is not int or gid < 0:
        raise ValueError("audio child must use a non-root owner identity")
    command = ["/usr/bin/setpriv", "--reuid", str(uid), "--regid", str(gid), "--init-groups",
               "/usr/bin/env", "-i", "HOME=" + str(home), "PATH=/usr/bin:/bin",
               "XDG_RUNTIME_DIR=/run/user/" + str(uid), "/usr/bin/python3",
               str(Path(source) / "tests_native/studio_alsa_playback.py"),
               "--authorize-direct-alsa-silence", "--sysfs-device", str(selected),
               "--evidence-dir", str(evidence)]
    if os.environ.get("XDG_STATE_HOME"):
        command.insert(command.index("/usr/bin/python3"), "XDG_STATE_HOME=" + os.environ["XDG_STATE_HOME"])
    return shlex.join(command)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sysfs-device", required=True)
    parser.add_argument("--evidence-dir", required=True)
    parser.add_argument("--authorize-feedback-status-silence", action="store_true")
    parser.add_argument("--interactive-sudo", action="store_true")
    parser.add_argument("--compile-only", action="store_true")
    args = parser.parse_args()
    if sys.platform != "linux" or os.getuid() == 0 or os.geteuid() != os.getuid():
        parser.error("run as the normal Linux desktop owner, never as root")
    if args.interactive_sudo and not args.authorize_feedback_status_silence:
        parser.error("interactive sudo requires explicit status/silence execution")
    if args.compile_only and args.authorize_feedback_status_silence:
        parser.error("compile-only cannot launch silence")
    root = Path(args.evidence_dir).resolve()
    if root == playback.REPO or playback.REPO in root.parents:
        parser.error("evidence must remain outside source")
    root.mkdir(mode=0o700, parents=False, exist_ok=False)
    summary = {"audio_started": False, "aggregate_only": True, "payload_reads": False,
               "live_probe_validated": False, "error": None}
    child = None
    baseline = None
    try:
        selected = Path(args.sysfs_device).resolve(strict=True)
        binding = playback.binding(selected)
        before = playback.owner.snapshot()
        objects = playback.collect()
        cards = [int(p.name[4:]) for p in Path("/sys/class/sound").glob("card[0-9]*")
                 if selected in (p / "device").resolve(strict=True).parents]
        if len(cards) != 1:
            raise RuntimeError("selected device must have one ALSA card")
        pins = [playback.endpoint(objects, cards[0], i) for i in range(1, 5)]
        playback.require_rate_isolation(objects, pins[0]["device"]["id"])
        for direction in ("p", "c"):
            status = Path("/proc/asound") / ("card%d" % cards[0]) / ("pcm0" + direction) / "sub0/status"
            if status.read_text().strip() != "closed":
                raise RuntimeError("selected PCM is already in use")
        baseline = {"snapshot": before, "binding": binding, "endpoint": pins[0]}
        playback.owner.save(root / "baseline.json", {**baseline, "endpoints": pins})
        program = probe_program(int((selected / "busnum").read_text()), int((selected / "devnum").read_text()))
        playback.owner.save_bytes(root / "observer.bt", program.encode())
        dependencies = ("tests_native/studio_feedback_stats.py", "tests_native/studio_alsa_playback.py",
                        "tests_native/studio_playback.py", "tests_native/usb_handshake.py",
                        "tests_native/private_core.py", "tools/pipewire_inventory.py",
                        "tools/pipewire_graph.py", "tools/usb_descriptor_probe.py", "tools/usb_feedback_review.py")
        hashes = {n: hashlib.sha256((playback.REPO / n).read_bytes()).hexdigest() for n in dependencies}
        playback.owner.save(root / "source-hashes.json", hashes)
        if not args.compile_only and not args.authorize_feedback_status_silence:
            summary["prepared_only"] = True
        elif playback.binding(selected) != binding or playback.owner.snapshot() != before:
            raise RuntimeError("baseline changed during observer preparation")
        audio_child = child_command(os.getuid(), os.getgid(), Path.home(), playback.REPO, selected, root / "playback")
        privilege = ["/usr/bin/sudo"] + ([] if args.interactive_sudo else ["-n"])
        command = ["/usr/bin/bpftrace", "-d", "codegen", "-e", program] if args.compile_only else (
            privilege + tracer_command(program, audio_child))
        env = {"PATH": "/usr/bin:/bin", "HOME": str(Path.home()), "LANG": "C"}
        if summary.get("prepared_only"):
            command = None
        elif not args.compile_only:
            check_command = privilege + tracer_command(program, audio_child, attachment_check=True)
            playback.owner.save(root / "execution-options.json", {"packet_bound": MAX_PACKETS,
                                 "matched_preflight_runtime_flags": True, "preflight_runs_audio": False})
            with (root / "attach-check.stdout").open("xb") as out, (root / "attach-check.stderr").open("xb") as err:
                child = subprocess.Popen(check_command, stdin=None if args.interactive_sudo else subprocess.DEVNULL,
                                         stdout=out, stderr=err, env=env)
                child.wait(timeout=60 if args.interactive_sudo else 20)
            summary["attach_check_exit"] = child.returncode
            if child.returncode != 0:
                # One diagnostic-only load, never an audio/I/O retry. Keep full verifier evidence.
                diagnostic = check_command + ["-v"]
                with (root / "attach-diagnostic.stdout").open("xb") as out, (root / "attach-diagnostic.stderr").open("xb") as err:
                    child = subprocess.Popen(diagnostic, stdin=None if args.interactive_sudo else subprocess.DEVNULL,
                                             stdout=out, stderr=err, env=env)
                    child.wait(timeout=20)
                summary["attach_diagnostic_exit"] = child.returncode
                raise RuntimeError("probe attachment check failed; no audio child was launched")
            summary["probe_attach_check_passed"] = True
            if playback.binding(selected) != binding or playback.owner.snapshot() != before:
                raise RuntimeError("baseline changed during probe attachment check")
        with (root / "observer.stdout").open("xb") as stdout, (root / "observer.stderr").open("xb") as stderr:
            if command is None:
                # Preparation never invokes bpftrace, sudo or an audio child.
                summary["probe_exit"] = None
            else:
                child = subprocess.Popen(command, stdin=None if args.interactive_sudo else subprocess.DEVNULL,
                                         stdout=stdout, stderr=stderr, env=env)
            started = time.monotonic()
            while child is not None and child.poll() is None:
                if time.monotonic() - started > 50 or stdout.tell() + stderr.tell() > 2 * 1024 * 1024:
                    raise RuntimeError("owned observer exceeded its time/output bound")
                time.sleep(.1)
            for handle in (stdout, stderr):
                handle.flush()
                os.fsync(handle.fileno())
        summary.update(compile_only=args.compile_only,
                       stderr_present=(root / "observer.stderr").stat().st_size != 0)
        journal = root / "playback/journal.json"
        marker = (root / "playback/run-started.json").exists()
        result = json.loads(journal.read_text()) if journal.is_file() else None
        summary.update(observer_outcome(child.returncode if child is not None else None, marker, result))
        if journal.is_file():
            playback.owner.save(root / "playback-result.json", result)
        # A successful compiler/exit code alone never establishes complete probe coverage.
        summary["aggregate_review_required"] = not args.compile_only and child is not None and child.returncode == 0
        if summary["aggregate_review_required"]:
            with (root / "observer.stdout").open("rb") as handle:
                raw = handle.read(usb_feedback_review.MAX_BYTES + 1)
            reviewed = usb_feedback_review.review(raw, max_packets=MAX_PACKETS)
            playback.owner.save(root / "aggregate-review.json", reviewed)
            summary["aggregate_review_required"] = (reviewed["needs_review"] or summary["stderr_present"] or
                                                     not reviewed["has_request_geometry"] or
                                                     not reviewed["has_sync_configuration"] or
                                                     not reviewed["device_clock_rates"] or
                                                     reviewed["attached_probes"] != 7)
            summary["live_probe_validated"] = (not summary["aggregate_review_required"] and
                                                reviewed["sync_callbacks"] > 0)
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        summary["error"] = str(error)
    finally:
        if child is not None and child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=6)
            except subprocess.TimeoutExpired:
                summary["remote_root_child_outcome_unknown"] = True
        if baseline is not None:
            comparison = playback.settled_comparison(baseline, lambda: playback.binding(args.sysfs_device))
            playback.owner.save(root / "comparison.json", comparison)
            summary["comparison"] = {k: v for k, v in comparison.items() if k != "after"}
        playback.owner.save(root / "summary.json", summary)
        print(json.dumps(summary))
    restored = all(summary.get("comparison", {}).get(k) for k in
                   ("settings_routes_exact_equal", "state_files_exact_equal", "connected_binding_equal"))
    return 0 if summary["error"] is None and (summary.get("prepared_only") or summary.get("probe_exit") == 0) and restored else 2


if __name__ == "__main__":
    raise SystemExit(main())
