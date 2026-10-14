#!/usr/bin/env python3
"""Explicitly authorized, disposable-core native executor test. Never a CI test."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time
import uuid

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import pipewire_inventory as inventory


def props(obj):
    return obj.get("info", {}).get("props", obj.get("props", {}))


def filtered(objects):
    """Settings, channels, routes and defaults; omit runtime state and titles."""
    result = {kind: [] for kind in ("Node", "Port", "Link", "Device", "Metadata")}
    for obj in objects:
        kind = obj.get("type", "").split(":")[-1]
        if kind not in result:
            continue
        p = props(obj)
        item = {"id": obj["id"], "serial": p.get("object.serial")}
        if kind == "Node":
            keys = ["object.serial", "client.id", "device.id", "node.virtual", "node.driver",
                    "node.autoconnect", "node.pause-on-idle", "priority.session", "audio.channels",
                    "audio.position", "audio.rate", "audio.format", "target.object", "target.node", "media.class"]
            item["props"] = {k: p[k] for k in keys if k in p}
            if not p.get("media.class", "").startswith("Stream/"):
                item["props"]["node.name"] = p.get("node.name")
            item["params"] = {k: v for k, v in obj.get("info", {}).get("params", {}).items()
                              if k in ("Props", "EnumFormat", "Format", "PortConfig")}
        elif kind in ("Port", "Link"):
            keys = (["object.serial", "node.id", "port.id", "port.direction", "audio.channel", "port.monitor", "port.control"]
                    if kind == "Port" else ["object.serial", "link.output.node", "link.output.port", "link.input.node", "link.input.port", "link.passive"])
            item["props"] = {k: p[k] for k in keys if k in p}
        elif kind == "Device":
            item["props"] = {k: p[k] for k in ("device.api", "device.name", "media.class") if k in p}
            item["params"] = {k: v for k, v in obj.get("info", {}).get("params", {}).items()
                              if k in ("Props", "Profile", "Route", "EnumRoute")}
        else:
            item["name"] = p.get("metadata.name")
            item["entries"] = [x for x in obj.get("metadata", [])
                               if x.get("key") in ("default.audio.sink", "default.audio.source", "default.configured.audio.sink", "default.configured.audio.source", "target.object", "target.node")
                               or x.get("key", "").startswith("clock.") or p.get("metadata.name") == "settings"]
        result[kind].append(item)
    for values in result.values():
        values.sort(key=lambda item: item["id"])
    return result


def state_hashes(env):
    folder = Path(env.get("XDG_STATE_HOME", str(Path.home() / ".local/state"))) / "wireplumber"
    return {name: hashlib.sha256((folder / name).read_bytes()).hexdigest() if (folder / name).is_file() else None
            for name in ("default-nodes", "stream-properties", "default-routes", "default-profile")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--authorize-isolated-routing", action="store_true")
    parser.add_argument("--executor", required=True)
    parser.add_argument("--codec", required=True)
    parser.add_argument("--evidence-dir", required=True)
    parser.add_argument("--fault-tests", action="store_true", help="requires native-test-faults executable")
    args = parser.parse_args()
    if not args.authorize_isolated_routing or sys.platform != "linux":
        parser.error("requires Linux and explicit owner authorization for isolated routing")
    root = Path(args.evidence_dir).resolve()
    repo = Path(__file__).resolve().parents[1]
    if root == repo or repo in root.parents:
        parser.error("evidence must stay outside the repository")
    root.mkdir(mode=0o700, parents=False, exist_ok=False)
    token = uuid.uuid4().hex
    remote = "beacn-test-" + token[:8]
    desktop_env = dict(os.environ)
    temp = None
    processes = []
    logs = []
    journal = {"token": token, "stages": [], "error": None, "binary_sha256": {name: hashlib.sha256(Path(path).read_bytes()).hexdigest() for name, path in (("executor", args.executor), ("codec", args.codec))}, "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    deadline = time.monotonic() + 90
    native_buffer = b""

    def save():
        (root / "journal.json").write_text(json.dumps(journal, indent=2))

    def dump(env, name, server=None):
        if time.monotonic() > deadline:
            raise RuntimeError("isolated test deadline exceeded")
        result = subprocess.run(["pw-dump", "-N", "-r", name], env=env, capture_output=True, timeout=5)
        if result.returncode or len(result.stdout) > inventory.MAX_DUMP_BYTES or len(result.stderr) > 16384:
            raise RuntimeError("bounded metadata query failed")
        objects = inventory.decode_dump(result.stdout)
        if server is not None:
            core = next(obj for obj in objects if obj.get("type") == "PipeWire:Interface:Core")
            assert props(core)["core.name"] == remote and int(props(core)["application.process.id"]) == server.pid
        return objects

    def start(role, command, env, stdin=None):
        log = (root / (role + ".stderr")).open("wb")
        logs.append(log)
        process = subprocess.Popen(command, env=env, stdin=stdin, stdout=subprocess.PIPE if role == "executor" else subprocess.DEVNULL, stderr=log)
        processes.append((role, process))
        return process

    def wait(predicate):
        end = time.monotonic() + 4
        while time.monotonic() < end:
            objects = dump(private_env, remote, server)
            if predicate(objects):
                return objects
            time.sleep(0.05)
        raise RuntimeError("private graph condition did not settle")

    def owned(obj):
        return props(obj).get("beacn.test.token") == token or props(obj).get("beacn.link.owner") == token

    def links(objects):
        return [obj for obj in objects if obj.get("type") == "PipeWire:Interface:Link" and props(obj).get("link.output.node") == source["id"]]

    def stage(label, target, count=2):
        objects = wait(lambda oo: len(links(oo)) == count and all(obj.get("info", {}).get("state") == "active" for obj in links(oo)))
        routed = links(objects)
        assert all(owned(obj) and props(obj)["link.input.node"] == target["id"] for obj in routed)
        journal["stages"].append({"label": label, "links": [{"id": obj["id"], "serial": props(obj)["object.serial"], "output": props(obj)["link.output.port"], "input": props(obj)["link.input.port"], "state": obj["info"]["state"]} for obj in routed]})
        save()
        return routed

    def native_command(command, success=True):
        nonlocal native_buffer
        executor.stdin.write((command + "\n").encode())
        executor.stdin.flush()
        end = time.monotonic() + 12
        output = b""
        while time.monotonic() < end:
            while b"\n" in native_buffer:
                line, native_buffer = native_buffer.split(b"\n", 1)
                output += line + b"\n"
                if line == b"OK" or line.startswith(b"STOP:"):
                    text = output.decode()
                    journal.setdefault("native_commands", []).append({"command": command, "output": text})
                    save()
                    assert (line == b"OK") == success, text
                    return text
            if executor.poll() is not None:
                raise RuntimeError("native executor exited before command acknowledgment")
            ready, _, _ = select.select([executor.stdout], [], [], 0.1)
            if ready:
                native_buffer += os.read(executor.stdout.fileno(), 8192)
                if len(native_buffer) > 65536:
                    raise RuntimeError("native command output limit exceeded")
        raise RuntimeError("native command acknowledgment deadline exceeded")

    try:
        before = dump(desktop_env, "pipewire-0")
        journal["desktop_baseline"] = filtered(before)
        journal["desktop_state_hashes"] = state_hashes(desktop_env)
        save()
        temp = Path(tempfile.mkdtemp(prefix="beacn-pw-"))
        temp.chmod(0o700)
        modules = ["protocol-native", "metadata", "spa-node-factory", "client-node", "adapter", "link-factory", "access"]
        conf = {"context.properties": {"core.daemon": True, "core.name": remote, "support.dbus": False, "default.clock.rate": 48000, "default.clock.quantum": 1024}, "context.spa-libs": {"audio.convert.*": "audioconvert/libspa-audioconvert", "audio.adapt": "audioconvert/libspa-audioconvert", "support.*": "support/libspa-support"}, "context.modules": [{"name": "libpipewire-module-" + name} for name in modules], "context.objects": [], "context.exec": []}
        client = {**conf, "context.properties": {"core.daemon": False, "support.dbus": False}, "context.modules": [{"name": "libpipewire-module-" + name} for name in ("protocol-native", "client-node", "adapter", "metadata")]}
        config = temp / "config"
        config.mkdir(mode=0o700)
        (config / "server.conf").write_text(json.dumps(conf))
        (config / "client.conf").write_text(json.dumps(client))
        minimal = "context.properties = { support.dbus = false context.num-data-loops = 0 }\ncontext.modules = [ { name = libpipewire-module-protocol-native } ]\ncontext.objects = []\ncontext.exec = []\n"
        (temp / "client.conf").write_text(minimal)
        (temp / "client.conf").chmod(0o600)
        private_env = {k: v for k, v in desktop_env.items() if not k.startswith("PIPEWIRE_")}
        private_env.update(PIPEWIRE_RUNTIME_DIR=str(temp), XDG_RUNTIME_DIR=str(temp), PIPEWIRE_REMOTE=remote, PIPEWIRE_CONFIG_PREFIX=str(config), PIPEWIRE_CONFIG_NAME="client.conf", XDG_STATE_HOME=str(temp / "state"))
        server_env = {**private_env, "PIPEWIRE_CONFIG_NAME": "server.conf"}
        server = start("server", ["pipewire", "-c", "server.conf"], server_env)
        end = time.monotonic() + 4
        while not (temp / remote).is_socket():
            assert server.poll() is None and time.monotonic() < end
            time.sleep(0.05)
        baseline = filtered(dump(private_env, remote, server))
        controller = start("controller", ["pw-cli", "-r", remote], private_env, subprocess.PIPE)
        names = ["beacn-test-" + token + "-" + suffix for suffix in ("A", "B", "source")]
        for name in names[:2]:
            properties = {"factory.name": "support.null-audio-sink", "node.name": name, "media.class": "Audio/Sink", "node.virtual": True, "node.driver": True, "object.linger": False, "node.autoconnect": False, "audio.channels": 2, "audio.position": ["FL", "FR"], "audio.rate": 48000, "state.restore-props": False, "state.restore-target": False, "beacn.test.token": token, "adapter.auto-port-config": {"mode": "dsp", "monitor": False, "position": "preserve"}}
            controller.stdin.write(("create-node adapter " + json.dumps(properties) + "\n").encode())
            controller.stdin.flush()
        destinations = wait(lambda oo: sum(obj.get("type") == inventory.NODE and owned(obj) for obj in oo) == 2)
        # Some supported PipeWire versions leave null sinks in convert mode
        # despite auto-port-config. Configure only our retained controller's
        # private destinations, then require actual labelled stereo ports.
        port_config = {"direction": "Input", "mode": "dsp", "monitor": False, "control": False,
                       "format": {"mediaType": "audio", "mediaSubtype": "raw", "format": "F32P",
                                  "rate": 48000, "channels": 2, "position": ["FL", "FR"]}}
        configured = []
        for name in names[:2]:
            node = next(obj for obj in destinations if obj.get("type") == inventory.NODE
                        and props(obj).get("node.name") == name and owned(obj))
            client = next(obj for obj in destinations if obj.get("type") == "PipeWire:Interface:Client"
                          and obj["id"] == int(props(node)["client.id"]))
            assert int(props(client).get("pipewire.sec.pid", props(client).get("application.process.id", 0))) == controller.pid
            assert props(node).get("object.serial") and props(node).get("media.class") == "Audio/Sink"
            configured.append({"id": node["id"], "serial": props(node)["object.serial"]})
            controller.stdin.write(("set-param " + str(node["id"]) + " PortConfig " + json.dumps(port_config) + "\n").encode())
            controller.stdin.flush()
        wait(lambda oo: all(sorted(props(obj).get("audio.channel", "") for obj in oo
                                   if obj.get("type") == "PipeWire:Interface:Port"
                                   and props(obj).get("node.id") == target["id"]
                                   and props(obj).get("port.direction") == "in") == ["FL", "FR"]
                            for target in configured))
        journal["configured_stereo_destinations"] = configured
        pcm = root / "synthetic-silence.raw"
        pcm.write_bytes(b"\0" * (48000 * 2 * 2 * 60))
        with pcm.open("rb") as handle:
            properties = {"node.name": names[2], "beacn.test.token": token, "node.autoconnect": False, "node.dont-reconnect": True, "state.restore-props": False, "state.restore-target": False, "adapter.auto-port-config": {"mode": "dsp", "monitor": False, "position": "preserve"}}
            playback = start("playback", ["pw-cat", "-p", "-a", "-R", remote, "--target", "0", "--rate", "48000", "--channels", "2", "--channel-map", "FL,FR", "--format", "s16", "-P", json.dumps(properties), "-"], private_env, handle)
        objects = wait(lambda oo: sum(obj.get("type") == inventory.NODE and owned(obj) for obj in oo) == 3)
        a, b, source = [next(obj for obj in objects if obj.get("type") == inventory.NODE and props(obj).get("node.name") == name and owned(obj)) for name in names]
        assert not links(objects)
        observed = inventory.observe(objects, 1)
        app = next(app for app in observed["applications"] if any(stream["node_id"] == source["id"] for stream in app["streams"]))
        assert app["process_pid"] == playback.pid
        core = next(obj for obj in objects if obj.get("type") == "PipeWire:Interface:Core")
        # Preserve the actual test layout when a native observation refuses it.
        # Private-core metadata stays in the caller's evidence directory.
        (root / "prepared-objects.json").write_text(json.dumps(objects, indent=2))
        native_env = {k: v for k, v in private_env.items() if not k.startswith("PIPEWIRE_CONFIG_")}
        executor = start("executor", [str(Path(args.executor).resolve()), str(temp), remote, str(core["info"]["cookie"]), str(server.pid), token, app["application_id"] + "=controlled-test", "1:" + str(a["id"]) + ":" + str(props(a)["object.serial"]), "2:" + str(b["id"]) + ":" + str(props(b)["object.serial"])], native_env, subprocess.PIPE)
        frames = [subprocess.check_output([args.codec, "encode", str(i) + ":controlled-test"], text=True).strip() for i in (1, 2)]
        assert "Complete prefix=2" in native_command("route 1 " + frames[0])
        stage("native-A", a)
        assert "Complete prefix=4" in native_command("route 2 " + frames[1])
        before_failure = stage("native-B", b)
        assert "Failed prefix=3" in native_command("route-fail-before 3 " + frames[0] + " 3", False)
        stage("partial-A", a, 1)
        assert 'Complete, 3' in native_command("rollback")
        restored = stage("restored-B", b)
        assert {props(obj)["object.serial"] for obj in restored}.isdisjoint(props(obj)["object.serial"] for obj in before_failure)
        if args.fault_tests:
            def stats():
                text=native_command("fault-stats")
                match=re.search(r"native-test-stats \[(\d+), (\d+), (\d+)\]",text)
                assert match,text
                return tuple(map(int,match.groups()))

            prior=stats()
            assert "Failed prefix=2" in native_command("route-native-reject 4 " + frames[0] + " 2",False)
            rejected=stats()
            assert rejected==(prior[0]+3,prior[1],prior[2]+1),(prior,rejected)
            assert not links(dump(private_env,remote,server))
            assert 'Complete, 2' in native_command("rollback")
            stage("server-rejection-restored-B",b)

            prior=stats()
            assert "Unknown prefix=2" in native_command("route-native-unknown 5 " + frames[0] + " 2",False)
            stage("unacknowledged-A",a,1)
            held=stats();assert held==(prior[0]+3,prior[1]+1,prior[2])
            native_command("rollback",False)
            native_command("route 6 " + frames[1],False)
            assert stats()==held,"blocked commands submitted a request"
            assert "Failed prefix=3" in native_command("reconcile")
            assert stats()==held,"reconciliation retried the native request"
            assert 'Complete, 3' in native_command("rollback")
            stage("lost-ack-restored-B",b)

            assert "Complete prefix=4" in native_command("route 6 " + frames[0])
            stage("pre-interruption-A",a)
            prior=stats()
            assert 'Unknown, 1' in native_command("rollback-native-unknown 1",False)
            assert not links(wait(lambda oo:not links(oo)))
            held=stats();assert held==(prior[0]+2,prior[1]+1,prior[2])
            native_command("resume-rollback",False)
            native_command("rollback",False)
            assert stats()==held
            assert 'Failed, 2' in native_command("reconcile")
            assert stats()==held
            assert 'Complete, 4' in native_command("resume-rollback")
            resumed=stats();assert resumed[0]==held[0]+2
            stage("interrupted-recovery-restored-B",b)

            assert "Complete prefix=4" in native_command("route 7 " + frames[0])
            prior=stats()
            assert 'Failed, 2' in native_command("rollback-native-reject 2",False)
            rejected=stats();assert rejected==(prior[0]+3,prior[1],prior[2]+1)
            assert not links(dump(private_env,remote,server))
            assert 'Complete, 4' in native_command("resume-rollback")
            assert stats()[0]==rejected[0]+2
            stage("rejected-recovery-restored-B",b)
            prior=stats()
            stable=[(obj['id'],props(obj)['object.serial']) for obj in links(dump(private_env,remote,server))]
            for generation in range(8,40):
                dump(private_env,remote,server)
                assert "Complete prefix=0" in native_command("route " + str(generation) + " " + frames[1])
            assert stats()==prior,"already-correct routes submitted mutation requests"
            assert stable==[(obj['id'],props(obj)['object.serial']) for obj in links(dump(private_env,remote,server))]
            journal['client_churn_iterations']=32
            journal['fault_tests_passed']=True
            journal['native_test_stats']=stats()
        native_command("quit")
        assert executor.wait(timeout=3) == 0
        journal["routing_passed"] = True
    except Exception as exc:
        journal["error"] = type(exc).__name__ + ": " + str(exc)
    finally:
        for role, process in reversed(processes):
            if role == "server":
                continue
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)
            journal.setdefault("process_cleanup", []).append({"role": role, "exit": process.returncode})
        pcm = root / "synthetic-silence.raw"
        if pcm.exists():
            pcm.unlink()
        try:
            if any(role == "server" for role, _ in processes):
                deadline = time.monotonic() + 10
                final = wait(lambda oo: not any(owned(obj) for obj in oo))
                journal["private_baseline_restored"] = filtered(final) == baseline
        except Exception as exc:
            journal["private_cleanup_error"] = str(exc)
        for role, process in processes:
            if role == "server" and process.poll() is None:
                process.terminate()
                process.wait(timeout=3)
        for log in logs:
            log.close()
        journal["diagnostics"] = {p.name: p.read_bytes()[:16384].decode(errors="replace") for p in root.glob("*.stderr")}
        try:
            after = dump(desktop_env, "pipewire-0")
            journal["desktop_after"] = filtered(after)
            journal["desktop_settings_unchanged"] = journal["desktop_after"] == journal["desktop_baseline"]
            journal["desktop_state_files_unchanged"] = state_hashes(desktop_env) == journal["desktop_state_hashes"]
            journal["desktop_has_no_owned_objects"] = not any(owned(obj) for obj in after)
            if temp is not None:
                assert temp.parent == Path("/tmp") and temp.name.startswith("beacn-pw-") and not temp.is_symlink()
                for folder in (temp / "config", temp / "state"):
                    if folder.exists():
                        assert not folder.is_symlink()
                        for item in folder.iterdir():
                            assert folder.name == "config" and item.name in ("server.conf", "client.conf") and item.is_file() and not item.is_symlink()
                            item.unlink()
                        folder.rmdir()
                for item in temp.iterdir():
                    assert item.name in (remote, remote + ".lock", "client.conf") and not item.is_symlink()
                    item.unlink()
                temp.rmdir()
            journal["runtime_removed"] = temp is None or not temp.exists()
        except Exception as exc:
            journal["desktop_cleanup_error"] = str(exc)
        save()
    checks = ("routing_passed", "private_baseline_restored", "desktop_settings_unchanged", "desktop_state_files_unchanged", "desktop_has_no_owned_objects", "runtime_removed")
    if args.fault_tests: checks+=("fault_tests_passed",)
    print(json.dumps({key: journal.get(key) for key in checks} | {"error": journal["error"], "native_test_stats": journal.get("native_test_stats"), "client_churn_iterations": journal.get("client_churn_iterations"), "stages": journal["stages"], "diagnostics": journal.get("diagnostics"), "cleanup_errors": [journal[key] for key in ("private_cleanup_error", "desktop_cleanup_error") if key in journal]}))
    return 0 if all(journal.get(key) for key in checks) and journal["error"] is None else 1


if __name__ == "__main__":
    raise SystemExit(main())
