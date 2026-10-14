#!/usr/bin/env python3
"""Authorized, finite Studio playback probe; no USB vendor I/O or native apply."""
import argparse
import copy
import hashlib
import json
import math
import os
import re
from pathlib import Path
import struct
import subprocess
import sys
import time
import uuid

sys.dont_write_bytecode = True
REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "tools"))
import pipewire_inventory as inventory
import pipewire_graph as graph
import usb_handshake as owner

RATE = 48000
LEAD = 8
DURATION = 12


def collect():
    env = dict(os.environ, XDG_RUNTIME_DIR="/run/user/" + str(os.getuid()))
    for key in ("PIPEWIRE_REMOTE", "PIPEWIRE_RUNTIME_DIR", "PIPEWIRE_CONFIG_DIR",
                "PIPEWIRE_CONFIG_PREFIX", "PIPEWIRE_CONFIG_NAME"):
        env.pop(key, None)
    result = subprocess.run(["pw-dump", "-N", "-r", "pipewire-0"], env=env,
                            capture_output=True, timeout=2)
    if result.returncode or len(result.stderr) > 16384:
        raise RuntimeError("bounded desktop graph query failed")
    return inventory.decode_dump(result.stdout)


def number(value):
    if isinstance(value, str) and value.startswith("0x"):
        return int(value, 16)
    return inventory.uint(value)


def positions(value):
    if not isinstance(value, str):
        raise ValueError("missing explicit split-channel positions")
    return value.replace("[", "").replace("]", "").replace(" ", "").split(",")


def ports(objects, node, direction, channels):
    candidates = [o for o in objects if o.get("type") == graph.PORT
                  and number(inventory.properties(o).get("node.id")) == node["id"]
                  and inventory.properties(o).get("port.direction") == direction]
    if sorted(inventory.properties(o).get("audio.channel") for o in candidates) != sorted(channels):
        raise ValueError("missing or extra selected audio ports")
    return {inventory.properties(o)["audio.channel"]: graph.ref(o) for o in candidates}


def endpoint(objects, card, link):
    """Pin a selected UCM split pair and its actual internal hardware links."""
    devices = [o for o in objects if o.get("type") == "PipeWire:Interface:Device"
               and number(inventory.properties(o).get("device.vendor.id", 0)) == 0x33ae
               and number(inventory.properties(o).get("device.product.id", 0)) == 0x4003]
    if len(devices) != 1 or number(inventory.properties(devices[0]).get("api.alsa.card")) != card:
        raise ValueError("selected USB audio device is absent or ambiguous")
    device = graph.ref(devices[0])
    pair = ["AUX" + str((link - 1) * 2 + offset) for offset in range(2)]
    hardware_channels = ["AUX" + str(i) for i in range(8)]
    candidates = [o for o in objects if o.get("type") == inventory.NODE
                  and inventory.properties(o).get("media.class") == "Audio/Sink"
                  and number(inventory.properties(o).get("device.id", 0)) == device["id"]
                  and positions(inventory.properties(o).get("api.alsa.split.position", "[]")) == pair]
    if len(candidates) != 1:
        raise ValueError("selected hardware split pair is absent or ambiguous")
    target = candidates[0]
    p = inventory.properties(target)
    if (number(p.get("api.alsa.pcm.card")) != card or p.get("api.alsa.pcm.stream") != "playback"
            or positions(p.get("audio.position")) != ["FL", "FR"]
            or positions(p.get("api.alsa.split.hw-position")) != hardware_channels):
        raise ValueError("selected pair has inconsistent ALSA mapping")
    hardware = [o for o in objects if o.get("type") == inventory.NODE
                and inventory.properties(o).get("media.class") == "Audio/Sink/Internal"
                and inventory.properties(o).get("node.name") == p.get("api.alsa.split.name")
                and number(inventory.properties(o).get("device.id", 0)) == device["id"]]
    outputs = [o for o in objects if o.get("type") == inventory.NODE
               and inventory.properties(o).get("media.class") == "Stream/Output/Audio/Internal"
               and inventory.properties(o).get("node.link-group") == p.get("node.link-group")]
    if not p.get("node.link-group") or len(hardware) != 1 or len(outputs) != 1:
        raise ValueError("split-to-hardware identity is absent or ambiguous")
    hp, op = inventory.properties(hardware[0]), inventory.properties(outputs[0])
    if (positions(hp.get("audio.position")) != hardware_channels
            or number(hp.get("api.alsa.pcm.card")) != card
            or op.get("target.object") != hp.get("node.name")
            or positions(op.get("audio.position")) != pair):
        raise ValueError("internal hardware mapping changed")
    target_ref, output_ref, hardware_ref = map(graph.ref, (target, outputs[0], hardware[0]))
    inputs = ports(objects, target_ref, "in", ["FL", "FR"])
    split_outputs = ports(objects, output_ref, "out", pair)
    hardware_inputs = ports(objects, hardware_ref, "in", hardware_channels)
    _, projected = graph.project_graph(objects, 1, {})
    links = [l for l in projected["links"] if l["output"] in split_outputs.values()]
    expected = {(split_outputs[c]["serial"], hardware_inputs[c]["serial"]) for c in pair}
    if len(links) != 2 or {(l["output"]["serial"], l["input"]["serial"]) for l in links} != expected:
        raise ValueError("internal split pair is not linked exclusively to the hardware pair")
    return {"device": device, "target": target_ref, "inputs": inputs, "pair": pair,
            "internal_output": output_ref, "hardware": hardware_ref, "hardware_links": links}


