#!/usr/bin/env python3
"""One read-only PipeWire dump, explicit wire bindings, optional pure planner call."""
import argparse
import json
import os
import re
import selectors
import shutil
import stat
import subprocess
import sys
import time

MAX_DUMP_BYTES = 8 * 1024 * 1024
MAX_OBJECTS = 16384
MAX_STREAMS = 4096
MAX_APPLICATIONS = 256
NODE = "PipeWire:Interface:Node"
CLIENT = "PipeWire:Interface:Client"


class SnapshotError(ValueError):
    pass


class Unavailable(Exception):
    pass


def uint(value, maximum=(1 << 64) - 1):
    if type(value) is int:
        number = value
    elif isinstance(value, str) and len(value) <= 20 and re.fullmatch(r"[0-9]+", value):
        number = int(value)
    else:
        raise SnapshotError("identity metadata is missing or not an unsigned integer")
    if not 0 <= number <= maximum:
        raise SnapshotError("identity metadata is out of range")
    return number


def properties(obj):
    info = obj.get("info")
    if not isinstance(info, dict):
        return {}
    props = info.get("props")
    return props if isinstance(props, dict) else {}


def no_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise SnapshotError("duplicate JSON key")
        result[key] = value
    return result


def decode_dump(raw):
    if len(raw) > MAX_DUMP_BYTES:
        raise SnapshotError("dump exceeds 8 MiB limit")
    try:
        objects = json.loads(raw.decode("utf-8"), object_pairs_hook=no_duplicate_keys,
                             parse_constant=lambda _: (_ for _ in ()).throw(SnapshotError("non-finite JSON number")))
    except (UnicodeError, json.JSONDecodeError, RecursionError, ValueError) as exc:
        raise SnapshotError("invalid UTF-8 PipeWire JSON") from exc
    if not isinstance(objects, list) or len(objects) > MAX_OBJECTS:
        raise SnapshotError("expected a bounded pw-dump JSON array")
    seen = set()
    for obj in objects:
        if not isinstance(obj, dict):
            raise SnapshotError("PipeWire objects must be JSON objects")
        ident = uint(obj.get("id"), (1 << 32) - 1)
        if ident in seen:
            raise SnapshotError("duplicate PipeWire global ID")
        seen.add(ident)
    return objects


def observe(objects, snapshot_id):
    uint(snapshot_id)
    clients = {uint(o["id"]): o for o in objects if o.get("type") == CLIENT}
    client_serials = {}
    for client_id, client in clients.items():
        value = properties(client).get("object.serial")
        if value is not None:
            serial = uint(value)
            if serial in client_serials:
                raise SnapshotError("duplicate client object serial")
            client_serials[serial] = client_id
    apps = {}
    unresolved = []
    serials = set()
    playback = 0
    for node in objects:
        props = properties(node)
        if node.get("type") != NODE or props.get("media.class") != "Stream/Output/Audio":
            continue
        playback += 1
        if playback > MAX_STREAMS:
            raise SnapshotError("more than 4096 playback streams")
        node_id = uint(node["id"])
        try:
            serial = uint(props.get("object.serial"))
            client_id = uint(props.get("client.id"), (1 << 32) - 1)
            client = clients.get(client_id)
            if client is None:
                raise SnapshotError("stream's client is absent from the same dump")
            cp = properties(client)
            client_serial = uint(cp.get("object.serial"))
            pid_source = "node" if "application.process.id" in props else "client"
            pid = uint(props.get("application.process.id", cp.get("application.process.id")), (1 << 32) - 1)
            if pid == 0:
                raise SnapshotError("application PID is zero")
        except SnapshotError as exc:
            unresolved.append({"node_id": node_id, "reason": str(exc)})
            continue
        if serial in serials:
            raise SnapshotError("duplicate playback object serial")
        serials.add(serial)
        app_id = f"pw-client:{client_serial}:pid:{pid}"
        if app_id not in apps:
            if len(apps) >= MAX_APPLICATIONS:
                raise SnapshotError("more than 256 application identities")
            apps[app_id] = {"application_id": app_id, "client_id": client_id,
                            "client_serial": client_serial, "process_pid": pid,
                            "application_names": [], "application_ids": [],
                            "process_binaries": [], "streams": []}
        app = apps[app_id]
        # A reused serial in different clients must not silently merge identities.
        if app["client_id"] != client_id:
            raise SnapshotError("client serial does not uniquely identify a client")
        for key, label in [("application.name", "application_names"),
                           ("application.id", "application_ids"),
                           ("application.process.binary", "process_binaries")]:
            value = props.get(key, cp.get(key))
            if isinstance(value, str) and value:
                display = value[:512]
                if display not in app[label]:
                    app[label].append(display)
        info = node.get("info")
        state = info.get("state") if isinstance(info, dict) else None
        state = state[:64] if isinstance(state, str) else None
        planner_state = "live" if state in ("running", "idle", "suspended") else "stale"
        app["streams"].append({"node_id": node_id, "object_serial": serial,
                               "stream_id": f"pw-node:{serial}", "node_state": state,
                               "pid_source": pid_source, "planner_state": planner_state})
    for app in apps.values():
        app["streams"].sort(key=lambda s: s["stream_id"])
        for label in ("application_names", "application_ids", "process_binaries"):
            app[label].sort()
    return {"status": "available", "snapshot_id": snapshot_id,
            "playback_stream_count": playback,
            "applications": [apps[k] for k in sorted(apps)],
            "unresolved_streams": sorted(unresolved, key=lambda s: s["node_id"]),
            "planner_ready": not unresolved}


