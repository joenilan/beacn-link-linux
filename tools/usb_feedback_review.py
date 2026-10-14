#!/usr/bin/env python3
"""Bounded offline review of owned aggregate bpftrace output; never device I/O."""
import json

MAX_BYTES = 2 * 1024 * 1024
SHAPES = {"@completions": 3, "@packets": 3, "@empty_packets": 2,
          "@successful_bytes": 2, "@unsupported_urbs": 2, "@invalid_lengths": 2,
          "@sync_callbacks": 1, "@requested_lengths": 3, "@invalid_requests": 1,
          "@sync_configuration": 6, "@invalid_configuration": 0, "@window_limit": 0,
          "@device_clock_rates": 1, "@clock_unknown": 0}


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate aggregate JSON key")
        result[key] = value
    return result


def review(raw, *, max_packets=64):
    if type(max_packets) is not int or max_packets not in (8, 64):
        raise ValueError("unknown observation packet bound")
    if len(raw) > MAX_BYTES:
        raise ValueError("aggregate output exceeds bound")
    text = raw.decode("utf-8", errors="strict")
    maps = {}
    attached = None
    warnings = sum("WARNING:" in line for line in text.splitlines())
    if any("ERROR:" in line for line in text.splitlines()):
        raise ValueError("tracer diagnostic error; coverage unknown")
    for line in text.splitlines():
        if not line.startswith("{"):
            continue  # Compiler text is retained separately; never per-event evidence.
        value = json.loads(line, object_pairs_hook=unique)
        if not isinstance(value, dict):
            raise ValueError("invalid aggregate record")
        kind = value.get("type")
        if kind is None:
            continue  # The known audio child's own final JSON journal.
        if kind == "attached_probes":
            count = value.get("data", {}).get("probes")
            if attached is not None or type(count) is not int or count not in (5, 7):
                raise ValueError("unexpected probe attachment record")
            attached = count
            continue
        if kind != "map" or type(value.get("data")) is not dict:
            raise ValueError("unexpected tracer record; coverage unknown")
        for name, data in value["data"].items():
            if name not in SHAPES or name in maps:
                raise ValueError("unknown or repeated aggregate map")
            if SHAPES[name] == 0:
                if type(data) is not int or not 0 <= data <= (1 << 63) - 1:
                    raise ValueError("invalid scalar aggregate")
                maps[name] = {(): data}
                continue
            if type(data) is not dict or len(data) > 4096:
                raise ValueError("invalid aggregate map")
            parsed = {}
            for key, count in data.items():
                parts = key.split(",")
                if len(parts) != SHAPES[name] or any(not p.isascii() or not p.isdecimal() for p in parts):
                    raise ValueError("invalid aggregate dimensions")
                dimensions = tuple(int(p) for p in parts)
                if type(count) is not int or not 0 <= count <= (1 << 63) - 1 or any(n > 192000 for n in dimensions):
                    raise ValueError("invalid aggregate value")
                if dimensions in parsed:
                    raise ValueError("reused aggregate dimensions")
                if name == "@sync_configuration":
                    for rate, packet, maximum in (dimensions[:3], dimensions[3:]):
                        if rate not in (48000, 96000) or not 0 < packet <= 416 or packet % 32 or maximum != 416:
                            raise ValueError("unexpected endpoint configuration")
                elif name == "@device_clock_rates":
                    if dimensions[0] not in (48000, 96000):
                        raise ValueError("unexpected device-reported clock rate")
                elif name == "@sync_callbacks":
                    if dimensions[0] > 90:
                        raise ValueError("observation time exceeds bound")
                elif name in ("@requested_lengths", "@invalid_requests"):
                    if dimensions[0] not in (2, 130):
                        raise ValueError("unexpected endpoint")
                    if name == "@requested_lengths" and (dimensions[1] > 6 or not 0 < dimensions[2] <= 416 or dimensions[2] % 32):
                        raise ValueError("unexpected requested length/status")
                elif dimensions[0] > 90 or dimensions[1] not in (2, 130) or (len(dimensions) == 3 and dimensions[2] > 6):
                    raise ValueError("unexpected endpoint/time/status")
                parsed[dimensions] = count
            maps[name] = parsed
    if attached not in (5, 7):
        raise ValueError("missing attachment record")
    endpoints = {}
    for ep in (2, 130):
        completions = [sum(n for key, n in maps.get("@completions", {}).items() if key[1:] == (ep, bucket)) for bucket in range(7)]
        packets = [sum(n for key, n in maps.get("@packets", {}).items() if key[1:] == (ep, bucket)) for bucket in range(7)]
        empty = sum(n for key, n in maps.get("@empty_packets", {}).items() if key[1] == ep)
        successful_bytes = sum(n for key, n in maps.get("@successful_bytes", {}).items() if key[1] == ep)
        if empty > packets[0] or successful_bytes > (packets[0] - empty) * 416:
            raise ValueError("inconsistent successful packet totals")
        if sum(packets) > (sum(completions) - completions[5]) * max_packets:
            raise ValueError("packet totals exceed bounded callbacks")
        if "@requested_lengths" in maps or "@invalid_requests" in maps:
            requested = sum(n for k, n in maps.get("@requested_lengths", {}).items() if k[0] == ep)
            invalid = sum(n for k, n in maps.get("@invalid_requests", {}).items() if k[0] == ep)
            if requested + invalid != sum(packets):
                raise ValueError("requested geometry does not account for all packet observations")
        endpoints[str(ep)] = {"completion_buckets": completions, "packet_buckets": packets,
                              "empty_successful_packets": empty, "successful_bytes": successful_bytes}
    flags = {name: sum(maps.get(name, {}).values()) for name in
             ("@unsupported_urbs", "@invalid_lengths", "@invalid_requests", "@invalid_configuration", "@window_limit", "@clock_unknown")}
    sync = sum(maps.get("@sync_callbacks", {}).values())
    if sync > sum(endpoints["130"]["completion_buckets"]) - endpoints["130"]["completion_buckets"][5]:
        raise ValueError("sync callbacks exceed noncancelled incoming completions")
    if "@sync_configuration" in maps or "@invalid_configuration" in maps:
        if sum(maps.get("@sync_configuration", {}).values()) + flags["@invalid_configuration"] != sync:
            raise ValueError("configuration samples do not account for sync callbacks")
    return {"attached_probes": attached, "compiler_warnings": warnings, "flags": flags,
            "endpoints": endpoints, "sync_callbacks": sync,
            "has_request_geometry": "@requested_lengths" in maps,
            "has_sync_configuration": "@sync_configuration" in maps,
            "device_clock_rates": [{"rate": k[0], "count": n} for k, n in sorted(maps.get("@device_clock_rates", {}).items())],
            "requested_lengths": [{"endpoint": k[0], "status_bucket": k[1], "requested_bytes": k[2], "count": n}
                                  for k, n in sorted(maps.get("@requested_lengths", {}).items())],
            "sync_configurations": [{"source_rate": k[0], "source_packet": k[1], "source_maximum": k[2],
                                     "sink_rate": k[3], "sink_packet": k[4], "sink_maximum": k[5], "count": n}
                                    for k, n in sorted(maps.get("@sync_configuration", {}).items())],
            "needs_review": warnings > 0 or any(flags.values())}
