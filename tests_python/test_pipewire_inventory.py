import importlib.util
import os
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("pipewire_inventory", ROOT / "tools/pipewire_inventory.py")
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)


def client(ident=5, serial=70, pid=2001):
    return {"id": ident, "type": adapter.CLIENT, "info": {"props": {
        "object.serial": serial, "application.process.id": pid,
        "application.name": "Native Game", "application.process.binary": "native-game"}}}


def node(ident=20, serial=900, client_id=5, state="running"):
    return {"id": ident, "type": adapter.NODE, "info": {"state": state, "props": {
        "object.serial": serial, "client.id": client_id, "media.class": "Stream/Output/Audio"}}}


class DiscoveryTests(unittest.TestCase):
    def test_metadata_ids_remain_distinct_and_wire_aliases_are_explicit(self):
        objects = [client(), node(), node(21, 901)]
        snapshot = adapter.observe(objects, 7)
        app = snapshot["applications"][0]
        self.assertEqual(app["application_id"], "pw-client:70:pid:2001")
        self.assertEqual(app["process_pid"], 2001)
        self.assertEqual(app["streams"][0]["node_id"], 20)
        self.assertEqual(adapter.project(snapshot, {})["applications"], [])
        projected = adapter.project(snapshot, {"pw-client:70:pid:2001": "game"})
        self.assertEqual(projected["applications"][0]["advertised_name"], "game")
        self.assertEqual(len(projected["applications"][0]["streams"]), 2)
        self.assertNotIn("Native Game", adapter.to_tsv(projected))
        planner = os.environ.get("BEACN_TEST_PLANNER_BIN")
        if planner:
            import subprocess
            result = subprocess.run([planner, "plan-snapshot", "7",
                                     "0e0000ac040167616d650500766f69636500"],
                                    input=adapter.to_tsv(projected).encode("ascii"),
                                    capture_output=True, check=True)
            self.assertIn(b"Proposed(", result.stdout)
            self.assertIn(b"OpaqueAssignment", result.stdout)

    def test_client_serial_and_pid_avoid_merging_same_named_processes(self):
        snapshot = adapter.observe([client(), node(), client(6, 71, 2001), node(22, 902, 6)], 8)
        inventory = adapter.project(snapshot, {"pw-client:70:pid:2001": "game",
                                              "pw-client:71:pid:2001": "game"})
        self.assertEqual(len(inventory["applications"]), 2)
        planner = os.environ.get("BEACN_TEST_PLANNER_BIN")
        if planner:
            import subprocess
            result = subprocess.run([planner, "plan-snapshot", "8",
                                     "0e0000ac040167616d650500766f69636500"],
                                    input=adapter.to_tsv(inventory).encode("ascii"),
                                    capture_output=True, check=True)
            self.assertIn(b"AmbiguousName", result.stdout)
            self.assertNotIn(b"Proposed(", result.stdout)

    def test_incomplete_playback_identity_blocks_projection_not_empty_inventory(self):
        objects = [client(), node()]
        del objects[0]["info"]["props"]["application.process.id"]
        snapshot = adapter.observe(objects, 9)
        self.assertEqual(snapshot["playback_stream_count"], 1)
        self.assertFalse(snapshot["planner_ready"])
        self.assertEqual(len(snapshot["unresolved_streams"]), 1)
        with self.assertRaises(adapter.SnapshotError):
            adapter.project(snapshot, {})

    def test_unknown_state_is_stale_and_nonplayback_nodes_are_ignored(self):
        output = node(state="error")
        capture = node(21, 901)
        capture["info"]["props"]["media.class"] = "Stream/Input/Audio"
        snapshot = adapter.observe([client(), output, capture], 1)
        self.assertEqual(snapshot["playback_stream_count"], 1)
        self.assertEqual(snapshot["applications"][0]["streams"][0]["planner_state"], "stale")

    def test_malformed_duplicate_metadata_and_invalid_aliases_are_rejected(self):
        for raw in [b"{}", b'[{"id":1,"id":2}]', b'[{"id":true}]', b"NaN", b'[{"id":' + b"1" * 5000 + b"}]"]:
            with self.assertRaises(adapter.SnapshotError):
                adapter.decode_dump(raw)
        with self.assertRaises(adapter.SnapshotError):
            adapter.uint("1" * 100)
        with self.assertRaises(adapter.SnapshotError):
            adapter.observe([client(), node(), node(21, 900)], 1)
        with self.assertRaises(adapter.SnapshotError):
            adapter.observe([client(), client(6, 70, 2002), node()], 1)
        for values in [["pw-client:70:pid:2001=gáme"], ["pw-client:70:pid:2001="],
                       ["pw-client:70:pid:2001=game", "pw-client:70:pid:2001=game"]]:
            with self.assertRaises(adapter.SnapshotError):
                adapter.binding_map(values)
        with self.assertRaises(adapter.SnapshotError):
            adapter.project(adapter.observe([], 1), {"pw-client:70:pid:2001": "game"})

    def test_missing_session_is_unavailable_while_valid_empty_dump_is_available(self):
        with patch.object(sys, "platform", "linux"), patch.dict(os.environ, {"XDG_RUNTIME_DIR": ""}), \
                patch.object(adapter.subprocess, "Popen") as spawn:
            with self.assertRaises(adapter.Unavailable):
                adapter.collect_live()
            spawn.assert_not_called()
        snapshot = adapter.observe(adapter.decode_dump(b"[]"), 1)
        self.assertEqual(snapshot["status"], "available")
        self.assertTrue(snapshot["planner_ready"])
        self.assertEqual(snapshot["playback_stream_count"], 0)


if __name__ == "__main__":
    unittest.main()