def stream_properties(token, target, rate=RATE, force_rate=False):
    if rate not in (48000, 96000):
        raise ValueError("unsupported diagnostic rate")
    result = {"application.name": "BEACN Linux VM Audio Test", "application.id": "beacn-test-" + token,
            "node.name": "beacn-audio-" + token, "node.autoconnect": True,
            "target.object": str(target["serial"]), "node.dont-fallback": True,
            "node.dont-reconnect": True, "node.dont-move": True, "stream.dont-remix": True,
            "state.restore-props": False, "state.restore-target": False, "object.linger": False}
    if force_rate:
        result["node.force-rate"] = rate
    return result


def require_rate_isolation(objects, selected_device, owned_node=None):
    """A forced graph rate is permitted only on the idle selected-device test graph."""
    for obj in objects:
        props = inventory.properties(obj)
        if obj.get("type") == inventory.NODE and obj["id"] != owned_node:
            if props.get("media.class") in ("Stream/Output/Audio", "Stream/Input/Audio"):
                raise RuntimeError("another application stream blocks the graph-rate diagnostic")
            if graph.audio_node(obj) and props.get("device.id") is not None and number(props["device.id"]) != selected_device:
                raise RuntimeError("another physical audio device blocks the graph-rate diagnostic")
            if props.get("node.force-rate") is not None and number(props["node.force-rate"]) != 0:
                raise RuntimeError("another node already forces a graph rate")
        if obj.get("type") == "PipeWire:Interface:Metadata":
            for entry in obj.get("metadata", []):
                if entry.get("key") == "clock.force-rate" and number(entry.get("value")) != 0:
                    raise RuntimeError("existing global rate override blocks the diagnostic")


def verify(objects, pinned, card, link, token, pid, baseline, rate=RATE, force_rate=False):
    if endpoint(objects, card, link) != pinned:
        raise ValueError("selected physical identities or pair links changed")
    apps = inventory.observe(objects, 1)["applications"]
    matches = [a for a in apps if a["process_pid"] == pid and "beacn-test-" + token in a["application_ids"]]
    if len(matches) != 1 or len(matches[0]["streams"]) != 1:
        raise LookupError("owned playback stream not yet available")
    app = matches[0]
    projected, current = graph.project_graph(objects, 1, {app["application_id"]: "beacn-audio-test"})
    source = next(s["node"] for s in current["streams"] if s["application_id"] == app["application_id"])
    if force_rate:
        require_rate_isolation(objects, pinned["device"]["id"], source["id"])
    node = next(o for o in objects if o["id"] == source["id"])
    p = inventory.properties(node)
    for key, expected in stream_properties(token, pinned["target"], rate, force_rate).items():
        actual = p.get(key)
        if isinstance(expected, bool):
            if actual not in (expected, str(expected).lower()):
                raise ValueError("owned stream safety property changed")
        elif str(actual) != str(expected):
            raise ValueError("owned stream identity or target property changed")
    output = ports(objects, source, "out", ["FL", "FR"])
    selected = [l for l in current["links"] if l["output"] in output.values() or l["input"] in output.values()]
    expected = {(output[c]["serial"], pinned["inputs"][c]["serial"]) for c in ("FL", "FR")}
    actual = {(l["output"]["serial"], l["input"]["serial"]) for l in selected}
    if actual - expected or len(selected) > 2:
        raise ValueError("owned stream has wrong-channel, foreign, or fan-out links")
    if len(selected) != 2 or actual != expected or node["info"].get("state") != "running":
        raise LookupError("owned stereo playback route not yet running")
    if current["core_id"] != baseline["core_id"]:
        raise ValueError("desktop core changed")
    for category in ("nodes", "ports", "links", "streams"):
        if any(item not in current[category] for item in baseline[category]):
            raise ValueError("foreign graph object or route changed")
    return {"source": source, "links": selected, "graph": current, "inventory": projected}


