#!/usr/bin/env python3
"""Read-only Linux feasibility preflight; saves private evidence, never plays audio."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time

import studio_playback as playback


def environment():
    if shutil.which("systemd-detect-virt") is None:
        return "unknown"
    result = subprocess.run(["systemd-detect-virt"], capture_output=True, text=True, timeout=3)
    value = result.stdout.strip()
    if result.returncode == 1 and value == "none":
        return "none"
    return value if result.returncode == 0 and value else "unknown"


def ready(summary):
    return all(summary.get(key) is True for key in (
        "native_environment", "released_usb_profile", "unique_selected_card",
        "all_four_endpoints_verified", "isolated_graph", "alsa_playback_closed",
        "alsa_capture_closed", "alsa_access", "aplay_available"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-dir", required=True)
    args = parser.parse_args()
    if sys.platform != "linux":
        parser.error("requires a Linux desktop session")
    root = Path(args.evidence_dir).resolve()
    if root == playback.REPO or playback.REPO in root.parents:
        parser.error("private evidence must be outside this source bundle")
    root.mkdir(mode=0o700, parents=False, exist_ok=False)
    summary = {"mode": "read_only", "playback_started": False,
               "native_executor_apply": False, "aplay_available": shutil.which("aplay") is not None,
               "python": platform.python_version(), "kernel": platform.release(), "errors": []}
    try:
        kind = environment()
        summary.update(virtualization=kind, native_environment=kind == "none")
        before = playback.owner.snapshot()
        objects = playback.collect()
        playback.owner.save(root / "baseline.json", {"snapshot": before, "objects": objects,
                                                      "time": time.time()})
        targets = playback.owner.targets()
        summary["selected_device_count"] = len(targets)
        if len(targets) != 1:
            raise RuntimeError("selected Studio USB device is absent or ambiguous")
        selected = targets[0].resolve(strict=True)
        binding = playback.owner.connected_binding(selected)
        summary["released_usb_profile"] = True
        playback.owner.save(root / "binding.json", binding)
        cards = [int(q.name[4:]) for q in Path("/sys/class/sound").glob("card[0-9]*")
                 if selected in (q / "device").resolve(strict=True).parents]
        summary["unique_selected_card"] = len(cards) == 1
        if len(cards) != 1:
            raise RuntimeError("selected Studio does not identify one ALSA card")
        card = cards[0]
        endpoints = [playback.endpoint(objects, card, link) for link in (1, 2, 3, 4)]
        playback.owner.save(root / "endpoints.json", endpoints)
        summary["all_four_endpoints_verified"] = True
        try:
            playback.require_rate_isolation(objects, endpoints[0]["device"]["id"])
            summary["isolated_graph"] = True
        except RuntimeError as error:
            summary["isolated_graph"] = False
            summary["errors"].append(str(error))
        alsa = Path("/proc/asound") / ("card" + str(card))
        for direction, name in (("p", "playback"), ("c", "capture")):
            status = (alsa / ("pcm0" + direction) / "sub0/status").read_bytes()
            playback.owner.save_bytes(root / ("alsa-" + name + "-status.txt"), status)
            summary["alsa_" + name + "_closed"] = status.strip() == b"closed"
        device = Path("/dev/snd") / ("pcmC%dD0p" % card)
        summary["alsa_access"] = os.access(device, os.R_OK | os.W_OK)
        summary["cached_descriptor_sha256"] = binding["descriptor_sha256"]
        summary["baseline_stable"] = (playback.owner.snapshot() == before
                                       and playback.owner.connected_binding(selected) == binding)
        if not summary["baseline_stable"]:
            raise RuntimeError("desktop or USB baseline changed during read-only preflight")
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        summary["errors"].append(str(error))
    summary["ready_for_separate_native_silence_test"] = ready(summary) and summary.get("baseline_stable", False)
    playback.owner.save(root / "preflight.json", summary)
    print(json.dumps(summary))
    # An incomplete preflight is evidence, never permission to bypass its guards.
    return 0 if summary["ready_for_separate_native_silence_test"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
