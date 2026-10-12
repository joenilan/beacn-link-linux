#!/usr/bin/env python3
"""Read one explicitly named Linux USB sysfs cache; no USB requests or writes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

MAX_BYTES = 256 * 1024


class ProbeError(ValueError):
    pass


def cached(path, limit):
    with path.open("rb") as handle:
        value = handle.read(limit + 1)
    if len(value) > limit:
        raise ProbeError("cached attribute exceeds limit")
    return value


def scalar(path, base, maximum):
    text = cached(path, 64).decode("ascii").strip()
    pattern = r"[0-9]+" if base == 10 else r"[0-9a-fA-F]+"
    if not re.fullmatch(pattern, text):
        raise ProbeError("invalid numeric cached attribute")
    value = int(text, base)
    if not 0 <= value <= maximum:
        raise ProbeError("numeric cached attribute outside range")
    return value


def sample(root):
    identity = root.stat()
    attributes = {name: scalar(root / name, base, maximum) for name, base, maximum in (
        ("idVendor", 16, 65535), ("idProduct", 16, 65535),
        ("bcdDevice", 16, 65535), ("bConfigurationValue", 10, 255))}
    if (attributes["idVendor"], attributes["idProduct"]) != (0x33AE, 0x4003):
        raise ProbeError("named device is not the static VID/PID target")
    interfaces = []
    prefix = root.name + ":"
    for child in sorted(root.iterdir()):
        if not child.name.startswith(prefix):
            continue
        suffix = child.name[len(prefix):]
        if not re.fullmatch(r"[0-9]+\.[0-9]+", suffix) or not child.is_dir() or child.is_symlink():
            raise ProbeError("unexpected cached interface directory")
        configuration, number = map(int, suffix.split("."))
        if configuration != attributes["bConfigurationValue"] or not 0 <= number <= 255:
            raise ProbeError("cached interface does not match active configuration")
        observed_number = scalar(child / "bInterfaceNumber", 16, 255)
        if number != observed_number:
            raise ProbeError("cached interface number mismatch")
        driver = child / "driver"
        if driver.is_symlink():
            driver_name = Path(os.readlink(driver)).name
            if not re.fullmatch(r"[A-Za-z0-9_.-]{1,64}", driver_name):
                raise ProbeError("invalid cached driver name")
        elif driver.exists():
            raise ProbeError("unexpected cached driver entry")
        else:
            driver_name = None
        interfaces.append({"number": number,
                           "alternate": scalar(child / "bAlternateSetting", 10, 255),
                           "class": scalar(child / "bInterfaceClass", 16, 255),
                           "driver": driver_name})
    if not interfaces or not any(i["number"] == 3 for i in interfaces):
        raise ProbeError("active interface 3 missing from cache")
    raw = cached(root / "descriptors", MAX_BYTES)
    if len(raw) < 18 or raw[:2] != bytes([18, 1]):
        raise ProbeError("invalid cached device descriptor")
    for offset, name in ((8, "idVendor"), (10, "idProduct"), (12, "bcdDevice")):
        if int.from_bytes(raw[offset:offset + 2], "little") != attributes[name]:
            raise ProbeError("cached descriptor and attributes disagree")
    return (identity.st_dev, identity.st_ino), attributes, interfaces, raw


def probe(directory, codec, fixture=False):
    root = Path(directory).resolve(strict=True)
    if not fixture:
        if sys.platform != "linux" or not root.is_relative_to(Path("/sys/devices")):
            raise ProbeError("live mode requires an explicitly named Linux sysfs USB device")
        if (root / "subsystem").resolve(strict=True) != Path("/sys/bus/usb"):
            raise ProbeError("named directory is not a USB sysfs device")
    before = sample(root)
    result = subprocess.run([str(Path(codec).resolve(strict=True)), "inspect-usb-descriptors",
                             str(before[1]["bConfigurationValue"])], input=before[3],
                            capture_output=True, timeout=5, check=False)
    after = sample(root)
    if after != before:
        raise ProbeError("cached device/interface state changed during observation")
    if len(result.stdout) + len(result.stderr) > 2 * 1024 * 1024:
        raise ProbeError("descriptor validator output exceeds limit")
    if result.returncode:
        raise ProbeError("offline descriptor validator refused the candidate")
    selected = next(i for i in before[2] if i["number"] == 3)
    gate = "candidate_only_no_authority"
    if selected["driver"] is not None:
        gate = "blocked_kernel_driver"
    elif selected["alternate"] != 1:
        gate = "requires_authorized_alternate_change"
    return {"mode": "synthetic_fixture" if fixture else "read_only_sysfs_cache",
            "attributes": before[1], "interfaces": before[2],
            "descriptor_sha256": hashlib.sha256(before[3]).hexdigest(),
            "cache_comparison_equal": True, "transport_gate": gate,
            "offline_report": result.stdout.decode("utf-8", errors="strict")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--sysfs-device")
    source.add_argument("--fixture-directory", help="synthetic directory only; cannot qualify hardware")
    parser.add_argument("--codec", required=True, help="trusted built link-protocol executable")
    args = parser.parse_args()
    try:
        report = probe(args.sysfs_device or args.fixture_directory, args.codec,
                       fixture=args.fixture_directory is not None)
        print(json.dumps(report, indent=2))
    except (ProbeError, OSError, UnicodeError, subprocess.TimeoutExpired):
        print("descriptor probe refused; no USB claim, transfer or routing was requested", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
