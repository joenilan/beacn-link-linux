import copy
import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import pipewire_graph as graph
import pipewire_inventory as adapter


def objects():
    def obj(ident, kind, props=None, **info):
        if props is not None:
            info["props"] = props
        return {"id": ident, "type": "PipeWire:Interface:" + kind, "info": info}
    result = [
        obj(0, "Core", cookie=123),
        obj(7, "Client", {"object.serial": 70, "application.process.id": 2001}),
        obj(10, "Node", {"object.serial": 100, "media.class": "Stream/Output/Audio",
                         "client.id": 7, "application.process.id": 2001,
                         "media.name": "PRIVATE TITLE MUST NOT EXPORT"}, state="running"),
        obj(20, "Node", {"object.serial": 200, "media.class": "Audio/Sink"}),
    ]
    for ident, node, direction, channel in [(11, 10, "out", "FL"), (12, 10, "out", "FR"),
                                           (21, 20, "in", "FL"), (22, 20, "in", "FR")]:
        result.append(obj(ident, "Port", {"object.serial": ident * 10, "node.id": node,
                          "port.direction": direction, "audio.channel": channel},
                          direction="output" if direction == "out" else "input"))
    for ident, out_port, in_port in [(30, 11, 21), (31, 12, 22)]:
        result.append(obj(ident, "Link", {"object.serial": ident * 10}, **{
            "output-node-id": 10, "output-port-id": out_port,
            "input-node-id": 20, "input-port-id": in_port}))
    return result


class GraphTests(unittest.TestCase):
    def project(self, source=None):
        return graph.project_graph(source if source is not None else objects(), 7,
                                   {"pw-client:70:pid:2001": "game"})

    def test_existing_identities_filtered_metadata_and_deterministic_projection(self):
        inv, observed = self.project()
        self.assertEqual(inv["applications"][0]["application_id"], "pw-client:70:pid:2001")
        self.assertEqual(observed["streams"][0]["stream_id"], "pw-node:100")
        self.assertEqual(observed["core_id"], "pw-core:123")
        self.assertEqual(self.project(list(reversed(objects()))), (inv, observed))
        self.assertNotIn("PRIVATE TITLE", json.dumps((inv, observed)))
        self.assertEqual(len(observed["links"]), 2)

    def test_missing_or_conflicting_metadata_refuses(self):
        cases = []
        source = objects(); source[0]["info"].pop("cookie"); cases.append(source)
        source = objects(); source[2]["info"]["props"].pop("client.id"); cases.append(source)
        source = objects(); source[4]["info"]["props"].pop("audio.channel"); cases.append(source)
        source = objects(); source[4]["info"]["direction"] = "input"; cases.append(source)
        source = objects(); source[4]["info"]["props"]["port.control"] = True; cases.append(source)
        source = objects(); source[8]["info"]["props"]["link.output.port"] = 999; cases.append(source)
        source = objects(); source.pop(6); cases.append(source)
        source = objects(); source[3]["info"]["props"]["object.serial"] = 100; cases.append(source)
        for source in cases:
            with self.subTest(source=source), self.assertRaises(adapter.SnapshotError):
                self.project(source)

    def test_foreign_links_retained_and_tag_does_not_create_receipt(self):
        source = objects(); source[8]["info"]["props"][graph.OWNER_KEY] = "ours"
        inv, observed = self.project(source)
        text = graph.preparation_tsv(inv, observed, observed,
                                    graph.destinations(["1:20:200"]), "ours", [])
        self.assertIn("ours", text)
        self.assertNotIn("\nreceipt\t", text)
        self.assertEqual(len(observed["links"]), 2)

    def test_internal_audio_split_links_remain_foreign_without_application_binding(self):
        source = objects()
        source[2]["info"]["props"]["media.class"] = "Stream/Output/Audio/Internal"
        source[3]["info"]["props"]["media.class"] = "Audio/Sink/Internal"
        inv, observed = graph.project_graph(source, 7, {})
        self.assertEqual(inv["applications"], [])
        self.assertEqual(observed["streams"], [])
        self.assertEqual(len(observed["links"]), 2)
        self.assertTrue(all(link["owner"] is None for link in observed["links"]))
        source.pop(6)
        with self.assertRaises(adapter.SnapshotError):
            graph.project_graph(source, 7, {})

    def test_explicit_destinations_refuse_missing_duplicate_and_unsupported_values(self):
        for values in [["1"], ["1:20"], ["0:20:200"], ["1:0:200"],
                       ["1:20:200", "1:30:300"], ["1:20:200", "2:20:200"]]:
            with self.assertRaises(adapter.SnapshotError):
                graph.destinations(values)

    def test_missing_configuration_refuses_before_any_collection(self):
        with patch.object(graph, "read_objects") as read, contextlib.redirect_stdout(io.StringIO()):
            rc = graph.main(["--prepare", "070000ac040167616d6500"])
        self.assertEqual(rc, 2); read.assert_not_called()

    def test_changed_bound_liveness_refuses_before_executor(self):
        changed = copy.deepcopy(objects()); changed[2]["info"]["state"] = "error"
        with patch.object(graph, "read_objects", side_effect=[objects(), changed]), patch.object(
                graph.subprocess, "run") as run, contextlib.redirect_stdout(io.StringIO()):
            rc = graph.main(["--snapshot-id", "7", "--bind", "pw-client:70:pid:2001=game",
                             "--destination", "1:20:200", "--owner", "ours",
                             "--prepare", "070000ac040167616d6500", "--executor-bin", "built-cli"])
        self.assertEqual(rc, 2); run.assert_not_called()

    def test_actual_rust_bridge_preserves_correct_foreign_route_and_refuses_wrong_serial(self):
        binary = os.environ.get("BEACN_TEST_PLANNER_BIN")
        if not binary:
            self.skipTest("explicit built CLI unavailable")
        inv, observed = self.project()
        def prepare(target):
            text = graph.preparation_tsv(inv, observed, observed, graph.destinations([target]), "ours", [])
            return subprocess.run([binary, "prepare-graph", "7", "070000ac040167616d6500"],
                                  input=text.encode("ascii"), capture_output=True, timeout=5)
        result = prepare("1:20:200")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(b"DRY RUN: 0 guarded operations", result.stdout)
        self.assertEqual(prepare("1:20:999").returncode, 2)

    def test_unavailable_is_separate_from_empty_complete_graph(self):
        with patch.object(graph, "read_objects", side_effect=adapter.Unavailable("no socket")), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(graph.main([]), 3)
        empty = [o for o in objects() if o["type"].endswith(":Core")]
        inv, observed = graph.project_graph(empty, 7, {})
        self.assertEqual(inv["applications"], [])
        self.assertEqual(observed["nodes"], [])


if __name__ == "__main__":
    unittest.main()
