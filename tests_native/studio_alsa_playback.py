#!/usr/bin/env python3
"""Explicit, finite direct ALSA silence diagnostic; no capture or route apply."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
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


def pcm(rate=RATE):
    # IEEE float positive zero is all-zero bytes, on every selected channel.
    return bytes(checked_rate(rate) * CHANNELS * 4 * DURATION)


def parameters(raw, rate=RATE):
    expected = {"format": "FLOAT_LE", "channels": str(CHANNELS), "rate": str(checked_rate(rate))}
    for key, value in expected.items():
        match = re.search(r"^" + key + r": (\S+)", raw, re.M)
        if match is None or match.group(1) != value:
            raise RuntimeError("direct ALSA hardware format does not match the selected silence")


def clock(raw, pid):
    owner = re.search(r"^owner_pid\s+: (\d+)$", raw, re.M)
    state = re.search(r"^state: (\w+)$", raw, re.M)
    pointer = re.search(r"^hw_ptr\s+: (\d+)$", raw, re.M)
    if len(raw) > 16384 or owner is None or int(owner.group(1)) != pid:
        raise RuntimeError("selected PCM is not owned by the retained ALSA child")
    if state is None or state.group(1) != "RUNNING" or pointer is None:
        raise RuntimeError("selected direct ALSA PCM is not running")
    return int(pointer.group(1))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--authorize-direct-alsa-silence", action="store_true")
    parser.add_argument("--matched-device-clock-96000", action="store_true",
                        help="explicit diagnostic for an independently observed 96000-Hz device clock")
    parser.add_argument("--sysfs-device", required=True)
    parser.add_argument("--evidence-dir", required=True)
    args = parser.parse_args()
    if sys.platform != "linux" or not args.authorize_direct_alsa_silence:
        parser.error("requires Linux and explicit direct ALSA silence authorization")
    rate = 96000 if args.matched_device_clock_96000 else RATE
    root = Path(args.evidence_dir).resolve()
    if root == playback.REPO or playback.REPO in root.parents:
        parser.error("evidence must be outside the source tree")
    root.mkdir(mode=0o700, parents=False, exist_ok=False)
    journal = {"audible": False, "rate": rate, "channels": CHANNELS,
               "progress_kind": "alsa_queue_pointer", "independent_delivery_evidence": False,
               "matched_device_clock_requested": args.matched_device_clock_96000,
               "duration": DURATION, "pipewire_playback_stream": False,
               "native_executor_apply": False, "child_exited": True,
               "clock_checks": 0, "error": None}
    child = log = None
    baseline = None
    cancellation = Cancellation()
    previous_signal = signal.signal(signal.SIGTERM, cancellation.request)
    get_binding = lambda: playback.owner.connected_binding(args.sysfs_device)
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
        raw = pcm(rate)
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
            playback.endpoint(current_objects, card, 1)
            playback.require_rate_isolation(current_objects, pinned["device"]["id"])
            status = (alsa / "pcm0p/sub0/status").read_text()
            if status.strip() == "closed" and child.poll() is not None:
                break
            if status.strip() == "closed" and elapsed < 1:
                time.sleep(0.1)
                continue
            pointer = clock(status, child.pid)
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
