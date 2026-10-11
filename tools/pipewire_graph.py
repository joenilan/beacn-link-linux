#!/usr/bin/env python3
"""Read-only audio graph metadata; optional guarded preparation, never application."""
import argparse
import json
import os
import subprocess
import time

import pipewire_inventory as inventory

CORE = "PipeWire:Interface:Core"
PORT = "PipeWire:Interface:Port"
LINK = "PipeWire:Interface:Link"
OWNER_KEY = "beacn.link.owner"


def label(value):
    if not isinstance(value, str) or not 1 <= len(value) <= 128 or any(
            not 0x20 <= ord(c) <= 0x7e for c in value):
        raise inventory.SnapshotError("invalid graph label")
    return value


def ref(obj):
    ident = inventory.uint(obj.get("id"), (1 << 32) - 1)
    serial = inventory.uint(inventory.properties(obj).get("object.serial"))
    if not ident or not serial:
        raise inventory.SnapshotError("zero graph object identity")
    return {"id": ident, "serial": serial}


def audio_node(obj):
    cls = inventory.properties(obj).get("media.class")
    return obj.get("type") == inventory.NODE and isinstance(cls, str) and (
        cls.startswith("Audio/") or cls in ("Stream/Output/Audio", "Stream/Input/Audio"))


def project_graph(objects, generation, bindings):
    """Reuse adapter identities; include foreign audio links, never infer ownership."""
    snapshot = inventory.observe(objects, generation)
    projected = inventory.project(snapshot, bindings)
    cores = [o for o in objects if o.get("type") == CORE]
    if len(cores) != 1 or not isinstance(cores[0].get("info"), dict):
        raise inventory.SnapshotError("missing or ambiguous native core metadata")
    cookie = inventory.uint(cores[0]["info"].get("cookie"), (1 << 32) - 1)
    nodes = {inventory.uint(o["id"]): ref(o) for o in objects if audio_node(o)}
    if len(nodes) > 8192:
        raise inventory.SnapshotError("too many audio nodes")
    ports = {}
    for obj in objects:
        if obj.get("type") != PORT:
            continue
        props = inventory.properties(obj)
        node_id = inventory.uint(props.get("node.id"), (1 << 32) - 1)
        if node_id not in nodes:
            continue
        direction = props.get("port.direction")
        info = obj.get("info")
        if not isinstance(info, dict):
            raise inventory.SnapshotError("missing port info")
        info_direction = info.get("direction")
        if direction not in ("in", "out") or info_direction != {"in": "input", "out": "output"}[direction]:
            raise inventory.SnapshotError("missing or inconsistent port direction")
        channel = props.get("audio.channel")
        label(channel)
        if props.get("port.control") not in (None, False, "false"):
            raise inventory.SnapshotError("unsupported audio control port")
        ports[inventory.uint(obj["id"])] = {"object": ref(obj), "node": nodes[node_id],
                            "direction": direction,
                            "channel": channel if channel in ("FL", "FR") else "other"}
    links = []
    for obj in objects:
        if obj.get("type") != LINK:
            continue
        info, props = obj.get("info"), inventory.properties(obj)
        if not isinstance(info, dict):
            raise inventory.SnapshotError("missing link info")
        fields = [("output-node-id", "link.output.node"), ("output-port-id", "link.output.port"),
                  ("input-node-id", "link.input.node"), ("input-port-id", "link.input.port")]
        values = []
        for key, prop in fields:
            value = inventory.uint(info.get(key), (1 << 32) - 1)
            if prop in props and inventory.uint(props[prop], (1 << 32) - 1) != value:
                raise inventory.SnapshotError("inconsistent link endpoint metadata")
            values.append(value)
        out_node, out_port, in_node, in_port = values
        if out_node not in nodes and in_node not in nodes and out_port not in ports and in_port not in ports:
            continue
        if out_node not in nodes or in_node not in nodes or out_port not in ports or in_port not in ports:
            raise inventory.SnapshotError("audio link endpoint absent from projection")
        output, target = ports[out_port], ports[in_port]
        if output["node"] != nodes[out_node] or target["node"] != nodes[in_node] or (
                output["direction"], target["direction"]) != ("out", "in"):
            raise inventory.SnapshotError("invalid audio link endpoint/direction")
        owner = label(props[OWNER_KEY]) if OWNER_KEY in props else None
        links.append({"object": ref(obj), "output": output["object"],
                      "input": target["object"], "owner": owner})
    streams = [{"application_id": app["application_id"], "stream_id": stream["stream_id"],
                "node": {"id": stream["node_id"], "serial": stream["object_serial"]}}
               for app in snapshot["applications"] for stream in app["streams"]]
    if len(ports) > 16384 or len(links) > 16384 or len(streams) > 4096:
        raise inventory.SnapshotError("graph projection limit exceeded")
    objects_ref = list(nodes.values()) + [p["object"] for p in ports.values()] + [l["object"] for l in links]
    if len({o["id"] for o in objects_ref}) != len(objects_ref) or len(
            {o["serial"] for o in objects_ref}) != len(objects_ref):
        raise inventory.SnapshotError("duplicate graph object identity")
    graph = {"generation": generation, "core_id": f"pw-core:{cookie}",
             "nodes": sorted(nodes.values(), key=lambda o: o["id"]),
             "ports": sorted(ports.values(), key=lambda p: p["object"]["id"]),
             "streams": sorted(streams, key=lambda s: s["stream_id"]),
             "links": sorted(links, key=lambda l: l["object"]["id"])}
    return projected, graph


