import contextlib
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
spec = importlib.util.spec_from_file_location("usb_handshake_runner", ROOT / "tests_native/usb_handshake.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.parent = Path(self.temp.name).resolve()
        self.root = self.parent / "private-evidence"
        self.binary = self.parent / "synthetic-executor"
        self.binary.write_bytes(b"not executable; subprocess is always mocked")
        self.device = self.parent / "synthetic-device"
        self.device.mkdir()
        self.cache = self.parent / "synthetic-cache"
        self.cache.write_bytes(b"private-cache-sentinel")
        self.snapshot = {"desktop": {"Node": []}, "state_files": {
            "stream-properties": {"sha256": "synthetic", "bytes_base64": "YQ==", "mode": 384}}}

    def invoke(self, stage, extra=(), targets=(), after=None, action=None, stalled=False):
        argv = ["runner", stage, "--evidence-dir", str(self.root), "--executor", str(self.binary), *extra]
        child = MagicMock(returncode=0, pid=123)
        child.communicate.return_value = (b"body redacted", b"")
        child.poll.return_value = 0
        if action:
            child.communicate.side_effect = [action, (b"", b"")]
            child.returncode = -9
            child.poll.return_value = -9
        if stalled:
            child.communicate.side_effect = [subprocess.TimeoutExpired("owned", 12), subprocess.TimeoutExpired("owned", 2)]
            child.returncode = None
            child.poll.return_value = None
        with patch.object(sys, "argv", argv), patch.object(runner.sys, "platform", "linux"), \
             patch.object(runner, "targets", return_value=list(targets)), \
             patch.object(runner, "snapshot", return_value=after or self.snapshot), \
             patch.object(runner.subprocess, "Popen", return_value=child) as run, \
             contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            result = runner.main()
        return result, run

    def prepare(self):
        result, run = self.invoke("prepare")
        self.assertEqual(result, 0)
        run.assert_not_called()

    def cache_sample(self, alternate=0):
        return ((1, 2), {"bConfigurationValue": 1}, [{"number": n, "alternate": alternate if n == 3 else 0, "driver": None} for n in range(4)], b"synthetic-descriptor")

    def test_connected_prepare_and_compare_need_no_usb_handoff_or_execution(self):
        with patch.object(runner.usb_cache, "sample", return_value=self.cache_sample()):
            result, run = self.invoke("prepare-connected", ("--sysfs-device", str(self.device)), [self.device])
            self.assertEqual(result, 0); run.assert_not_called()
            data = json.loads((self.root / "baseline.json").read_text())
            self.assertEqual(data["scope"], "connected")
            self.assertEqual(data["snapshot"], self.snapshot)
            result, run = self.invoke("compare-connected", targets=[self.device])
            self.assertEqual(result, 0); run.assert_not_called()
            report = json.loads((self.root / "connected-comparison.json").read_text())
            self.assertTrue(report["connected_binding_equal"])
            self.assertFalse(report["guest_target_absent"])

    def test_connected_stages_refuse_missing_ambiguous_owned_or_replaced_target(self):
        with self.assertRaises(SystemExit): self.invoke("prepare-connected", targets=[self.device])
        for found, sample in (([], self.cache_sample()), ([self.device, self.device], self.cache_sample()), ([self.device], self.cache_sample(1))):
            with patch.object(runner.usb_cache, "sample", return_value=sample):
                with self.assertRaises(RuntimeError): self.invoke("prepare-connected", ("--sysfs-device", str(self.device)), found)
        with patch.object(runner.usb_cache, "sample", return_value=self.cache_sample()):
            self.invoke("prepare-connected", ("--sysfs-device", str(self.device)), [self.device])
        changed = self.cache_sample(); changed = ((1, 3), *changed[1:])
        with patch.object(runner.usb_cache, "sample", return_value=changed):
            with self.assertRaises(SystemExit): self.invoke("compare-connected", targets=[self.device])
        with self.assertRaises(RuntimeError): self.invoke("compare-connected", targets=[])
        self.assertFalse((self.root / "connected-comparison.json").exists())

    def test_connected_comparison_preserves_concurrent_work_and_refuses_foreign_scope(self):
        self.prepare()
        with self.assertRaises(SystemExit): self.invoke("compare-connected", targets=[self.device])
        self.root = self.parent / "connected-evidence"
        changed = {"desktop": {"Node": [{"id": 9}]}, "state_files": self.snapshot["state_files"]}
        with patch.object(runner.usb_cache, "sample", return_value=self.cache_sample()):
            self.invoke("prepare-connected", ("--sysfs-device", str(self.device)), [self.device])
            result, run = self.invoke("compare-connected", targets=[self.device], after=changed)
        self.assertEqual(result, 2); run.assert_not_called()
        self.assertEqual(json.loads((self.root / "connected-comparison.json").read_text())["after"], changed)
        self.assertEqual(json.loads((self.root / "baseline.json").read_text())["snapshot"], self.snapshot)

    def test_missing_completion_journal_blocks_comparison(self):
        self.prepare()
        (self.root / "run-started.json").write_text("{}")
        with self.assertRaises(SystemExit): self.invoke("compare")
        self.assertFalse((self.root / "comparison.json").exists())

    def run_args(self):
        return ("--authorize-handshake", "--sysfs-device", str(self.device), "--expected-cache", str(self.cache))

    def test_prepare_saves_complete_state_content_and_does_not_execute_native(self):
        self.prepare()
        data = json.loads((self.root / "baseline.json").read_text())
        self.assertEqual(data["snapshot"], self.snapshot)
        self.assertEqual(data["binary_sha256"], hashlib.sha256(self.binary.read_bytes()).hexdigest())

    def test_missing_authority_ambiguous_selection_and_stale_baseline_refused(self):
        self.prepare()
        for extra, targets in (((), [self.device]), (self.run_args(), []), (self.run_args(), [self.device, self.device])):
            with self.assertRaises(SystemExit): self.invoke("run", extra, targets)
        data = json.loads((self.root / "baseline.json").read_text())
        data["time"] = time.time() - 601
        (self.root / "baseline.json").write_text(json.dumps(data))
        with self.assertRaises(SystemExit): self.invoke("run", self.run_args(), [self.device])
        self.assertFalse((self.root / "run-started.json").exists())

    def test_one_attempt_uses_private_pinned_cache_and_records_completion(self):
        self.prepare()
        result, run = self.invoke("run", self.run_args(), [self.device])
        self.assertEqual(result, 0)
        self.assertEqual((self.root / "expected-cache.bin").read_bytes(), self.cache.read_bytes())
        self.assertEqual(run.call_count, 1)
        self.assertEqual(run.return_value.communicate.call_args.kwargs["timeout"], 12)
        self.assertEqual(run.call_args.kwargs["env"]["LIBUSB_DEBUG"], "0")
        self.assertEqual(run.call_args.args[0][-1], str(self.root / "expected-cache.bin"))
        with self.assertRaises(SystemExit): self.invoke("run", self.run_args(), [self.device])

    def test_timeout_is_unknown_and_journal_prevents_retry(self):
        self.prepare()
        result, run = self.invoke("run", self.run_args(), [self.device], action=subprocess.TimeoutExpired("owned-child", 12))
        self.assertEqual(result, 2)
        self.assertEqual(run.call_count, 1)
        self.assertTrue(json.loads((self.root / "handshake.json").read_text())["timed_out"])
        with self.assertRaises(SystemExit): self.invoke("run", self.run_args(), [self.device])

    def test_comparison_preserves_differences_without_restoring_or_executing(self):
        self.prepare()
        changed = {"desktop": {"Node": [{"id": 9}]}, "state_files": {"stream-properties": None}}
        result, run = self.invoke("compare", after=changed)
        self.assertEqual(result, 2)
        run.assert_not_called()
        self.assertEqual(json.loads((self.root / "comparison.json").read_text())["after"], changed)
        self.assertEqual(json.loads((self.root / "baseline.json").read_text())["snapshot"], self.snapshot)

    def test_changed_executor_and_not_returned_device_refuse_comparison(self):
        self.prepare()
        with self.assertRaises(SystemExit): self.invoke("compare", targets=[self.device])
        self.binary.write_bytes(b"changed")
        with self.assertRaises(SystemExit): self.invoke("run", self.run_args(), [self.device])

    def test_unconfirmed_child_exit_cannot_be_reported_as_successful_cleanup(self):
        self.prepare()
        result, run = self.invoke("run", self.run_args(), [self.device], stalled=True)
        self.assertEqual(result, 2)
        run.return_value.kill.assert_called_once()
        self.assertEqual([c.kwargs["timeout"] for c in run.return_value.communicate.call_args_list], [12, 2])
        self.assertFalse(json.loads((self.root / "handshake.json").read_text())["child_exited"])
        with self.assertRaises(SystemExit): self.invoke("compare")


if __name__ == "__main__":
    unittest.main()
