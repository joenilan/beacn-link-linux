"""Bounded native Studio stage while the initialized USB owner awaits completion.

Only an already verified assignment-1 private route can enter this stage.
No vendor bulk exchanges, capture, defaults, desktop routes or driver changes
occur here; the existing ALSA driver carries the authorized playback data.
"""
import hashlib
import json
import math
from pathlib import Path
import stat
import struct
import subprocess
import time
import uuid

import studio_alsa_playback as alsa

owner = alsa.playback.owner
NODE = "PipeWire:Interface:Node"
PORT = "PipeWire:Interface:Port"
LINK = "PipeWire:Interface:Link"
RATE = 96000


def stereo_pcm(audible=False):
    """Authored 120-second S16LE source, no capture. Tone capped at -24 dBFS."""
    block = bytearray(RATE * 2 * 2 * 4)
    if audible:
        for second in (1, 2, 3):
            frames, ramp = RATE * 35 // 100, RATE // 100
            for i in range(frames):
                envelope = min(1.0, i / ramp, (frames - 1 - i) / ramp)
                value = int(32767 * 10 ** (-24 / 20) * envelope * math.sin(2 * math.pi * 440 * i / RATE))
                struct.pack_into("<hh", block, (second * RATE + i) * 4, value, value)
    # Repeated source pattern avoids losing a finite tone while the bridge is
    # being prepared. Physical output is independently bounded to four seconds.
    return bytes(block) * 30


def props(obj):
    return obj.get("info", {}).get("props", {})


def pcm_path(card, direction="p"):
    return Path("/proc/asound") / ("card%d" % card) / ("pcm0%s/sub0/status" % direction)


def pcms_closed(card):
    return all(pcm_path(card, d).read_text().strip() == "closed" for d in ("p", "c"))


def preflight(selected):
    alsa.require_native_environment()
    selected = Path(selected).resolve(strict=True)
    binding = owner.connected_binding(selected)
    cards = [int(p.name[4:]) for p in Path("/sys/class/sound").glob("card[0-9]*")
             if selected in (p / "device").resolve(strict=True).parents]
    if len(cards) != 1 or not pcms_closed(cards[0]):
        raise RuntimeError("physical stage needs one selected native card with closed PCMs")
    device = Path("/dev/snd/pcmC%dD0p" % cards[0])
    identity = device.stat()
    if not stat.S_ISCHR(identity.st_mode):
        raise RuntimeError("selected playback PCM is not a character device")
    return {"binding": binding, "snapshot": owner.snapshot(), "card": cards[0],
            "pcm_identity": [identity.st_dev, identity.st_ino, identity.st_rdev]}


def client_pid(objects, obj):
    matches = [o for o in objects if o.get("type") == "PipeWire:Interface:Client"
               and o["id"] == int(props(obj).get("client.id", -1))]
    if len(matches) != 1:
        raise RuntimeError("physical stage object client is absent or ambiguous")
    p = props(matches[0])
    return int(p.get("pipewire.sec.pid", p.get("application.process.id", 0)))


def assignment_one(objects, target, target_pid, source_pid, native_pid, token):
    """Require actual complete stereo route, exact serials and live process owners."""
    matches = [o for o in objects if o.get("type") == NODE and o["id"] == target["id"]
               and props(o).get("object.serial") == props(target).get("object.serial")]
    if len(matches) != 1:
        raise RuntimeError("assignment-1 destination changed")
    current = matches[0]
    p = props(current)
    if (p.get("beacn.test.token") != token or p.get("node.virtual") is not True or p.get("media.class") != "Audio/Sink"
            or "device.id" in p or client_pid(objects, current) != target_pid):
        raise RuntimeError("assignment-1 destination ownership refused")
    sources = [o for o in objects if o.get("type") == NODE
               and props(o).get("media.class") == "Stream/Output/Audio"
               and client_pid(objects, o) == source_pid]
    if len(sources) != 1 or sources[0].get("info", {}).get("state") != "running":
        raise RuntimeError("physical stage source changed")
    ports = {o["id"]: o for o in objects if o.get("type") == PORT}
    links = [o for o in objects if o.get("type") == LINK]
    channels = []
    if len(links) != 2:
        raise RuntimeError("physical stage requires exactly two native source-route links")
    for link in links:
        q = props(link)
        output, input_ = ports.get(int(q["link.output.port"])), ports.get(int(q["link.input.port"]))
        if (not q.get("object.serial") or link.get("info", {}).get("state") != "active" or client_pid(objects, link) != native_pid
                or int(q["link.output.node"]) != sources[0]["id"] or int(q["link.input.node"]) != current["id"]
                or output is None or input_ is None or props(output).get("port.direction") != "out"
                or props(input_).get("port.direction") != "in"
                or props(output).get("node.id") != sources[0]["id"] or props(input_).get("node.id") != current["id"]
                or not props(output).get("object.serial") or not props(input_).get("object.serial")
                or props(output).get("audio.channel") != props(input_).get("audio.channel")):
            raise RuntimeError("physical stage native route endpoints/owner refused")
        channels.append(props(output).get("audio.channel"))
    if sorted(channels) != ["FL", "FR"]:
        raise RuntimeError("physical stage requires exact native FL/FR routing")
    return current


