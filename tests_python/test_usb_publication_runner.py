import contextlib
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import MagicMock, patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("publication_runner", ROOT / "tests_native/usb_publication.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def objects():
    return [{"id": 4, "type": "PipeWire:Interface:Client", "info": {"props": {
        "object.serial": 44, "application.process.id": 123}}},
        {"id": 6, "type": "PipeWire:Interface:Node", "info": {"state": "suspended", "props": {
            "object.serial": 66, "client.id": 4, "application.process.id": 123,
            "media.class": "Stream/Output/Audio", "node.name": "owned-source"}}}]


class PublicationRunnerTests(unittest.TestCase):
    def test_only_exact_owned_process_and_node_receive_the_explicit_wire_binding(self):
        data = runner.owned_inventory(objects(), 123, "owned-source", 1)
        self.assertEqual(data, {"snapshot_id": 1, "applications": [{"application_id": "pw-client:44:pid:123",
            "advertised_name": runner.WIRE_NAME, "streams": [{"stream_id": "pw-node:66", "state": "live"}]}]})
        for pid, name in ((124, "owned-source"), (123, "foreign-source")):
            with self.assertRaises(RuntimeError): runner.owned_inventory(objects(), pid, name, 1)

    def test_links_stale_streams_or_unresolved_identities_block_native_publication(self):
        variants = []
        data = objects(); data.append({"id": 8, "type": "PipeWire:Interface:Link"}); variants.append(data)
        data = objects(); data[1]["info"]["state"] = "error"; variants.append(data)
        data = objects(); data[1]["info"]["props"].pop("object.serial"); variants.append(data)
        data = objects(); data[1]["info"]["props"]["client.id"] = 9; variants.append(data)
        for data in variants:
            with self.assertRaises((RuntimeError, runner.base.inventory.SnapshotError)):
                runner.owned_inventory(data, 123, "owned-source", 1)

    def test_private_cleanup_targets_only_retained_process_and_bounds_unknown_exit(self):
        process = MagicMock()
        process.poll.return_value = None
        process.wait.side_effect = subprocess.TimeoutExpired("owned", 2)
        self.assertFalse(runner.stop_owned(process))
        process.terminate.assert_called_once_with(); process.kill.assert_called_once_with()
        self.assertEqual([call.kwargs["timeout"] for call in process.wait.call_args_list], [2, 2])
        process = MagicMock(); process.poll.return_value = 0
        self.assertTrue(runner.stop_owned(process)); process.terminate.assert_not_called()

    def test_missing_authority_stale_binary_and_reused_attempt_launch_nothing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve(); binary = root / "fake-binary"; binary.write_bytes(b"never executed")
            baseline = {"time": time.time(), "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
            path = root / "baseline.json"; path.write_text(json.dumps(baseline))
            argv = ["test", "run", "--evidence-dir", str(root), "--executor", str(binary)]
            extra = ["--authorize-publication", "--sysfs-device", str(root), "--expected-cache", str(binary)]
            for kind in ("authority", "age", "binary", "reuse"):
                data = copy.deepcopy(baseline)
                if kind == "age": data["time"] -= 601
                if kind == "binary": data["binary_sha256"] = "changed"
                if kind == "reuse": (root / "run-started.json").write_text("{}")
                path.write_text(json.dumps(data))
                with patch.object(sys, "argv", argv + ([] if kind == "authority" else extra)), \
                        patch.object(runner.sys, "platform", "linux"), patch.object(runner.subprocess, "Popen") as child, \
                        contextlib.redirect_stderr(io.StringIO()):
                    with self.assertRaises(SystemExit): runner.main()
                    child.assert_not_called()

    def test_comparison_refuses_unconfirmed_private_process_exit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / "handshake.json").write_text(json.dumps({"owned_processes_stopped": [False]}))
            argv = ["test", "compare", "--evidence-dir", str(root), "--executor", "unused"]
            with patch.object(sys, "argv", argv), patch.object(runner.base, "main") as shared, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit): runner.main()
                shared.assert_not_called()

    def test_metadata_stages_use_existing_snapshot_owner_and_restore_arguments(self):
        for stage in ("prepare", "compare"):
            argv = ["test", stage, "--evidence-dir", "not-existing-test-path", "--executor", "unused"]
            with patch.object(sys, "argv", argv), patch.object(runner.base, "main", return_value=0) as shared:
                self.assertEqual(runner.main(), 0); shared.assert_called_once_with(); self.assertIs(sys.argv, argv)


if __name__ == "__main__": unittest.main()
