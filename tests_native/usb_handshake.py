#!/usr/bin/env python3
"""Explicitly authorized one-shot handshake; prepare/compare are metadata only."""
import argparse
import base64
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import time

sys.dont_write_bytecode = True
REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "tools"))
import pipewire_inventory as inventory
import usb_descriptor_probe as usb_cache
spec = importlib.util.spec_from_file_location("private_core", REPO / "tests_native/private_core.py")
core = importlib.util.module_from_spec(spec)
spec.loader.exec_module(core)


def snapshot():
    env = os.environ.copy()
    env["XDG_RUNTIME_DIR"] = "/run/user/" + str(os.getuid())
    for key in ("PIPEWIRE_REMOTE", "PIPEWIRE_RUNTIME_DIR", "PIPEWIRE_CONFIG_DIR"):
        env.pop(key, None)
    p = subprocess.run(["pw-dump", "-N", "-r", "pipewire-0"], env=env, capture_output=True, timeout=5)
    if p.returncode or len(p.stdout) > inventory.MAX_DUMP_BYTES or len(p.stderr) > 16384:
        raise RuntimeError("bounded desktop metadata query failed")
    state = Path(env.get("XDG_STATE_HOME", str(Path.home() / ".local/state"))) / "wireplumber"
    files = {}
    for name in ("default-nodes", "stream-properties", "default-routes", "default-profile"):
        path = state / name
        if path.exists():
            with path.open("rb") as handle:
                raw = handle.read(1024 * 1024 + 1)
            if len(raw) > 1024 * 1024:
                raise RuntimeError("state backup exceeds limit")
            files[name] = {"sha256": hashlib.sha256(raw).hexdigest(),
                           "bytes_base64": base64.b64encode(raw).decode(), "mode": path.stat().st_mode & 0o777}
        else:
            files[name] = None
    return {"desktop": core.filtered(inventory.decode_dump(p.stdout)), "state_files": files}


def targets():
    return [p for p in Path("/sys/bus/usb/devices").iterdir() if (p / "idVendor").is_file()
            and (p / "idProduct").is_file() and (p / "idVendor").read_text().strip() == "33ae"
            and (p / "idProduct").read_text().strip() == "4003"]