def run(selected, before, root, env, remote, dump, server, destinations, native,
        target, source_pid, token, register, audible=False):
    root.mkdir(mode=0o700, exist_ok=False)
    journal = {"error": None, "rate": RATE, "audible": audible, "assignment": 1,
               "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               "vendor_bulk_exchanges_paused": True, "physical_progress_verified": False}
    bridge = None
    log = None
    observations = []
    device = Path("/dev/snd/pcmC%dD0p" % before["card"])
    stage_token = uuid.uuid4().hex
    native_start = Path("/proc/%d/stat" % native.pid).read_text().rsplit(")", 1)[1].split()[19]
    deadline = time.monotonic() + 12

    def wait(predicate):
        end = min(time.monotonic() + 3, deadline)
        while time.monotonic() < end:
            if server.poll() is not None or native.poll() is not None or (bridge is not None and bridge.poll() is not None):
                raise RuntimeError("retained physical-stage owner exited")
            _, objects = dump()
            if predicate(objects):
                return objects
            time.sleep(.05)
        raise RuntimeError("physical stage did not settle within bound")

    def send(command):
        bridge.stdin.write((command + "\n").encode())
        bridge.stdin.flush()

    held = None
    private_before = None
    try:
        if owner.snapshot() != before["snapshot"] or not pcms_closed(before["card"]):
            raise RuntimeError("physical stage pre-command baseline drift")
        held = alsa.playback.binding(selected, native.pid, native_start)
        expected = {**before["binding"], "interfaces": [dict(i) for i in before["binding"]["interfaces"]]}
        expected["interfaces"][-1].update(alternate=1, driver=held["interfaces"][-1]["driver"])
        if any(held[k] != expected[k] for k in ("instance", "attributes", "interfaces", "descriptor_sha256")):
            raise RuntimeError("initialized vendor claim differs from pinned released device")
        _, objects = dump()
        core = next(o for o in objects if o.get("type") == "PipeWire:Interface:Core")
        if int(props(core).get("application.process.id", 0)) != server.pid:
            raise RuntimeError("private server identity changed")
        target = assignment_one(objects, target, destinations.pid, source_pid, native.pid, token)
        private_before = owner.core.filtered(objects)
        owner.save(root / "private-before.json", objects)
        log = (root / "controller.log").open("xb")
        bridge = subprocess.Popen(["pw-cli", "-r", remote], env=env, stdin=subprocess.PIPE, stdout=log, stderr=log)
        register(bridge)
        name = "beacn-physical-" + stage_token
        positions = ["AUX%d" % i for i in range(8)]
        p = {"factory.name": "api.alsa.pcm.sink", "api.alsa.path": "hw:%d,0" % before["card"],
             "api.alsa.use-chmap": False, "node.name": name, "media.class": "Audio/Sink",
             "node.driver": True, "node.pause-on-idle": True, "node.autoconnect": False,
             "object.linger": False, "audio.rate": RATE, "audio.allowed-rates": [RATE],
             "audio.format": "F32LE", "audio.channels": 8, "audio.position": positions,
             "beacn.test.token": stage_token}
        send("create-node adapter " + json.dumps(p))
        def hardware_nodes(objects):
            return [o for o in objects if o.get("type") == NODE and props(o).get("node.name") == name
                    and props(o).get("beacn.test.token") == stage_token]
        objects = wait(lambda oo: len(hardware_nodes(oo)) == 1)
        hardware = hardware_nodes(objects)[0]
        if not props(hardware).get("object.serial") or client_pid(objects, hardware) != bridge.pid:
            raise RuntimeError("retained hardware adapter ownership refused")
        config = {"direction": "Input", "mode": "dsp", "monitor": False, "control": False,
                  "format": {"mediaType": "audio", "mediaSubtype": "raw", "format": "F32P",
                             "rate": RATE, "channels": 8, "position": positions}}
        send("set-param %d PortConfig " % hardware["id"] + json.dumps(config))
        objects = wait(lambda oo: sorted(props(o).get("audio.channel", "") for o in oo if o.get("type") == PORT
                                        and props(o).get("node.id") == hardware["id"] and props(o).get("port.direction") == "in") == positions)
        pairs = set()
        for channel, auxiliary in (("FL", "AUX0"), ("FR", "AUX1")):
            output = next(o for o in objects if o.get("type") == PORT and props(o).get("node.id") == target["id"]
                          and props(o).get("port.direction") == "out" and props(o).get("port.monitor") is True
                          and props(o).get("audio.channel") == channel)
            input_ = next(o for o in objects if o.get("type") == PORT and props(o).get("node.id") == hardware["id"]
                          and props(o).get("port.direction") == "in" and props(o).get("audio.channel") == auxiliary)
            pairs.add((output["id"], input_["id"]))
            send("create-link %d %d %d %d " % (target["id"], int(props(output)["port.id"]), hardware["id"], int(props(input_)["port.id"]))
                 + json.dumps({"object.linger": False, "beacn.test.token": stage_token}))
        def bridge_links(objects):
            return [o for o in objects if o.get("type") == LINK and props(o).get("beacn.test.token") == stage_token]
        objects = wait(lambda oo: {(int(props(o)["link.output.port"]), int(props(o)["link.input.port"])) for o in bridge_links(oo)} == pairs
                       and all(o.get("info", {}).get("state") == "active" for o in bridge_links(oo)))
        journal["bridge_refs"] = [{"id": o["id"], "serial": props(o)["object.serial"]} for o in bridge_links(objects)]
        if any(client_pid(objects, o) != bridge.pid for o in bridge_links(objects)):
            raise RuntimeError("bridge link controller identity changed")
        owner.save(root / "physical-active.json", objects)
        end = min(time.monotonic() + 4, deadline)
        while time.monotonic() < end:
            _, current_objects = dump()
            refs = [{"id": o["id"], "serial": props(o)["object.serial"]} for o in bridge_links(current_objects)]
            if refs != journal["bridge_refs"] or any(o.get("info", {}).get("state") != "active" for o in bridge_links(current_objects)):
                raise RuntimeError("physical bridge changed during playback")
            assignment_one([o for o in current_objects if o not in bridge_links(current_objects)], target, destinations.pid, source_pid, native.pid, token)
            identity = device.stat()
            if [identity.st_dev, identity.st_ino, identity.st_rdev] != before["pcm_identity"] or owner.snapshot() != before["snapshot"]:
                raise RuntimeError("physical PCM identity or foreign desktop changed")
            status = pcm_path(before["card"]).read_text()
            pointer = alsa.clock(status, server.pid)
            params = pcm_path(before["card"]).with_name("hw_params").read_text()
            alsa.parameters(params, RATE)
            activation = alsa.playback.audio_activation(selected, held)
            observation = {"pointer": pointer, "status": status, "hw_params": params, "activation": activation}
            observations.append(observation)
            owner.save(root / ("clock-%03d.json" % len(observations)), observation)
            time.sleep(.2)
        if len(observations) < 2 or observations[-1]["pointer"] - observations[0]["pointer"] < RATE * 3:
            raise RuntimeError("physical stage queue progress insufficient")
        journal["physical_progress_verified"] = True
    except (OSError, ValueError, RuntimeError, KeyError, TypeError, StopIteration, subprocess.TimeoutExpired) as error:
        journal["error"] = type(error).__name__ + ": " + str(error)
    finally:
        if bridge is not None:
            if bridge.poll() is None:
                bridge.terminate()
                try:
                    bridge.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    bridge.kill()
                    try:
                        bridge.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        pass
            journal["controller_stopped"] = bridge.poll() is not None
        if log is not None:
            log.close()
        try:
            end = time.monotonic() + 2
            while True:
                _, objects = dump()
                private_after = owner.core.filtered(objects)
                if private_after == private_before and pcms_closed(before["card"]):
                    break
                if time.monotonic() >= end:
                    raise RuntimeError("owned physical graph or PCM did not release")
                time.sleep(.05)
            journal["private_graph_restored"] = True
            journal["held_binding_restored"] = held is not None and alsa.playback.binding(selected, native.pid, native_start) == held
            journal["desktop_restored"] = owner.snapshot() == before["snapshot"]
            owner.save(root / "private-after.json", objects)
        except (OSError, ValueError, RuntimeError, KeyError, TypeError, StopIteration, subprocess.TimeoutExpired) as error:
            journal["cleanup_error"] = str(error)
        journal["clock_checks"] = len(observations)
        if len(observations) > 1:
            journal["queue_progress"] = observations[-1]["pointer"] - observations[0]["pointer"]
        owner.save(root / "journal.json", journal)
    if journal["error"] is not None or not all(journal.get(k) for k in ("physical_progress_verified", "controller_stopped", "private_graph_restored", "held_binding_restored", "desktop_restored")):
        raise RuntimeError("physical stage failed; inspect retained journal before further USB exchanges")
    return True