def binding_map(values):
    result = {}
    for value in values:
        app_id, sep, name = value.partition("=")
        if not sep or not re.fullmatch(r"pw-client:[0-9]+:pid:[0-9]+", app_id):
            raise SnapshotError("binding requires an observed application-id=wire-name")
        if app_id in result:
            raise SnapshotError("duplicate application binding")
        if not 1 <= len(name) <= 255 or any(not 0x20 <= ord(c) <= 0x7e for c in name):
            raise SnapshotError("wire names must be nonempty printable ASCII, at most 255 bytes")
        result[app_id] = name
    return result


def project(snapshot, bindings):
    if not snapshot["planner_ready"]:
        raise SnapshotError("unresolved playback identities block planner projection")
    observed = {a["application_id"]: a for a in snapshot["applications"]}
    if any(k not in observed for k in bindings):
        raise SnapshotError("binding refers to an identity absent from this snapshot")
    applications = []
    for app_id in sorted(bindings):
        app = observed[app_id]
        applications.append({"application_id": app_id, "advertised_name": bindings[app_id],
                             "streams": [{"stream_id": s["stream_id"], "state": s["planner_state"]}
                                         for s in app["streams"]]})
    return {"snapshot_id": snapshot["snapshot_id"], "applications": applications}


def to_tsv(inventory):
    return "".join(f'{a["application_id"]}\t{a["advertised_name"]}\t{s["stream_id"]}\t{s["state"]}\n'
                   for a in inventory["applications"] for s in a["streams"])


