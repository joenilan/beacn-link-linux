#!/usr/bin/env python3
"""Collect only an existing LinX window capture over its opt-in local socket."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
from hashlib import sha256
from io import BytesIO
import json
import os
from pathlib import Path
import socket
import stat
import struct
import sys

try:
    from PIL import Image
except ImportError:
    raise SystemExit("Screenshot helpers require Pillow (python3-pil on Debian 13).")


ROOT = Path(__file__).resolve().parents[1]
RAW_DEFAULT = ROOT / "local" / "linx-console-raw.png"
MANIFEST_DEFAULT = ROOT / "local" / "linx-console-capture.json"
FRAME_DEFAULT = ROOT / "assets" / "screenshots" / "linx-console.png"
MAX_PPM = 8 * 1024 * 1024
MAX_REPLY = 2 * 1024 * 1024
UID = os.geteuid() if hasattr(os, "geteuid") else -1


def canonical_output(value: str) -> Path:
    path = Path(os.path.abspath(value))
    if path.parent.resolve(strict=True) != path.parent:
        raise ValueError(f"output parent must be canonical and contain no symlinks: {path.parent}")
    if os.path.lexists(path) and stat.S_ISLNK(path.lstat().st_mode):
        raise ValueError(f"refusing symlink output: {path}")
    return path


def check_socket(path: Path) -> None:
    if not path.is_absolute() or path.parent.resolve(strict=True) != path.parent:
        raise ValueError("socket path must be absolute and have a canonical parent")
    parent = path.parent.stat()
    if not stat.S_ISDIR(parent.st_mode) or parent.st_uid != UID or parent.st_mode & 0o077:
        raise ValueError("socket parent must be a private directory owned by this user")
    info = path.lstat()
    if not stat.S_ISSOCK(info.st_mode) or info.st_uid != UID or stat.S_IMODE(info.st_mode) != 0o600:
        raise ValueError("socket must be an owned Unix socket with mode 0600")


def receive_line(sock: socket.socket, pending: bytearray) -> bytes:
    while b"\n" not in pending:
        part = sock.recv(4096)
        if not part:
            raise ValueError("LinX closed the native-control connection")
        pending.extend(part)
        if len(pending) > MAX_REPLY + 1:
            raise ValueError("native-control response exceeds 2 MiB")
    end = pending.index(b"\n")
    if end > MAX_REPLY:
        raise ValueError("native-control response exceeds 2 MiB")
    line = bytes(pending[:end])
    del pending[: end + 1]
    return line


def request(sock: socket.socket, pending: bytearray, ident: str, op: str) -> dict:
    frame = json.dumps({"id": ident, "op": op}, separators=(",", ":")).encode() + b"\n"
    sock.sendall(frame)
    response = json.loads(receive_line(sock, pending))
    if not isinstance(response, dict) or response.get("id") != ident or "error" in response:
        raise ValueError(f"LinX refused {op}: {response}")
    result = response.get("result")
    if not isinstance(result, dict) or result.get("ok") is not True or not isinstance(result.get("value"), dict):
        raise ValueError(f"unexpected {op} response: {response}")
    return result["value"]


def executable_sha256(pid: int) -> str:
    digest = sha256()
    with open(f"/proc/{pid}/exe", "rb") as executable:
        while chunk := executable.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def read_capture(path: Path) -> tuple[bytes, tuple[int, int, int]]:
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != UID or stat.S_IMODE(info.st_mode) != 0o600:
        raise ValueError("capture is not an owned regular file with mode 0600")
    if info.st_nlink != 1 or not 0 < info.st_size < MAX_PPM:
        raise ValueError("capture has an unexpected link count or size (must be below 8 MiB)")
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        opened = os.fstat(fd)
        if (opened.st_dev, opened.st_ino) != (info.st_dev, info.st_ino):
            raise ValueError("capture changed while opening")
        with os.fdopen(fd, "rb", closefd=False) as source:
            content = source.read(MAX_PPM)
        after = os.fstat(fd)
        if (after.st_dev, after.st_ino, after.st_size) != (info.st_dev, info.st_ino, len(content)):
            raise ValueError("capture changed while reading")
        return content, (info.st_dev, info.st_ino, info.st_uid)
    finally:
        os.close(fd)


def png_bytes(ppm: bytes) -> tuple[bytes, int, int]:
    with Image.open(BytesIO(ppm)) as source:
        if source.format != "PPM" or source.width > 1600 or source.height > 1200:
            raise ValueError("native capture is not a supported LinX PPM frame")
        source.load()
        image = source.convert("RGB")
    output = BytesIO()
    image.save(output, format="PNG", optimize=True)
    return output.getvalue(), image.width, image.height


def write_output(path: Path, data: bytes, mode: int) -> None:
    existed = os.path.lexists(path)
    prior = path.lstat() if existed else None
    if prior and (not stat.S_ISREG(prior.st_mode) or prior.st_uid != UID or prior.st_nlink != 1):
        raise ValueError(f"refusing to replace non-private output: {path}")
    flags = os.O_WRONLY | os.O_CREAT | os.O_NOFOLLOW | os.O_CLOEXEC
    if not existed:
        flags |= os.O_EXCL
    fd = os.open(path, flags, mode)
    try:
        opened = os.fstat(fd)
        if not stat.S_ISREG(opened.st_mode) or opened.st_uid != UID or opened.st_nlink != 1:
            raise ValueError(f"output identity is unsafe: {path}")
        if prior and (opened.st_dev, opened.st_ino) != (prior.st_dev, prior.st_ino):
            raise ValueError(f"output changed while opening: {path}")
        os.ftruncate(fd, 0)
        os.fchmod(fd, mode)
        with os.fdopen(fd, "wb", closefd=False) as destination:
            destination.write(data)
            destination.flush()
            os.fsync(fd)
    finally:
        os.close(fd)


def unlink_capture(path: Path, identity: tuple[int, int, int], expected: bytes) -> None:
    info = path.lstat()
    if (info.st_dev, info.st_ino, info.st_uid) != identity or not stat.S_ISREG(info.st_mode):
        raise ValueError("capture path changed; leaving it in place")
    current, current_identity = read_capture(path)
    if current_identity != identity or current != expected:
        raise ValueError("capture contents changed; leaving it in place")
    os.unlink(path)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--socket", required=True, help="absolute path to an existing LinX native-control socket")
    parser.add_argument("--output", default=str(RAW_DEFAULT), help="private PNG output path")
    parser.add_argument("--manifest", default=str(MANIFEST_DEFAULT), help="private JSON manifest path")
    parser.add_argument("--frame", action="store_true", help="also write the framed README screenshot")
    parser.add_argument("--frame-output", default=str(FRAME_DEFAULT), help="framed PNG output path")
    args = parser.parse_args()
    if not sys.platform.startswith("linux"):
        raise SystemExit("capture_screenshot.py requires Linux Unix sockets and SO_PEERCRED")

    socket_path = Path(args.socket)
    check_socket(socket_path)
    ppm_path = Path(str(socket_path) + ".ppm")
    if os.path.lexists(ppm_path):
        raise SystemExit(f"Capture already exists: {ppm_path}. Collect or move the previous capture before retrying.")
    output = canonical_output(args.output)
    manifest = canonical_output(args.manifest)
    frame_path = canonical_output(args.frame_output) if args.frame else None
    if output.suffix.lower() != ".png" or manifest.suffix.lower() != ".json" or (frame_path and frame_path.suffix.lower() != ".png"):
        raise ValueError("PNG outputs must end in .png and the private manifest in .json")
    paths = [socket_path, ppm_path, output, manifest] + ([frame_path] if frame_path else [])
    normalized = [os.path.normcase(str(path)) for path in paths]
    if len(set(normalized)) != len(normalized):
        raise ValueError("socket, capture, PNG, manifest and frame paths must all be different")

    pending = bytearray()
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(7)
        client.connect(str(socket_path))
        peer_pid, peer_uid, _peer_gid = struct.unpack(
            "3i", client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i"))
        )
        if peer_uid != UID or peer_pid <= 0:
            raise ValueError("native-control peer credentials do not match this user")
        state = request(client, pending, "linx-capture-state", "state")
        if state.get("pid") != peer_pid:
            raise ValueError("LinX state PID does not match the authenticated socket peer")
        binary_hash = executable_sha256(peer_pid)
        captured = request(client, pending, "linx-capture-frame", "screenshot")
        if captured.get("path") != str(ppm_path) or captured.get("format") != "ppm" or captured.get("own_window_only") is not True:
            raise ValueError("LinX returned an unexpected or non-window screenshot result")
        if isinstance(captured.get("bytes"), bool) or not isinstance(captured.get("bytes"), int):
            raise ValueError("LinX returned an invalid capture size")

    ppm, identity = read_capture(ppm_path)
    if len(ppm) != captured["bytes"]:
        raise ValueError("capture size does not match the authenticated response")
    png, width, height = png_bytes(ppm)
    capture_hash = sha256(ppm).hexdigest()
    write_output(output, png, 0o600)
    if frame_path:
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        import frame_screenshot

        with Image.open(BytesIO(png)) as source:
            framed = frame_screenshot.frame(source, frame_screenshot.DEFAULT_CANVAS, None)
        buffer = BytesIO()
        framed.save(buffer, format="PNG", optimize=True)
        write_output(frame_path, buffer.getvalue(), 0o644)

    applications = state.get("applications")
    if not isinstance(applications, list) or not isinstance(state.get("phase"), str):
        raise ValueError("LinX state response is missing phase or application data")
    assignments = sorted({app["assignment"] for app in applications
                          if isinstance(app, dict) and isinstance(app.get("assignment"), int)
                          and not isinstance(app["assignment"], bool) and 1 <= app["assignment"] <= 4})
    manifest_data = {
        "capturedAtUtc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "phase": state.get("phase"),
        "verifiedPairs": assignments,
        "binarySha256": binary_hash,
        "captureSha256": capture_hash,
        "pngSha256": sha256(png).hexdigest(),
        "dimensions": [width, height],
    }
    write_output(manifest, (json.dumps(manifest_data, indent=2) + "\n").encode(), 0o600)
    unlink_capture(ppm_path, identity, ppm)
    print(f"Saved {output}")
    if frame_path:
        print(f"Saved {frame_path}")
    print(f"Saved private manifest {manifest}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        raise SystemExit(f"capture refused: {error}") from error
