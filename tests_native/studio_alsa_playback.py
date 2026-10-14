#!/usr/bin/env python3
"""Explicit, finite direct ALSA silence diagnostic; no capture or route apply."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import signal
import struct
import subprocess
import sys
import time

import studio_playback as playback

RATE = 48000
CHANNELS = 8
DURATION = 4


class Cancellation:
    def __init__(self):
        self.requested = False

    def request(self, signum, frame):
        # Do not throw from a signal handler while owned cleanup is running.
        self.requested = True

    def check(self):
        if self.requested:
            raise RuntimeError("direct ALSA diagnostic cancellation requested")


def checked_rate(rate):
    if type(rate) is not int or rate not in (48000, 96000):
        raise ValueError("direct ALSA diagnostic supports only 48000 or measured 96000 Hz")
    return rate


def check_held_arguments(pid, start):
    if pid is None and start is None:
        return
    if (type(pid) is not int or pid < 1 or not isinstance(start, str)
            or re.fullmatch(r"[0-9]{1,20}", start) is None or int(start) < 1):
        raise ValueError("held vendor coexistence requires a positive PID and exact process start identity")


def pcm(rate=RATE, audible=False):
    # IEEE float positive zero is all-zero bytes, on every selected channel.
    checked_rate(rate)
    if not audible:
        return bytes(rate * CHANNELS * 4 * DURATION)
    data = bytearray(rate * CHANNELS * 4 * DURATION)
    for second in (1, 2, 3):
        frames = rate * 35 // 100
        ramp = rate // 100
        for i in range(frames):
            envelope = min(1.0, i / ramp, (frames - 1 - i) / ramp)
            value = 10 ** (-24 / 20) * envelope * math.sin(2 * math.pi * 440 * i / rate)
            struct.pack_into("<ff", data, (second * rate + i) * CHANNELS * 4, value, value)
    return bytes(data)


def check_authorization(silence, tone, audible):
    if bool(silence) == bool(tone) or bool(tone) != bool(audible):
        raise ValueError("select explicit silence authorization or audible first-pair tone authorization")


def parameters(raw, rate=RATE):
    expected = {"format": "FLOAT_LE", "channels": str(CHANNELS), "rate": str(checked_rate(rate))}
    for key, value in expected.items():
        match = re.search(r"^" + key + r": (\S+)", raw, re.M)
        if match is None or match.group(1) != value:
            raise RuntimeError("direct ALSA hardware format does not match the selected silence")


def clock(raw, pid, allow_draining=False):
    owner = re.search(r"^owner_pid\s+: (\d+)$", raw, re.M)
    state = re.search(r"^state: (\w+)$", raw, re.M)
    pointer = re.search(r"^hw_ptr\s+: (\d+)$", raw, re.M)
    if len(raw) > 16384 or owner is None or int(owner.group(1)) != pid:
        raise RuntimeError("selected PCM is not owned by the retained ALSA child")
    allowed = ("RUNNING", "DRAINING") if allow_draining else ("RUNNING",)
    if state is None or state.group(1) not in allowed or pointer is None:
        raise RuntimeError("selected direct ALSA PCM is not running")
    return int(pointer.group(1))


def native_card_endpoint(objects, card):
    """Pin the selected raw ALSA card without requiring a particular UCM layout."""
    candidates = [obj for obj in objects if obj.get("type") == "PipeWire:Interface:Device"
                  and playback.number(playback.inventory.properties(obj).get("device.vendor.id", 0)) == 0x33ae
                  and playback.number(playback.inventory.properties(obj).get("device.product.id", 0)) == 0x4003]
    if len(candidates) != 1 or playback.number(
            playback.inventory.properties(candidates[0]).get("api.alsa.card", -1)) != card:
        raise RuntimeError("selected native USB audio card is absent or ambiguous")
    return {"device": playback.graph.ref(candidates[0]), "direct_alsa_card": card}


def require_native_environment():
    native = subprocess.run(["systemd-detect-virt"], capture_output=True, text=True, timeout=3)
    if native.returncode != 1 or native.stdout.strip() != "none":
        raise RuntimeError("native direct-card mode requires a confirmed nonvirtual environment")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    authorization = parser.add_mutually_exclusive_group()
    authorization.add_argument("--authorize-direct-alsa-silence", action="store_true")
    authorization.add_argument("--authorize-direct-alsa-tone", action="store_true")
    parser.add_argument("--audible-first-pair", action="store_true",
                        help="three quiet 440-Hz tones on channels 0/1; requires owner ready to listen")
    parser.add_argument("--matched-device-clock-96000", action="store_true",
                        help="explicit diagnostic for an independently observed 96000-Hz device clock")
    parser.add_argument("--sysfs-device", required=True)
    parser.add_argument("--evidence-dir", required=True)
    parser.add_argument("--held-vendor-pid", type=int, help="caller-owned initialized helper; never stopped here")
    parser.add_argument("--held-vendor-start")
    parser.add_argument("--native-direct-card", action="store_true",
                        help="native-only raw PCM check; preserves other devices and all graph rates")
    args = parser.parse_args()
    if sys.platform != "linux":
        parser.error("requires Linux")
    try:
        check_authorization(args.authorize_direct_alsa_silence, args.authorize_direct_alsa_tone,
                            args.audible_first_pair)
        check_held_arguments(args.held_vendor_pid, args.held_vendor_start)
    except ValueError as error:
        parser.error(str(error))
    rate = 96000 if args.matched_device_clock_96000 else RATE
    if args.native_direct_card:
        try:
            require_native_environment()
        except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
            parser.error(str(error))
    root = Path(args.evidence_dir).resolve()
    if root == playback.REPO or playback.REPO in root.parents:
        parser.error("evidence must be outside the source tree")
    root.mkdir(mode=0o700, parents=False, exist_ok=False)
    journal = {"audible": args.audible_first_pair, "rate": rate, "channels": CHANNELS,
               "tone_pair": [0, 1] if args.audible_first_pair else None,
               "tone_peak_dbfs": -24 if args.audible_first_pair else None,
               "progress_kind": "alsa_queue_pointer", "independent_delivery_evidence": False,
               "matched_device_clock_requested": args.matched_device_clock_96000,
               "held_vendor_coexistence": args.held_vendor_pid is not None,
               "native_direct_card": args.native_direct_card,
               "graph_rate_forced": False,
               "duration": DURATION, "pipewire_playback_stream": False,
               "native_executor_apply": False, "child_exited": True,
               "clock_checks": 0, "error": None}
    child = log = None
    baseline = None
    cancellation = Cancellation()
    previous_signal = signal.signal(signal.SIGTERM, cancellation.request)
    get_binding = lambda: playback.binding(args.sysfs_device, args.held_vendor_pid, args.held_vendor_start)
    try:
        binding = get_binding()
        before = playback.owner.snapshot()
        objects = playback.collect()
        selected = Path(args.sysfs_device).resolve(strict=True)
        cards = [int(p.name[4:]) for p in Path("/sys/class/sound").glob("card[0-9]*")
                 if selected in (p / "device").resolve(strict=True).parents]
        if len(cards) != 1:
            raise RuntimeError("selected USB device does not uniquely identify an ALSA card")
        card = cards[0]
        if args.native_direct_card:
            pinned = native_card_endpoint(objects, card)
        else:
            pinned = playback.endpoint(objects, card, 1)
            playback.require_rate_isolation(objects, pinned["device"]["id"])
        alsa = Path("/proc/asound") / ("card" + str(card))
        for direction in ("p", "c"):
            if (alsa / ("pcm0" + direction) / "sub0/status").read_text().strip() != "closed":
                raise RuntimeError("selected ALSA card is already in use")
        device = Path("/dev/snd") / ("pcmC%dD0p" % card)
        stamp = device.stat()
        identity = (stamp.st_dev, stamp.st_ino, stamp.st_rdev)
        if get_binding() != binding or playback.owner.snapshot() != before:
            raise RuntimeError("baseline changed during direct ALSA preparation")
        baseline = {"snapshot": before, "binding": binding, "endpoint": pinned}
        playback.owner.save(root / "baseline.json", baseline)
        playback.owner.save(root / "baseline-objects.json", objects)
        binary = Path("/usr/bin/aplay").resolve(strict=True)
        raw = pcm(rate, audible=args.audible_first_pair)
        playback.owner.save_bytes(root / "authored-pcm.floatle", raw)
        journal.update(binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                       pcm_sha256=hashlib.sha256(raw).hexdigest(),
                       harness_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
        command = [str(binary), "--quiet", "--nonblock", "--disable-resample",
                   "--disable-channels", "--disable-format", "--disable-softvol",
                   "--device", "hw:%d,0" % card, "--file-type", "raw",
                   "--format", "FLOAT_LE", "--channels", str(CHANNELS),
                   "--rate", str(rate), "--duration", str(DURATION),
                   str(root / "authored-pcm.floatle")]
        log = (root / "aplay.log").open("xb")
        playback.owner.save(root / "run-started.json", {"time": time.time(), "direct_alsa": True})
        cancellation.check()
        child = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=log, stderr=log,
                                 env=dict(os.environ, ALSA_CONFIG_PATH="/usr/share/alsa/alsa.conf"))
        journal.update(owned_pid=child.pid, child_exited=False)
        started = last_progress = time.monotonic()
        last_pointer = None
        while child.poll() is None:
            cancellation.check()
            elapsed = time.monotonic() - started
            if elapsed > DURATION + 3 or log.tell() > 65536:
                raise RuntimeError("direct ALSA child exceeded its time or log bound")
            current = device.stat()
            if (current.st_dev, current.st_ino, current.st_rdev) != identity:
                raise RuntimeError("selected ALSA device node changed")
            current_objects = playback.collect()
            if playback.owner.core.filtered(current_objects) != before["desktop"]:
                raise RuntimeError("foreign PipeWire graph changed during direct ALSA test")
            if args.native_direct_card:
                if native_card_endpoint(current_objects, card) != pinned:
                    raise RuntimeError("selected native ALSA card identity changed")
            else:
                playback.endpoint(current_objects, card, 1)
                playback.require_rate_isolation(current_objects, pinned["device"]["id"])
            status = (alsa / "pcm0p/sub0/status").read_text()
            if status.strip() == "closed" and child.poll() is not None:
                break
            sufficient_progress = last_pointer is not None and last_pointer >= rate * (DURATION - 1)
            if status.strip() == "closed" and (elapsed < 1 or sufficient_progress):
                time.sleep(0.1)
                continue
            pointer = clock(status, child.pid, allow_draining=sufficient_progress)
            params = (alsa / "pcm0p/sub0/hw_params").read_text()
            parameters(params, rate)
            activation = playback.audio_activation(args.sysfs_device, binding)
            journal["clock_checks"] += 1
            playback.owner.save(root / ("clock-%02d.json" % journal["clock_checks"]),
                                {"elapsed": elapsed, "status": status, "hw_params": params,
                                 "activation": activation})
            if last_pointer is None:
                journal["first_hw_pointer"] = pointer
            if last_pointer is None or pointer > last_pointer:
                last_progress = time.monotonic()
            elif pointer < last_pointer:
                raise RuntimeError("direct ALSA queue pointer restarted")
            journal["last_hw_pointer"] = pointer
            last_pointer = pointer
            if time.monotonic() - last_progress > 2:
                raise RuntimeError("direct ALSA queue pointer stalled for two seconds")
            time.sleep(0.2)
        if child.returncode or last_pointer is None or last_pointer < rate * (DURATION - 1):
            raise RuntimeError("direct ALSA did not complete sufficient queue-progress-checked silence")
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        journal["error"] = str(error)
    finally:
        if child is not None:
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    child.kill()
                    try:
                        child.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        pass
            journal.update(child_exited=child.poll() is not None, exit=child.returncode)
        if log is not None:
            log.close()
        if baseline is not None and journal["child_exited"]:
            comparison = playback.settled_comparison(baseline, get_binding)
            playback.owner.save(root / "comparison.json", comparison)
            journal["comparison"] = {k: v for k, v in comparison.items() if k != "after"}
        playback.owner.save(root / "journal.json", journal)
        signal.signal(signal.SIGTERM, previous_signal)
    print(json.dumps(journal))
    return 0 if (journal["error"] is None and journal["child_exited"] and all(
        journal.get("comparison", {}).get(k) for k in
        ("settings_routes_exact_equal", "state_files_exact_equal", "connected_binding_equal"))) else 2


if __name__ == "__main__":
    raise SystemExit(main())
