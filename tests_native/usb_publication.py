#!/usr/bin/env python3
"""One Linux-owned synthetic application, one USB publication/poll; never routing."""
import argparse
import hashlib
import json
import os
import selectors
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import usb_handshake as base

WIRE_NAME = "BEACN Linux VM Test"
MAX_CYCLES = 40


def inventory_message(inventory):
    text = base.inventory.to_tsv(inventory).encode("ascii")
    if not 0 < len(text) <= 3000:
        raise RuntimeError("fresh inventory exceeds diagnostic IPC bound")
    return f'{inventory["snapshot_id"]}\t{len(text)}\n'.encode("ascii") + text


def execute_periodic(command, fresh, cycles, timeout=50):
    """Linux-only bounded IPC; no shell, retries, or background inventory writer."""
    child = subprocess.Popen(command, env={**os.environ, "LIBUSB_DEBUG": "0"},
                             stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    result = {"private_child_pid": child.pid, "child_exited": False, "timed_out": False, "exit": 2}
    output = {"stdout": bytearray(), "stderr": bytearray()}
    pending = bytearray()
    requests = 0
    deadline = time.monotonic() + timeout
    try:
        os.set_blocking(child.stdin.fileno(), False)
        with selectors.DefaultSelector() as selector:
            for name in output:
                pipe = getattr(child, name)
                os.set_blocking(pipe.fileno(), False)
                selector.register(pipe, selectors.EVENT_READ, name)
            while selector.get_map():
                if time.monotonic() >= deadline:
                    result["timed_out"] = True
                    raise RuntimeError("periodic child exceeded deadline")
                for key, _ in selector.select(timeout=min(0.1, max(0, deadline - time.monotonic()))):
                    data = os.read(key.fileobj.fileno(), 4096)
                    if not data:
                        selector.unregister(key.fileobj)
                        continue
                    output[key.data].extend(data)
                    if len(output[key.data]) > 65536:
                        raise RuntimeError("periodic child output exceeds bound")
                    if key.data != "stdout":
                        continue
                    pending.extend(data)
                    while b"\n" in pending:
                        line, _, remainder = pending.partition(b"\n")
                        pending = bytearray(remainder)
                        if line.startswith(b"INVENTORY"):
                            if line != f"INVENTORY {requests}".encode() or requests >= 2 * cycles + 2:
                                raise RuntimeError("inventory request sequence/budget refused")
                            message = inventory_message(fresh(requests))
                            if time.monotonic() >= deadline:
                                result["timed_out"] = True
                                raise RuntimeError("fresh inventory exceeded child deadline")
                            # <= PIPE_BUF; nonblocking write cannot hang on an
                            # unresponsive or departed child. Partial writes stop.
                            if os.write(child.stdin.fileno(), message) != len(message):
                                raise RuntimeError("partial inventory delivery")
                            requests += 1
                    if len(pending) > 4096:
                        raise RuntimeError("periodic child line exceeds bound")
            child.wait(timeout=max(0.01, deadline - time.monotonic()))
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        result["supervisor_error"] = type(error).__name__ + ": " + str(error)
        if isinstance(error, subprocess.TimeoutExpired):
            result["timed_out"] = True
    finally:
        if child.poll() is None:
            child.kill()
            try:
                child.wait(timeout=2)
            except subprocess.TimeoutExpired:
                pass
        result["child_exited"] = child.poll() is not None
        result["exit"] = child.returncode if result["child_exited"] else None
        result.update({name: data.decode("utf-8", errors="replace") for name, data in output.items()})
        result["inventory_requests"] = requests
        for pipe in (child.stdin, child.stdout, child.stderr):
            pipe.close()
        if result["child_exited"]:
            result["after_child"] = base.snapshot()
    return result


def owned_inventory(objects, pid, node_name, snapshot_id):
    snapshot = base.inventory.observe(objects, snapshot_id)
    matching = [a for a in snapshot["applications"] if a["process_pid"] == pid]
    if len(matching) != 1:
        raise RuntimeError("owned playback process does not have one application identity")
    app = matching[0]
    nodes = {o["id"]: o for o in objects if o.get("type") == base.inventory.NODE}
    if not app["streams"] or any(s["planner_state"] != "live" or
            base.inventory.properties(nodes[s["node_id"]]).get("node.name") != node_name for s in app["streams"]):
        raise RuntimeError("owned stream identity/state mismatch")
    if any(o.get("type") == "PipeWire:Interface:Link" for o in objects):
        raise RuntimeError("private test stream must remain unlinked")
    return base.inventory.project(snapshot, {app["application_id"]: WIRE_NAME})


def stop_owned(process):
    if process.poll() is not None:
        return True
    process.terminate()
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        process.kill()
        try:
            process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            return False
    return process.poll() is not None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stage", choices=("prepare", "prepare-connected", "source-probe", "run", "compare", "compare-connected"))
    parser.add_argument("--evidence-dir", required=True)
    parser.add_argument("--executor", required=True)
    parser.add_argument("--authorize-publication", action="store_true")
    parser.add_argument("--authorize-isolated-source", action="store_true")
    parser.add_argument("--sysfs-device")
    parser.add_argument("--expected-cache")
    parser.add_argument("--cycles", type=int, default=0, help="explicit periodic budget 1..40; default one-shot")
    parser.add_argument("--observed-startup", action="store_true", help="explicit pre-bulk vendor-input preparation experiment")
    parser.add_argument("--observe-rejected-polls", action="store_true", help="explicit bounded rejected-marker observation policy; requires observed-startup")
    parser.add_argument("--initialization-diagnostic", action="store_true", help="single A2 read with a five-second diagnostic limit; all later bulk deadlines stay 500 ms")
    args = parser.parse_args()
    if args.cycles and not 1 <= args.cycles <= MAX_CYCLES:
        parser.error("periodic cycle budget outside 1..40")
    if args.observed_startup and (not args.cycles or args.stage != "run"):
        parser.error("observed-startup requires a periodic hardware run")
    if args.observe_rejected_polls and (not args.observed_startup or not args.cycles or args.stage != "run"):
        parser.error("rejected-marker observation requires an observed-startup periodic hardware run")
    if args.initialization_diagnostic and not args.observe_rejected_polls:
        parser.error("initialization diagnostic requires explicit observed marker/startup policy")
    source_only = args.stage == "source-probe"
    if args.stage not in ("run", "source-probe"):
        journal = Path(args.evidence_dir) / "handshake.json"
        if args.stage in ("compare", "compare-connected") and (Path(args.evidence_dir) / "run-started.json").exists() and not journal.exists():
            parser.error("attempt completion unconfirmed; manual review required before comparison")
        if args.stage in ("compare", "compare-connected") and journal.exists():
            state = json.loads(journal.read_text())
            if state.get("retained_private_process_pids") or not all(state.get("owned_processes_stopped", [False])):
                parser.error("owned private process exit unconfirmed; manual review required")
        previous = sys.argv
        try:
            sys.argv = ["shared-usb-runner", args.stage, "--evidence-dir", args.evidence_dir, "--executor", args.executor]
            if args.stage == "prepare-connected" and args.sysfs_device:
                sys.argv += ["--sysfs-device", args.sysfs_device]
            return base.main()
        finally:
            sys.argv = previous
    root = Path(args.evidence_dir).resolve(strict=True)
    binary = Path(args.executor).resolve(strict=True)
    if sys.platform != "linux" or root == base.REPO or base.REPO in root.parents:
        parser.error("requires Linux and private evidence outside the repository")
    before = json.loads((root / "baseline.json").read_text())
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    if source_only and not args.authorize_isolated_source:
        parser.error("source-probe requires authorization for the isolated synthetic source")
    if not source_only and (not args.authorize_publication or not args.sysfs_device or not args.expected_cache):
        parser.error("requires owner-authorized publication and exact selected device/cache")
    if digest != before["binary_sha256"] or not 0 <= time.time() - before["time"] <= 600:
        parser.error("changed binary or stale baseline")
    if (root / "run-started.json").exists() or (root / "handshake.json").exists():
        parser.error("evidence already used; never retry this attempt")
    found = base.targets()
    cache_hash = None
    if source_only:
        if found:
            parser.error("source-only probe requires Studio to remain on Windows")
    else:
        selected = Path(args.sysfs_device).resolve(strict=True)
        if len(found) != 1 or found[0].resolve() != selected:
            parser.error("selected target absent or ambiguous")
        if before.get("scope") == "connected" and base.connected_binding(selected) != before["binding"]:
            parser.error("connected target differs from baseline")
        with Path(args.expected_cache).open("rb") as handle:
            raw = handle.read(256 * 1024 + 1)
        if len(raw) > 256 * 1024:
            parser.error("expected cache exceeds limit")
        pinned = root / "expected-cache.bin"
        with pinned.open("xb") as handle:
            handle.write(raw)
        cache_hash = hashlib.sha256(raw).hexdigest()
    attached = base.snapshot()
    base.save(root / "run-started.json", {"snapshot": attached, "binary_sha256": digest,
                                          "expected_cache_sha256": cache_hash, "usb_io_requested": not source_only,
                                          "wire_name": WIRE_NAME, "initial_assignment": 0,
                                          "periodic_cycles": args.cycles,
                                          "observed_startup": args.observed_startup,
                                          "observe_rejected_polls": args.observe_rejected_polls,
                                          "initialization_diagnostic": args.initialization_diagnostic})
    # PipeWire also creates a '-manager' socket; evidence paths can exceed the
    # Linux Unix-socket limit. mkdtemp retains exclusive 0700 ownership.
    runtime = Path(tempfile.mkdtemp(prefix="beacn-pub-"))
    config = root / "private-config"
    config.mkdir(mode=0o700)
    remote = "pub-" + uuid.uuid4().hex[:8]
    modules = ["protocol-native", "metadata", "spa-node-factory", "client-node", "adapter", "link-factory", "access"]
    conf = {"context.properties": {"core.daemon": True, "core.name": remote, "support.dbus": False},
            "context.spa-libs": {"audio.convert.*": "audioconvert/libspa-audioconvert", "audio.adapt": "audioconvert/libspa-audioconvert", "support.*": "support/libspa-support"},
            "context.modules": [{"name": "libpipewire-module-" + name} for name in modules],
            "context.objects": [], "context.exec": []}
    client = {**conf, "context.properties": {"core.daemon": False, "support.dbus": False},
              "context.modules": [{"name": "libpipewire-module-" + name} for name in ("protocol-native", "client-node", "adapter", "metadata")]}
    (config / "server.conf").write_text(json.dumps(conf))
    (config / "client.conf").write_text(json.dumps(client))
    env = {k: v for k, v in os.environ.items() if not k.startswith("PIPEWIRE_")}
    env.update(PIPEWIRE_RUNTIME_DIR=str(runtime), XDG_RUNTIME_DIR=str(runtime), PIPEWIRE_REMOTE=remote,
               PIPEWIRE_CONFIG_PREFIX=str(config), PIPEWIRE_CONFIG_NAME="client.conf", XDG_STATE_HOME=str(runtime / "state"))
    processes = []
    logs = []
    playback = None
    server = None
    private_baseline = None
    result = {"binary_sha256": digest, "child_exited": True, "exit": 2, "timed_out": False,
              "wire_name": WIRE_NAME, "remote_visibility": "unobserved", "routes_applied": False}
    result["usb_io_requested"] = not source_only
    result["periodic_cycles"] = args.cycles
    result["observed_startup"] = args.observed_startup
    result["observe_rejected_polls"] = args.observe_rejected_polls
    result["initialization_diagnostic"] = args.initialization_diagnostic
    result["private_runtime"] = str(runtime)

    def start(name, command, child_env, stdin=None):
        log = (root / (name + ".log")).open("xb")
        logs.append(log)
        p = subprocess.Popen(command, env=child_env, stdin=stdin or subprocess.DEVNULL, stdout=log, stderr=log)
        processes.append(p)
        return p

    def dump():
        p = subprocess.run(["pw-dump", "-N", "-r", remote], env=env, capture_output=True, timeout=3)
        if p.returncode or len(p.stdout) > base.inventory.MAX_DUMP_BYTES:
            raise RuntimeError("private inventory query failed")
        return p.stdout, base.inventory.decode_dump(p.stdout)

    try:
        server = start("private-core", ["pipewire", "-c", "server.conf"], {**env, "PIPEWIRE_CONFIG_NAME": "server.conf"})
        deadline = time.monotonic() + 4
        while not (runtime / remote).is_socket():
            if server.poll() is not None or time.monotonic() >= deadline:
                raise RuntimeError("private core did not become ready")
            time.sleep(0.05)
        _, objects = dump()
        private_baseline = base.core.filtered(objects)
        base.save(root / "private-core-baseline.json", private_baseline)
        pcm = root / "owned-synthetic-silence.raw"
        pcm.write_bytes(bytes(1024 * 1024))
        node_name = "beacn-publication-" + uuid.uuid4().hex[:8]
        properties = {"node.name": node_name, "application.name": "BEACN Linux VM Test", "node.autoconnect": False,
                      "node.dont-reconnect": True, "state.restore-props": False, "state.restore-target": False}
        with pcm.open("rb") as handle:
            playback = start("owned-playback", ["pw-cat", "-p", "-a", "-R", remote, "--target", "0", "--rate", "48000", "--channels", "2", "--format", "s16", "-P", json.dumps(properties), "-"], env, handle)
        deadline = time.monotonic() + 4
        while True:
            raw_dump, objects = dump()
            try:
                inventory = owned_inventory(objects, playback.pid, node_name, time.time_ns())
                break
            except RuntimeError:
                if playback.poll() is not None or time.monotonic() >= deadline:
                    raise
                time.sleep(0.05)
        (root / "application-dump.json").write_bytes(raw_dump)
        base.save(root / "application-inventory.json", inventory)
        text = base.inventory.to_tsv(inventory)
        projected = root / "inventory.tsv"
        projected.write_text(text, encoding="ascii")
        # Confirm the same owned source immediately before the single native child.
        _, objects = dump()
        if playback.poll() is not None or owned_inventory(objects, playback.pid, node_name, inventory["snapshot_id"]) != inventory:
            raise RuntimeError("owned source changed before native execution")
        if args.cycles:
            last_id = inventory["snapshot_id"]

            def fresh(request):
                nonlocal last_id
                if server.poll() is not None or playback.poll() is not None:
                    raise RuntimeError("owned source/core exited")
                raw, current_objects = dump()
                last_id = max(last_id + 1, time.time_ns())
                current = owned_inventory(current_objects, playback.pid, node_name, last_id)
                if current["applications"] != inventory["applications"]:
                    raise RuntimeError("pinned source identity changed")
                (root / f"cycle-{request:03d}-dump.json").write_bytes(raw)
                base.save(root / f"cycle-{request:03d}-inventory.json", current)
                return current

            command = ([str(binary), "--check-periodic", str(args.cycles)] if source_only else
                       [str(binary), "--authorize-periodic-initialization-diagnostic" if args.initialization_diagnostic else "--authorize-periodic-observed-polls" if args.observe_rejected_polls else "--authorize-periodic-startup" if args.observed_startup else "--authorize-periodic", str(selected), str(pinned), str(args.cycles)])
            result.update(execute_periodic(command, fresh, args.cycles))
        else:
            command = ([str(binary), "--check-inventory", str(projected), str(inventory["snapshot_id"])] if source_only else
                       [str(binary), "--authorize-publication", str(selected), str(pinned), str(projected), str(inventory["snapshot_id"])])
            result.update(base.execute_child(command, timeout=12 if source_only else 30))
        _, objects = dump()
        result["owned_source_preserved"] = playback.poll() is None and owned_inventory(objects, playback.pid, node_name, inventory["snapshot_id"]) == inventory
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        result["runner_error"] = type(error).__name__ + ": " + str(error)
    finally:
        if result["child_exited"]:
            stopped = []
            for p in reversed(processes):
                stopped.append(stop_owned(p))
                if p is playback and stopped[-1] and server.poll() is None:
                    try:
                        deadline = time.monotonic() + 2
                        while True:
                            _, objects = dump()
                            restored = base.core.filtered(objects)
                            if restored == private_baseline or time.monotonic() >= deadline:
                                break
                            time.sleep(0.05)
                        result["private_core_restored"] = restored == private_baseline
                        base.save(root / "private-core-cleaned.json", restored)
                    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired):
                        result["private_core_restored"] = False
            result["owned_processes_stopped"] = stopped
            if not all(stopped):
                result["retained_private_process_pids"] = [p.pid for p in processes if p.poll() is None]
        else:
            result["retained_private_process_pids"] = [p.pid for p in processes if p.poll() is None]
        for log in logs:
            log.close()
        result["after_owned_cleanup"] = base.snapshot()
        base.save(root / "handshake.json", result)
    print(json.dumps({k: v for k, v in result.items() if k not in ("after_child", "after_owned_cleanup", "private_child_pid", "retained_private_process_pids")}))
    return 0 if result["exit"] == 0 and not result["timed_out"] and result.get("owned_source_preserved") and result.get("private_core_restored") and all(result.get("owned_processes_stopped", [False])) and "runner_error" not in result and "supervisor_error" not in result else 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        print("publication test stopped: " + type(error).__name__, file=sys.stderr)
        raise SystemExit(2)
