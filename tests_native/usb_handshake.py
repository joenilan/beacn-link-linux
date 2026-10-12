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


def save(path, value):
    with path.open("x", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stage", choices=("prepare", "run", "compare"))
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
    if args.stage == "prepare":
        if targets():
            parser.error("prepare while Studio is still on Windows")
        root.mkdir(mode=0o700, parents=False, exist_ok=False)
        save(root / "baseline.json", {"time": time.time(), "binary_sha256": digest, "snapshot": snapshot()})
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
        expected = Path(args.expected_cache).resolve(strict=True)
        with expected.open("rb") as handle:
            raw = handle.read(256 * 1024 + 1)
        if len(raw) > 256 * 1024:
            parser.error("expected cache too large")
        pinned = root / "expected-cache.bin"
        with pinned.open("xb") as handle:
            handle.write(raw)
        attached = snapshot()
        save(root / "run-started.json", {"binary_sha256": digest,
                                        "expected_cache_sha256": hashlib.sha256(raw).hexdigest(), "snapshot": attached})
        env = os.environ.copy()
        env["LIBUSB_DEBUG"] = "0"
        result = {"timed_out": False, "binary_sha256": digest}
        p = subprocess.Popen([str(binary), "--authorize-handshake", str(selected), str(pinned)],
                             env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        result["private_child_pid"] = p.pid
        try:
            stdout, stderr = p.communicate(timeout=12)
        except subprocess.TimeoutExpired:
            result["timed_out"] = True
            p.kill()  # Only this owned child, never services/another process.
            try:
                stdout, stderr = p.communicate(timeout=2)
            except subprocess.TimeoutExpired as error:
                # Kernel-blocked exit is possible. Do not wait indefinitely or
                # report successful cleanup. Preserve the owned PID for review.
                stdout, stderr = error.output or b"", error.stderr or b""
        result["child_exited"] = p.poll() is not None
        result.update(exit=p.returncode, stdout=stdout.decode("utf-8", "replace"), stderr=stderr.decode("utf-8", "replace"))
        if result["child_exited"]:
            result["after_child"] = snapshot()
        else:
            p.stdout.close()
            p.stderr.close()
        save(root / "handshake.json", result)
        print(json.dumps({"timed_out": result["timed_out"], "exit": result.get("exit"),
                          "child_exited": result["child_exited"],
                          "next": "owner returns Studio to Windows, then compare"}))
        return 0 if result.get("exit") == 0 and not result["timed_out"] else 2
    journal = root / "handshake.json"
    if journal.exists() and not json.loads(journal.read_text())["child_exited"]:
        parser.error("owned child exit was unconfirmed; manual review required before comparison")
    if targets():
        parser.error("compare only after owner return to Windows")
    after = snapshot()
    comparison = {"guest_target_absent": True,
                  "desktop_exact_equal": after["desktop"] == before["snapshot"]["desktop"],
                  "state_files_exact_equal": after["state_files"] == before["snapshot"]["state_files"],
                  "after": after}
    save(root / "comparison.json", comparison)
    print(json.dumps({k: v for k, v in comparison.items() if k != "after"}))
    # Preserve differences. Never automatically overwrite live persistent state.
    return 0 if comparison["desktop_exact_equal"] and comparison["state_files_exact_equal"] else 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.TimeoutExpired, ValueError) as error:
        print("handshake test stopped: " + type(error).__name__, file=sys.stderr)
        raise SystemExit(2)