def save_bytes(path, value):
    """Exclusive durable evidence, including its directory entry, before device I/O."""
    with path.open("xb") as handle:
        handle.write(value)
        handle.flush()
        os.fsync(handle.fileno())
    if os.name == "posix":
        # Evidence roots are newly created under an existing private parent.
        # Persist both the file entry and that new root's entry before playback.
        for parent in dict.fromkeys((path.parent, path.parent.parent)):
            directory = os.open(parent, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(directory)
            finally:
                os.close(directory)


def save(path, value):
    # Serialize first so invalid evidence cannot leave a newly created empty file.
    save_bytes(path, json.dumps(value, indent=2).encode("utf-8"))


def connected_binding(directory):
    """Read-only comparison stamp, never authority for a claim or deletion."""
    selected = Path(directory).resolve(strict=True)
    found = targets()
    if len(found) != 1 or found[0].resolve() != selected:
        raise RuntimeError("connected target absent or ambiguous")
    first = usb_cache.sample(selected)
    if usb_cache.sample(selected) != first:
        raise RuntimeError("connected target changed during observation")
    instance, attributes, interfaces, raw = first
    if attributes["bConfigurationValue"] != 1 or [(i["number"], i["alternate"]) for i in interfaces] != [(0, 0), (1, 0), (2, 0), (3, 0)] or interfaces[-1]["driver"] is not None:
        raise RuntimeError("connected target is not in its released baseline profile")
    return {"sysfs_device": str(selected), "instance": list(instance), "attributes": attributes,
            "interfaces": interfaces, "descriptor_sha256": hashlib.sha256(raw).hexdigest()}


def execute_child(command, timeout=12):
    """Bounded owned-child supervision shared by the explicit USB test runners."""
    env = os.environ.copy()
    env["LIBUSB_DEBUG"] = "0"
    result = {"timed_out": False}
    p = subprocess.Popen(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    result["private_child_pid"] = p.pid
    try:
        stdout, stderr = p.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        result["timed_out"] = True
        p.kill()
        try:
            stdout, stderr = p.communicate(timeout=2)
        except subprocess.TimeoutExpired as error:
            stdout, stderr = error.output or b"", error.stderr or b""
    result["child_exited"] = p.poll() is not None
    result.update(exit=p.returncode, stdout=stdout.decode("utf-8", "replace"), stderr=stderr.decode("utf-8", "replace"))
    if result["child_exited"]:
        result["after_child"] = snapshot()
    else:
        p.stdout.close()
        p.stderr.close()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stage", choices=("prepare", "prepare-connected", "run", "compare", "compare-connected"))
    parser.add_argument("--evidence-dir", required=True)
    parser.add_argument("--executor", required=True)
    parser.add_argument("--authorize-handshake", action="store_true")
    parser.add_argument("--sysfs-device")
    parser.add_argument("--expected-cache")
    args = parser.parse_args()
    root = Path(args.evidence_dir).resolve()
    if sys.platform != "linux" or root == REPO or REPO in root.parents:
        parser.error("requires Linux and private evidence outside the repository")
    binary = Path(args.executor).resolve(strict=True)
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    if args.stage in ("prepare", "prepare-connected"):
        connected = args.stage == "prepare-connected"
        if connected and not args.sysfs_device:
            parser.error("prepare-connected requires an exact selected sysfs device")
        binding = connected_binding(args.sysfs_device) if connected else None
        if not connected and targets():
            parser.error("prepare while Studio is still on Windows")
        root.mkdir(mode=0o700, parents=False, exist_ok=False)
        observed = snapshot()
        if connected and connected_binding(args.sysfs_device) != binding:
            parser.error("connected target changed across baseline snapshot")
        save(root / "baseline.json", {"time": time.time(), "binary_sha256": digest, "snapshot": observed,
                                      "scope": "connected" if connected else "pre-handoff", "binding": binding})
        print("baseline and complete tracked state-file backups saved; no device I/O")
        return 0
    before = json.loads((root / "baseline.json").read_text())
    if digest != before["binary_sha256"]:
        parser.error("executor differs from baseline")
    if args.stage == "run":
        if not args.authorize_handshake or not args.sysfs_device or not args.expected_cache:
            parser.error("run requires specific owner authorization, exact sysfs device and expected cache")
        if (root / "handshake.json").exists() or (root / "run-started.json").exists():
            parser.error("one-shot evidence already used; no retry")
        if time.time() - before["time"] > 600:
            parser.error("baseline older than ten minutes; do not run")
        selected = Path(args.sysfs_device).resolve(strict=True)
        found = targets()
        if len(found) != 1 or found[0].resolve() != selected:
            parser.error("exact selected target absent or ambiguous")
        if before.get("scope") == "connected" and connected_binding(selected) != before["binding"]:
            parser.error("connected target differs from baseline")
        expected = Path(args.expected_cache).resolve(strict=True)
        with expected.open("rb") as handle:
            raw = handle.read(256 * 1024 + 1)
        if len(raw) > 256 * 1024:
            parser.error("expected cache too large")
        pinned = root / "expected-cache.bin"
        save_bytes(pinned, raw)
        attached = snapshot()
        save(root / "run-started.json", {"binary_sha256": digest,
                                        "expected_cache_sha256": hashlib.sha256(raw).hexdigest(), "snapshot": attached})
        result = execute_child([str(binary), "--authorize-handshake", str(selected), str(pinned)])
        result["binary_sha256"] = digest
        save(root / "handshake.json", result)
        print(json.dumps({"timed_out": result["timed_out"], "exit": result.get("exit"),
                          "child_exited": result["child_exited"],
                          "next": "owner returns Studio to Windows, then compare"}))
        return 0 if result.get("exit") == 0 and not result["timed_out"] else 2
    journal = root / "handshake.json"
    if (root / "run-started.json").exists() and not journal.exists():
        parser.error("attempt completion unconfirmed; manual review required before comparison")
    if journal.exists() and not json.loads(journal.read_text())["child_exited"]:
        parser.error("owned child exit was unconfirmed; manual review required before comparison")
    connected = args.stage == "compare-connected"
    if connected and before.get("scope") != "connected":
        parser.error("compare-connected requires a connected-state baseline")
    binding = connected_binding(before["binding"]["sysfs_device"]) if connected else None
    if connected and binding != before["binding"]:
        parser.error("connected target differs from baseline")
    if not connected and targets():
        parser.error("compare only after owner return to Windows")
    after = snapshot()
    if connected and connected_binding(before["binding"]["sysfs_device"]) != binding:
        parser.error("connected target changed across comparison snapshot")
    comparison = {"guest_target_absent": not connected,
                  "baseline_scope": before.get("scope", "pre-handoff"),
                  "connected_binding_equal": True if connected else None,
                  "desktop_exact_equal": after["desktop"] == before["snapshot"]["desktop"],
                  "state_files_exact_equal": after["state_files"] == before["snapshot"]["state_files"],
                  "after": after}
    save(root / ("connected-comparison.json" if connected else "comparison.json"), comparison)
    print(json.dumps({k: v for k, v in comparison.items() if k != "after"}))
    # Preserve differences. Never automatically overwrite live persistent state.
    return 0 if comparison["desktop_exact_equal"] and comparison["state_files_exact_equal"] else 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.TimeoutExpired, ValueError) as error:
        print("handshake test stopped: " + type(error).__name__, file=sys.stderr)
        raise SystemExit(2)