def explicit_ref(value):
    fields = value.split(":")
    if len(fields) != 2:
        raise inventory.SnapshotError("object reference requires ID:SERIAL")
    result = {"id": inventory.uint(fields[0], (1 << 32) - 1),
              "serial": inventory.uint(fields[1])}
    if not result["id"] or not result["serial"]:
        raise inventory.SnapshotError("zero explicit object identity")
    return result


def destinations(values):
    result, assignments, nodes = [], set(), set()
    for value in values:
        assignment, sep, identity = value.partition(":")
        number = inventory.uint(assignment, 255)
        if not sep:
            raise inventory.SnapshotError("destination requires ASSIGNMENT:ID:SERIAL")
        node = explicit_ref(identity)
        if number not in (1, 2, 3, 4) or number in assignments or (
                node["id"], node["serial"]) in nodes:
            raise inventory.SnapshotError("unsupported or ambiguous explicit destination")
        assignments.add(number)
        nodes.add((node["id"], node["serial"]))
        result.append({"assignment": number, "node": node})
    if len(result) > 4:
        raise inventory.SnapshotError("too many destinations")
    return result


def preparation_tsv(projected, expected, observed, targets, token, receipts):
    if expected["generation"] != observed["generation"] or projected["snapshot_id"] != observed["generation"]:
        raise inventory.SnapshotError("generation mismatch")
    rows = []
    for row in inventory.to_tsv(projected).splitlines():
        rows.append("inventory\t" + row)
    def identity(obj):
        return [str(obj["id"]), str(obj["serial"])]
    for side, graph in (("expected", expected), ("observed", observed)):
        rows.append("\t".join([side, "core", label(graph["core_id"])]))
        for node in graph["nodes"]:
            rows.append("\t".join([side, "node"] + identity(node)))
        for port in graph["ports"]:
            rows.append("\t".join([side, "port"] + identity(port["object"]) + identity(port["node"]) +
                                  [port["direction"], port["channel"]]))
        for stream in graph["streams"]:
            rows.append("\t".join([side, "stream", label(stream["application_id"]),
                                  label(stream["stream_id"])] + identity(stream["node"])))
        for link in graph["links"]:
            rows.append("\t".join([side, "link"] + identity(link["object"]) + identity(link["output"]) +
                                  identity(link["input"]) + [label(link["owner"]) if link["owner"] is not None else ""]))
    for target in targets:
        rows.append("\t".join(["destination", str(target["assignment"])] + identity(target["node"])))
    rows.append("owner\t" + label(token))
    for receipt in receipts:
        rows.append("\t".join(["receipt"] + identity(receipt)))
    result = "\n".join(rows) + "\n"
    if len(result.encode("ascii")) > 8 * 1024 * 1024:
        raise inventory.SnapshotError("preparation TSV exceeds 8 MiB")
    return result


def read_objects(path):
    if path:
        with open(path, "rb") as source:
            raw = source.read(inventory.MAX_DUMP_BYTES + 1)
    else:
        raw = inventory.collect_live()
    return inventory.decode_dump(raw)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dump")
    parser.add_argument("--expected-dump")
    parser.add_argument("--snapshot-id", type=int)
    parser.add_argument("--bind", action="append", default=[])
    parser.add_argument("--destination", action="append", default=[], metavar="ASSIGNMENT:NODE-ID:SERIAL")
    parser.add_argument("--owner")
    parser.add_argument("--receipt", action="append", default=[], metavar="LINK-ID:SERIAL")
    parser.add_argument("--prepare", metavar="DATA-FRAME-HEX")
    parser.add_argument("--executor-bin")
    args = parser.parse_args(argv)
    try:
        generation = inventory.uint(args.snapshot_id if args.snapshot_id is not None else time.time_ns())
        bindings = inventory.binding_map(args.bind)
        targets = destinations(args.destination)
        receipts = [explicit_ref(v) for v in args.receipt]
        if len(receipts) > 16384 or len({(o["id"], o["serial"]) for o in receipts}) != len(receipts):
            raise inventory.SnapshotError("duplicate or excessive receipt")
        if args.expected_dump and (not args.dump or not args.prepare):
            raise inventory.SnapshotError("--expected-dump requires --dump and --prepare")
        if args.prepare and (not args.executor_bin or not targets or not args.owner):
            raise inventory.SnapshotError("preparation requires explicit executable, destinations and owner")
        if args.prepare and args.dump and not args.expected_dump:
            raise inventory.SnapshotError("file preparation requires explicit expected and observed dumps")
        projected, expected = project_graph(read_objects(args.expected_dump if args.dump and args.prepare else args.dump),
                                            generation, bindings)
        if not args.prepare:
            print(json.dumps({"status": "available", "inventory": projected, "graph": expected}, indent=2))
            return 0
        fresh, observed = project_graph(read_objects(args.dump), generation, bindings)
        if fresh != projected:
            raise inventory.SnapshotError("bound inventory changed between observations")
        text = preparation_tsv(fresh, expected, observed, targets, args.owner, receipts)
        result = subprocess.run([os.path.abspath(args.executor_bin), "prepare-graph", str(generation), args.prepare],
                                input=text.encode("ascii"), capture_output=True, timeout=5)
        print(json.dumps({"status": "prepared" if result.returncode == 0 else "refused",
                          "application_enabled": False, "exit_code": result.returncode,
                          "output": result.stdout.decode("utf-8", "replace"),
                          "error": result.stderr.decode("utf-8", "replace")}, indent=2))
        return result.returncode
    except inventory.Unavailable as exc:
        print(json.dumps({"status": "unavailable", "reason": str(exc)}))
        return 3
    except (inventory.SnapshotError, OSError, subprocess.TimeoutExpired) as exc:
        print(json.dumps({"status": "invalid-input", "reason": str(exc)}))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