def collect_live():
    if sys.platform != "linux":
        raise Unavailable("live discovery requires Linux")
    runtime = os.environ.get("XDG_RUNTIME_DIR", "")
    if not runtime or not os.path.isabs(runtime):
        raise Unavailable("user audio runtime directory is unavailable")
    try:
        st = os.stat(os.path.join(runtime, "pipewire-0"))
        if not stat.S_ISSOCK(st.st_mode) or st.st_uid != os.getuid():
            raise Unavailable("default user PipeWire socket is unavailable")
    except OSError as exc:
        raise Unavailable("default user PipeWire socket is unavailable") from exc
    exe = shutil.which("pw-dump")
    if not exe:
        raise Unavailable("pw-dump is missing; Ubuntu package: pipewire-bin")
    # No monitor, parameter write, stream creation or routing operation.
    child_env = os.environ.copy()
    child_env["PIPEWIRE_RUNTIME_DIR"] = runtime
    child_env.pop("PIPEWIRE_REMOTE", None)
    try:
        proc = subprocess.Popen([exe, "-N", "-r", "pipewire-0"], stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, env=child_env)
    except OSError as exc:
        raise Unavailable("pw-dump could not be launched") from exc
    output, errors = bytearray(), bytearray()
    selector = selectors.DefaultSelector()
    selector.register(proc.stdout, selectors.EVENT_READ, output)
    selector.register(proc.stderr, selectors.EVENT_READ, errors)
    deadline = time.monotonic() + 5
    try:
        while selector.get_map():
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise Unavailable("pw-dump exceeded the five-second query budget")
            for key, _ in selector.select(remaining):
                chunk = os.read(key.fileobj.fileno(), 65536)
                if not chunk:
                    selector.unregister(key.fileobj)
                    continue
                key.data.extend(chunk)
                if len(output) > MAX_DUMP_BYTES or len(errors) > 16384:
                    raise Unavailable("pw-dump exceeded its output budget")
        rc = proc.wait(timeout=max(0.01, deadline - time.monotonic()))
        if rc:
            raise Unavailable(f"pw-dump failed with exit {rc}")
        return bytes(output)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise Unavailable("pw-dump query failed") from exc
    finally:
        selector.close()
        if proc.poll() is None:
            proc.kill()  # Only this owned, read-only query, never an audio service.
        proc.wait()
        proc.stdout.close()
        proc.stderr.close()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dump", help="read supplied pw-dump JSON instead of accessing the session")
    parser.add_argument("--snapshot-id", type=int, default=None)
    parser.add_argument("--bind", action="append", default=[], metavar="APPLICATION-ID=WIRE-NAME")
    parser.add_argument("--plan", metavar="DATA-FRAME-HEX", help="run only the pure planner on this supplied frame")
    parser.add_argument("--planner-bin", help="explicit path to the built link-protocol executable")
    args = parser.parse_args(argv)
    try:
        bindings = binding_map(args.bind)
        snapshot_id = args.snapshot_id if args.snapshot_id is not None else time.time_ns()
        uint(snapshot_id)
        if args.dump:
            with open(args.dump, "rb") as source:
                raw = source.read(MAX_DUMP_BYTES + 1)
        else:
            raw = collect_live()
        snapshot = observe(decode_dump(raw), snapshot_id)
        inventory = project(snapshot, bindings) if snapshot["planner_ready"] else None
        snapshot["planner_inventory"] = inventory
        snapshot["unbound_application_ids"] = [a["application_id"] for a in snapshot["applications"]
                                                if a["application_id"] not in bindings]
        if args.plan:
            if not args.planner_bin:
                raise SnapshotError("--plan requires an explicit --planner-bin")
            if inventory is None:
                raise SnapshotError("unresolved playback identities block planning")
            binary = os.path.abspath(args.planner_bin)
            result = subprocess.run([binary, "plan-snapshot", str(snapshot_id), args.plan],
                                    input=to_tsv(inventory).encode("ascii"), capture_output=True, timeout=5)
            snapshot["plan_exit_code"] = result.returncode
            snapshot["plan_output"] = result.stdout.decode("utf-8", "replace")
            snapshot["plan_error"] = result.stderr.decode("utf-8", "replace")
            print(json.dumps(snapshot, indent=2, ensure_ascii=True))
            return result.returncode
        print(json.dumps(snapshot, indent=2, ensure_ascii=True))
        return 0
    except Unavailable as exc:
        print(json.dumps({"status": "unavailable", "reason": str(exc), "planner_inventory": None}))
        return 3
    except (SnapshotError, OSError, subprocess.TimeoutExpired) as exc:
        print(json.dumps({"status": "invalid-input", "reason": str(exc), "planner_inventory": None}))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
