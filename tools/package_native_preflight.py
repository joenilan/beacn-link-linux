#!/usr/bin/env python3
"""Package an explicit source-only allowlist for a temporary Linux feasibility check."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parents[1]
FILES = (
    "tests_native/studio_native_preflight.py", "tests_native/studio_alsa_playback.py",
    "tests_native/studio_playback.py", "tests_native/usb_handshake.py",
    "tests_native/private_core.py", "tools/pipewire_inventory.py",
    "tools/pipewire_graph.py", "tools/usb_descriptor_probe.py",
)
README = """BEACN Linux native feasibility preflight

Purpose: establish hardware feasibility before committing to a Linux migration.
This source-only bundle is not the finished BEACN Link replacement.

Use an Ubuntu desktop live session (Try Ubuntu), not the installer.
Official release: https://releases.ubuntu.com/26.04.1/
Live-session guide: https://documentation.ubuntu.com/desktop/en/latest/tutorial/try-ubuntu-desktop/
No reboot, USB formatting or Windows disk changes are performed by this bundle.
Creating boot media and temporarily leaving Windows are separate owner actions.

Extract the bundle in the live user's home directory and open a terminal there.
Run the READ-ONLY step:

  python3 tests_native/studio_native_preflight.py --evidence-dir "$HOME/beacn-native-preflight-$(date -u +%Y%m%dT%H%M%SZ)"

This saves private desktop/settings backups, reads cached USB identity, verifies
actual four-pair endpoint mappings and detects hardware/VM context. It creates
no audio stream, sends no vendor command and changes no route/default/volume/mute.
It needs existing python3, pw-dump and systemd-detect-virt; installs nothing.
A nonzero exit or readiness=false is a report to review, not a reason to bypass
the device/isolation guards. Other audio devices can currently block readiness.
Readiness is not evidence that the audio clock or finished application works.

Preserve the evidence directory outside Git and before ending the live session;
live-home files are temporary. Do not mount or edit Windows disks for this check.
The included direct silence runner is a separate explicitly authorized test.
Do not start playback until the preflight and fresh baseline have been reviewed.
No Rust binary, proprietary material, private artifacts or access credentials
are included. MANIFEST.json records every bundled source file's SHA256.
"""


def build(output):
    contents = {name: (ROOT / name).read_bytes() for name in FILES}
    manifest = {"kind": "source_only_read_only_native_preflight",
                "files": {name: hashlib.sha256(value).hexdigest() for name, value in contents.items()}}
    with output.open("xb") as handle:
        with zipfile.ZipFile(handle, "w", compression=zipfile.ZIP_STORED) as archive:
            for name, value in contents.items():
                archive.writestr(name, value)
            archive.writestr("README.txt", README)
            archive.writestr("MANIFEST.json", json.dumps(manifest, indent=2))
        handle.flush()
        os.fsync(handle.fileno())
    return {"source_files": len(contents), "bytes": output.stat().st_size,
            "sha256": hashlib.sha256(output.read_bytes()).hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    output = Path(args.output).resolve()
    if output == ROOT or ROOT in output.parents:
        parser.error("save the portable bundle outside the repository")
    print(json.dumps(build(output)))


if __name__ == "__main__":
    main()