def pcm(audible, rate=RATE):
    """Authored stereo PCM only: eight silent seconds then three quiet beeps."""
    if rate not in (48000, 96000):
        raise ValueError("unsupported diagnostic rate")
    data = bytearray(DURATION * rate * 4)
    if audible:
        for second in (LEAD, LEAD + 1, LEAD + 2):
            for i in range(rate // 2):
                ramp = rate // 100
                envelope = min(1.0, i / ramp, (rate // 2 - 1 - i) / ramp)
                value = round(32767 * 10 ** (-24 / 20) * envelope * math.sin(2 * math.pi * 440 * i / rate))
                struct.pack_into("<hh", data, (second * rate + i) * 4, value, value)
    return bytes(data)


def compare_settings(before, after, pinned):
    """Report an exact comparison plus one explicitly known negotiated-format cache."""
    clean = copy.deepcopy(after)
    before_nodes = {n["id"]: n for n in before["desktop"]["Node"]}
    format_cache = []
    for node in clean["desktop"]["Node"]:
        prior = before_nodes.get(node["id"])
        if (prior is not None and node["id"] == pinned["internal_output"]["id"]
                and str(node["serial"]) == str(pinned["internal_output"]["serial"])):
            expected = {"mediaType": "audio", "mediaSubtype": "raw", "format": "F32P",
                        "channels": 2, "position": pinned["pair"]}
            old = prior.get("params", {}).get("Format")
            new = node.get("params", {}).get("Format")
            if old == [] and new == [expected]:
                node["params"]["Format"] = []
                format_cache.append({"node": pinned["internal_output"], "before": old, "after": new})
    return {"desktop_exact_equal": before["desktop"] == after["desktop"],
            "settings_routes_exact_equal": before["desktop"] == clean["desktop"],
            "state_files_exact_equal": before["state_files"] == after["state_files"],
            "negotiated_format_cache_changes": format_cache, "after": after}


def settled_comparison(baseline, get_binding, snapshot=None, sleep=None):
    snapshot = snapshot or owner.snapshot
    sleep = sleep or time.sleep
    for _ in range(20):
        try:
            after = snapshot()
        except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
            # Unreachable is not an unsettled format cache; no repeated queries.
            return {"error": str(error), "desktop_available": False, "cleanup_verified": False}
        try:
            after_binding = get_binding()
            comparison = compare_settings(baseline["snapshot"], after, baseline["endpoint"])
            comparison["connected_binding_equal"] = after_binding == baseline["binding"]
            if all(comparison[k] for k in ("settings_routes_exact_equal", "state_files_exact_equal", "connected_binding_equal")):
                return comparison
        except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
            comparison = {"error": str(error)}
        sleep(0.5)
    return comparison


def planner_check(proof, codec, target, token, link):
    """Existing pure planner must recognize the actual route as zero operations."""
    encoded = subprocess.run([str(codec), "encode", str(link) + ":beacn-audio-test"],
                             capture_output=True, timeout=1)
    if encoded.returncode or len(encoded.stdout) > 1024 or len(encoded.stderr) > 1024:
        raise RuntimeError("bounded existing codec check failed")
    frame = encoded.stdout.decode("ascii").strip()
    text = graph.preparation_tsv(proof["inventory"], proof["graph"], proof["graph"],
                                 [{"assignment": link, "node": target}], token, [])
    checked = subprocess.run([str(codec), "prepare-graph", "1", frame], input=text.encode("ascii"),
                             capture_output=True, timeout=1)
    if checked.returncode or not checked.stdout.startswith(
            b"DRY RUN: 0 guarded operations; actual application disabled\n"):
        raise RuntimeError("existing planner did not verify the actual stereo route")
    return {"exit": checked.returncode, "operations": 0, "application_enabled": False}


def clock_sample(path):
    raw = path.read_text()
    state = re.search(r"^state: (\w+)$", raw, re.M)
    pointer = re.search(r"^hw_ptr\s+: (\d+)$", raw, re.M)
    if len(raw) > 16384 or state is None or state.group(1) != "RUNNING" or pointer is None:
        raise RuntimeError("selected ALSA playback clock is not running")
    return raw, int(pointer.group(1))


def audio_activation(directory, baseline):
    """Cached sysfs only; audio alternates may activate, every identity stays pinned."""
    selected = Path(directory).resolve(strict=True)
    first = owner.usb_cache.sample(selected)
    instance, attributes, interfaces, raw = first
    if (list(instance) != baseline["instance"] or attributes != baseline["attributes"]
            or hashlib.sha256(raw).hexdigest() != baseline["descriptor_sha256"]):
        raise RuntimeError("selected USB identity changed during playback")
    expected = {i["number"]: i for i in baseline["interfaces"]}
    if len(interfaces) != len(expected) or {i["number"] for i in interfaces} != set(expected):
        raise RuntimeError("selected USB interface set changed during playback")
    endpoints = []
    for interface in interfaces:
        prior = expected[interface["number"]]
        if any(interface[k] != prior[k] for k in ("class", "driver")):
            raise RuntimeError("selected USB interface binding changed during playback")
        allowed = (0, 1) if interface["number"] in (1, 2) else (prior["alternate"],)
        if interface["alternate"] not in allowed:
            raise RuntimeError("unexpected selected USB alternate during playback")
        folder = selected / (selected.name + ":1." + str(interface["number"]))
        for ep in sorted(folder.glob("ep_*")):
            values = {name: owner.usb_cache.cached(ep / name, 64).decode("ascii").strip()
                      for name in ("bEndpointAddress", "bmAttributes", "wMaxPacketSize", "bInterval", "type", "direction")
                      if (ep / name).is_file()}
            endpoints.append({"interface": interface["number"], "values": values})
    if owner.usb_cache.sample(selected) != first:
        raise RuntimeError("selected USB activation changed across observation")
    return {"interfaces": interfaces, "endpoints": endpoints,
            "audio_alternates": {str(i["number"]): i["alternate"] for i in interfaces if i["number"] in (1, 2)},
            "both_audio_alternates_active": all(i["alternate"] == 1 for i in interfaces if i["number"] in (1, 2))}


def binding(directory, held_pid=None, held_start=None):
    if held_pid is None:
        return owner.connected_binding(directory)
    selected = Path(directory).resolve(strict=True)
    found = owner.targets()
    if len(found) != 1 or found[0].resolve() != selected:
        raise RuntimeError("held selected USB device absent or ambiguous")
    process = Path("/proc") / str(held_pid)
    # Coexistence checks, never authority to release the caller's claim.
    if process.joinpath("stat").read_text().rsplit(")", 1)[1].split()[19] != held_start:
        raise RuntimeError("held vendor child identity changed")
    node = Path("/dev/bus/usb") / ("%03d" % int((selected / "busnum").read_text())) / ("%03d" % int((selected / "devnum").read_text()))
    expected = node.stat()
    opened = False
    for fd in process.joinpath("fd").iterdir():
        try:
            actual = fd.stat()
            opened |= (actual.st_dev, actual.st_ino, actual.st_rdev) == (expected.st_dev, expected.st_ino, expected.st_rdev)
        except FileNotFoundError:
            continue
    if not opened:
        raise RuntimeError("held vendor child no longer retains the selected USB device")
    first = owner.usb_cache.sample(selected)
    if owner.usb_cache.sample(selected) != first:
        raise RuntimeError("held selected device changed across observation")
    instance, attributes, interfaces, raw = first
    if (attributes["bConfigurationValue"] != 1 or [(i["number"], i["alternate"]) for i in interfaces] != [(0, 0), (1, 0), (2, 0), (3, 1)]
            or interfaces[-1]["driver"] not in (None, "usbfs")):
        raise RuntimeError("held device is not in its expected coexistence baseline")
    return {"instance": list(instance), "attributes": attributes, "interfaces": interfaces,
            "descriptor_sha256": hashlib.sha256(raw).hexdigest(), "held_pid": held_pid, "held_start": held_start}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--authorize-physical-playback", action="store_true")
    parser.add_argument("--audible", action="store_true", help="requires owner ready to observe; silence is default")
    parser.add_argument("--link", type=int, choices=(1, 2, 3, 4), required=True)
    parser.add_argument("--sysfs-device", required=True)
    parser.add_argument("--evidence-dir", required=True)
    parser.add_argument("--codec", required=True, help="exact built existing link-protocol executable")
    parser.add_argument("--rate", type=int, choices=(48000, 96000), default=RATE)
    parser.add_argument("--force-graph-rate", action="store_true", help="explicit temporary rate on the owned node only")
    parser.add_argument("--held-vendor-pid", type=int, help="caller-owned initialized native helper, never stopped here")
    parser.add_argument("--held-vendor-start")
    args = parser.parse_args()
    if not args.authorize_physical_playback or sys.platform != "linux":
        parser.error("requires Linux and authorized physical playback")
    if (args.held_vendor_pid is None) != (args.held_vendor_start is None):
        parser.error("held vendor coexistence requires exact process start identity")
    root = Path(args.evidence_dir).resolve()
    if root == REPO or REPO in root.parents:
        parser.error("private evidence must be outside the source tree")
    root.mkdir(mode=0o700, parents=False, exist_ok=False)
    journal = {"audible": args.audible, "link": args.link, "native_executor_apply": False,
               "rate": args.rate, "force_graph_rate": args.force_graph_rate,
               "error": None, "child_exited": True, "route_checks": 0}
    child = None
    log = None
    try:
        get_binding = lambda: binding(args.sysfs_device, args.held_vendor_pid, args.held_vendor_start)
        before_binding = get_binding()
        before = owner.snapshot()
        objects = collect()
        selected = Path(args.sysfs_device).resolve(strict=True)
        cards = [int(p.name[4:]) for p in Path("/sys/class/sound").glob("card[0-9]*")
                 if selected in (p / "device").resolve(strict=True).parents]
        if len(cards) != 1:
            raise ValueError("selected USB device does not uniquely identify an ALSA card")
        card = cards[0]
        pinned = endpoint(objects, card, args.link)
        if args.force_graph_rate:
            require_rate_isolation(objects, pinned["device"]["id"])
        _, baseline_graph = graph.project_graph(objects, 1, {})
        if get_binding() != before_binding or owner.snapshot() != before:
            raise ValueError("baseline changed during preparation")
        binary = Path("/usr/bin/pw-cat").resolve(strict=True)
        codec = Path(args.codec).resolve(strict=True)
        journal.update(binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                       codec_sha256=hashlib.sha256(codec.read_bytes()).hexdigest(),
                       harness_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
        owner.save(root / "baseline.json", {"snapshot": before, "binding": before_binding, "endpoint": pinned})
        owner.save(root / "baseline-objects.json", objects)
        raw = pcm(args.audible, args.rate)
        (root / "authored-pcm.s16le").write_bytes(raw)
        journal["pcm_sha256"] = hashlib.sha256(raw).hexdigest()
        config = root / "client-config"
        config.mkdir(mode=0o700)
        (config / "client.conf").write_text(json.dumps({"context.properties": {"support.dbus": False},
            "context.spa-libs": {"audio.convert.*": "audioconvert/libspa-audioconvert", "support.*": "support/libspa-support"},
            "context.modules": [{"name": "libpipewire-module-" + name} for name in
                                ("protocol-native", "client-node", "adapter", "metadata")]}))
        env = dict(os.environ, XDG_RUNTIME_DIR="/run/user/" + str(os.getuid()))
        for key in ("PIPEWIRE_REMOTE", "PIPEWIRE_RUNTIME_DIR", "PIPEWIRE_CONFIG_DIR"):
            env.pop(key, None)
        env.update(PIPEWIRE_CONFIG_PREFIX=str(config), PIPEWIRE_CONFIG_NAME="client.conf")
        token = uuid.uuid4().hex
        command = [str(binary), "-v", "-p", "-a", "-R", "pipewire-0", "--target", str(pinned["target"]["serial"]),
                   "--rate", str(args.rate), "--channels", "2", "--channel-map", "FL,FR", "--format", "s16",
                   "-P", json.dumps(stream_properties(token, pinned["target"], args.rate, args.force_graph_rate)), str(root / "authored-pcm.s16le")]
        log = (root / "pw-cat.log").open("xb")
        owner.save(root / "run-started.json", {"time": time.time(), "token": token})
        child = subprocess.Popen(command, env=env, stdin=subprocess.DEVNULL, stdout=log, stderr=log)
        journal.update(child_exited=False, owned_pid=child.pid)
        started = time.monotonic()
        verified = False
        last_pointer = None
        last_progress = started
        while child.poll() is None:
            elapsed = time.monotonic() - started
            if elapsed > DURATION + 5:
                raise RuntimeError("owned playback child exceeded deadline")
            if log.tell() > 65536:
                raise RuntimeError("owned playback log exceeded limit")
            objects = collect()
            try:
                proof = verify(objects, pinned, card, args.link, token, child.pid, baseline_graph, args.rate, args.force_graph_rate)
            except LookupError:
                if child.poll() == 0:
                    break
                if verified or time.monotonic() - started > 4:
                    raise RuntimeError("owned playback route was absent or stopped")
            else:
                verified = True
                journal["route_checks"] += 1
                owner.save(root / ("route-%02d.json" % journal["route_checks"]), {"elapsed": elapsed, "proof": proof})
                if journal["route_checks"] == 1:
                    journal["planner_check"] = planner_check(proof, codec, pinned["target"], token, args.link)
                    status = Path("/proc/asound") / ("card" + str(card)) / "pcm0p/sub0/status"
                    (root / "alsa-playback-status.txt").write_text(status.read_text())
                    alsa = Path("/proc/asound") / ("card" + str(card))
                    (root / "alsa-stream-during.txt").write_text((alsa / "stream0").read_text())
                    hardware_params = (alsa / "pcm0p/sub0/hw_params").read_text()
                    (root / "alsa-playback-hw-params.txt").write_text(hardware_params)
                    hardware_rate = re.search(r"^rate: (\d+) ", hardware_params, re.M)
                    journal["hardware_rate"] = int(hardware_rate.group(1)) if hardware_rate else None
                    if args.force_graph_rate and journal["hardware_rate"] != args.rate:
                        raise RuntimeError("selected ALSA rate does not match the explicitly forced rate")
                    (root / "alsa-capture-hw-params.txt").write_text((alsa / "pcm0c/sub0/hw_params").read_text())
                status = Path("/proc/asound") / ("card" + str(card)) / "pcm0p/sub0/status"
                raw_status, pointer = clock_sample(status)
                (root / ("alsa-status-%02d.txt" % journal["route_checks"])).write_text(raw_status)
                activation = audio_activation(args.sysfs_device, before_binding)
                owner.save(root / ("usb-activation-%02d.json" % journal["route_checks"]), activation)
                journal["last_audio_alternates"] = activation["audio_alternates"]
                journal["both_audio_alternates_active"] = activation["both_audio_alternates_active"]
                if last_pointer is None:
                    journal["first_hw_pointer"] = pointer
                if last_pointer is None or pointer > last_pointer:
                    last_progress = time.monotonic()
                elif pointer < last_pointer:
                    raise RuntimeError("selected ALSA playback clock restarted")
                journal["last_hw_pointer"] = pointer
                last_pointer = pointer
                if time.monotonic() - last_progress > 3:
                    raise RuntimeError("selected ALSA playback clock stalled for three seconds")
            time.sleep(0.3)
        journal["exit"] = child.returncode
        if not verified or child.returncode:
            raise RuntimeError("playback did not complete a verified physical route")
    except (OSError, ValueError, RuntimeError, LookupError, subprocess.TimeoutExpired) as error:
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
        baseline_path = root / "baseline.json"
        if journal["child_exited"] and baseline_path.exists():
            baseline = json.loads(baseline_path.read_text())
            comparison = settled_comparison(baseline, get_binding)
            owner.save(root / "comparison.json", comparison)
            journal["comparison"] = {k: v for k, v in comparison.items() if k != "after"}
        owner.save(root / "journal.json", journal)
    print(json.dumps(journal))
    return 0 if (journal["error"] is None and journal["child_exited"] and all(journal.get("comparison", {}).get(k)
        for k in ("settings_routes_exact_equal", "state_files_exact_equal", "connected_binding_equal"))) else 2


if __name__ == "__main__":
    raise SystemExit(main())
