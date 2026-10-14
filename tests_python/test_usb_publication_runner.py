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
    def test_source_exit_injection_cannot_enter_hardware_or_default_modes(self):
        for extra in ([], ['--application-mpv'], ['--application-mpv','--cycles','5'], ['--cycles','5','--private-routing']):
            argv=['test','run','--probe-source-exit','--evidence-dir','unused','--executor','unused',*extra]
            with patch.object(sys,'argv',argv),patch.object(runner.subprocess,'Popen') as child,contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):runner.main()
                child.assert_not_called()

    def test_selected_pair_requires_its_authority_before_any_child(self):
        for extra in (['--studio-pair','2'], ['--studio-pair','2','--authorize-studio-first-pair-tone','--audible-first-pair'],
                      ['--studio-pair','2','--authorize-studio-pair-tone'], ['--studio-pair','1','--authorize-studio-pair-tone','--audible-studio-pair']):
            argv=['test','run','--evidence-dir','unused','--executor','unused',*extra]
            with patch.object(sys,'argv',argv),patch.object(runner.subprocess,'Popen') as child,contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):runner.main()
                child.assert_not_called()

    @unittest.skipUnless(sys.platform == 'linux', 'Linux pipe supervisor')
    def test_selected_pair_ipc_accepts_only_the_authorized_pair(self):
        for pair in (2, 3, 4):
            for requested in (pair, 1):
                actions=[]
                def concurrent(action,child):actions.append(action);return True
                def fresh(n):return runner.owned_inventory(objects(),123,'owned-source',n+1)
                code="import sys\nfor n in range(3):\n print('INVENTORY '+str(n),flush=True)\n h=sys.stdin.buffer.readline();sys.stdin.buffer.read(int(h.split(b'\\t')[1]))\n"
                code+="print('PHYSICAL START %d',flush=True);a=sys.stdin.buffer.readline();print('PHYSICAL STOP %d',flush=True);b=sys.stdin.buffer.readline()\n" % (requested, requested)
                code+="assert a==%r and b==%r\n" % (('PHYSICAL ACTIVE %d\n' % pair).encode(), ('PHYSICAL COMPLETE %d\n' % pair).encode())
                with patch.object(runner.base,'snapshot',return_value={}):
                    result=runner.execute_periodic([sys.executable,'-c',code],fresh,2,timeout=2,concurrent=concurrent,pair=pair)
                self.assertTrue(result['child_exited'])
                self.assertEqual('supervisor_error' in result,requested!=pair)
                if requested==pair:self.assertEqual(actions,['start','stop']);self.assertEqual(result['exit'],0)

    @unittest.skipUnless(sys.platform == "linux", "Linux pipe supervisor")
    def test_worker_failure_aborts_owned_audio_before_ending_native_ipc(self):
        actions=[]
        def concurrent(action, child):
            actions.append((action,child.poll() is None))
            return True
        def pulse():
            if actions and actions[-1][0]=="start":raise RuntimeError("owned playback proof lost")
        def fresh(n):return runner.owned_inventory(objects(),123,"owned-source",n+1)
        code="import sys\nfor n in range(3):\n print('INVENTORY '+str(n),flush=True)\n h=sys.stdin.buffer.readline();sys.stdin.buffer.read(int(h.split(b'\\t')[1]))\nprint('PHYSICAL START 1',flush=True);sys.stdin.buffer.readline();sys.stdin.buffer.readline()\n"
        with patch.object(runner.base,"snapshot",return_value={}):
            result=runner.execute_periodic([sys.executable,"-c",code],fresh,2,timeout=2,concurrent=concurrent,pulse=pulse)
        self.assertTrue(result["child_exited"])
        self.assertIn("supervisor_error",result)
        self.assertEqual(actions,[("start",True),("abort",True)])

    def test_continuous_proof_requires_one_bounded_owner_check_and_progress_summary(self):
        proof="native owned PCM checks=12 pointer_progress=288000; metadata only"
        self.assertEqual(runner.concurrent_proof(proof,96000),{"owned_pcm_checks":12,"owned_pcm_control_progress":288000})
        for bad in ("",proof+proof,proof.replace("12","11"),proof.replace("12","4097"),proof.replace("288000","287999")):
            with self.assertRaises(RuntimeError):runner.concurrent_proof(bad,96000)

    def test_continuous_mode_requires_physical_authority_and_progress_budget(self):
        for extra in (["--cycles", "40"], ["--cycles", "8", "--authorize-studio-first-pair-silence", "--private-routing"]):
            argv = ["test", "run", "--evidence-dir", "unused", "--executor", "unused", "--continuous-studio-first-pair", *extra]
            with patch.object(sys, "argv", argv), patch.object(runner.subprocess, "Popen") as child, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):runner.main()
                child.assert_not_called()

    @unittest.skipUnless(sys.platform == "linux", "Linux pipe supervisor")
    def test_continuous_ipc_enforces_exact_once_start_then_stop(self):
        for mode in ("complete", "unauthorized", "stop-first", "duplicate-start", "duplicate-stop", "wrong-pair"):
            actions=[]
            def concurrent(action, child):actions.append(action);return True
            def fresh(n):return runner.owned_inventory(objects(),123,"owned-source",n+1)
            code="import sys\nfor n in range(3):\n print('INVENTORY '+str(n),flush=True)\n h=sys.stdin.buffer.readline();sys.stdin.buffer.read(int(h.split(b'\\t')[1]))\n"
            first="PHYSICAL STOP 1" if mode=="stop-first" else "PHYSICAL START 2" if mode=="wrong-pair" else "PHYSICAL START 1"
            code+="print("+repr(first)+",flush=True);sys.stdin.buffer.readline()\n"
            if mode=="duplicate-start":code+="print('PHYSICAL START 1',flush=True);sys.stdin.buffer.readline()\n"
            code+="print('PHYSICAL STOP 1',flush=True);sys.stdin.buffer.readline()\n"
            if mode=="duplicate-stop":code+="print('PHYSICAL STOP 1',flush=True);sys.stdin.buffer.readline()\n"
            with patch.object(runner.base,"snapshot",return_value={}):
                result=runner.execute_periodic([sys.executable,"-c",code],fresh,2,timeout=2,concurrent=None if mode=="unauthorized" else concurrent)
            self.assertTrue(result["child_exited"])
            self.assertEqual("supervisor_error" in result,mode!="complete")
            if mode=="complete":self.assertEqual(actions,["start","stop"]);self.assertTrue(result["physical_stage_completed"])
            elif mode!="unauthorized":self.assertEqual(actions[-1],"abort")

    def test_audible_stage_requires_exact_authorization_and_excludes_silence(self):
        for extra in (["--audible-first-pair"], ["--authorize-studio-first-pair-tone"], ["--audible-first-pair", "--authorize-studio-first-pair-tone", "--authorize-studio-first-pair-silence"]):
            argv = ["test", "run", "--evidence-dir", "unused", "--executor", "unused", *extra]
            with patch.object(sys, "argv", argv), patch.object(runner.subprocess, "Popen") as child, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):runner.main()
                child.assert_not_called()

    def test_physical_stage_refuses_offline_hold_and_missing_private_mode(self):
        for stage, extra in (("routing-probe", ["--private-routing", "--cycles", "3"]), ("run", ["--cycles", "3"]), ("run", ["--private-routing", "--cycles", "1"]), ("run", ["--private-routing", "--cycles", "3", "--single-publication-hold"])):
            argv = ["test", stage, "--evidence-dir", "unused", "--executor", "unused", "--authorize-studio-first-pair-silence", *extra]
            with patch.object(sys, "argv", argv), patch.object(runner.subprocess, "Popen") as child, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit): runner.main()
                child.assert_not_called()

    @unittest.skipUnless(sys.platform == "linux", "Linux pipe supervisor")
    def test_physical_ipc_requires_authority_prior_inventory_and_exact_once_completion(self):
        for mode in ("complete", "unauthorized", "wrong-pair", "too-early", "duplicate", "incomplete"):
            callbacks = []
            def physical(child):
                callbacks.append(child.pid)
                return mode != "incomplete"
            def fresh(n):
                return runner.owned_inventory(objects(), 123, "owned-source", n + 1)
            code = "import sys\n"
            if mode != "too-early":
                code += "for n in range(3):\n print('INVENTORY '+str(n),flush=True)\n h=sys.stdin.buffer.readline();sys.stdin.buffer.read(int(h.split(b'\\t')[1]))\n"
            code += "print('PHYSICAL 2' if " + repr(mode == "wrong-pair") + " else 'PHYSICAL 1',flush=True)\nreply=sys.stdin.buffer.readline()\n"
            if mode == "duplicate":code += "print('PHYSICAL 1',flush=True);sys.stdin.buffer.readline()\n"
            with patch.object(runner.base, "snapshot", return_value={}):
                result = runner.execute_periodic([sys.executable, "-c", code], fresh, 2, timeout=2,
                                                 physical=None if mode == "unauthorized" else physical)
            self.assertTrue(result["child_exited"])
            self.assertEqual("supervisor_error" in result, mode != "complete")
            self.assertEqual(len(callbacks), int(mode in ("complete", "duplicate", "incomplete")))
            self.assertEqual(bool(result.get("physical_stage_completed")), mode in ("complete", "duplicate"))

    def test_single_publication_hold_refuses_repetition_routing_and_offline_modes(self):
        common = ["--windows-c-string-names", "--initialization-diagnostic", "--observe-rejected-polls", "--observed-startup"]
        for stage, cycles, extra in (("run", "2", []), ("run", "1", ["--private-routing"]), ("source-probe", "1", []), ("run", "1", ["--private-routing", "--authorize-isolated-routing"])):
            argv = ["test", stage, "--evidence-dir", "unused", "--executor", "unused", "--single-publication-hold", "--cycles", cycles, *common, *extra]
            with patch.object(sys, "argv", argv), patch.object(runner.subprocess, "Popen") as child, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit): runner.main()
                child.assert_not_called()

    def test_private_routing_requires_explicit_authorization_and_mode(self):
        for stage, extra in (("run", []), ("routing-probe", []), ("run", ["--authorize-isolated-routing"]), ("source-probe", ["--authorize-isolated-routing", "--windows-c-string-names", "--cycles", "3"]), ("run", ["--windows-c-string-names", "--cycles", "3"])):
            argv = ["test", stage, "--evidence-dir", "unused", "--executor", "unused", "--private-routing", *extra]
            with patch.object(sys, "argv", argv), patch.object(runner.subprocess, "Popen") as child, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit): runner.main()
                child.assert_not_called()

    def test_owned_inventory_retains_strict_default_and_checks_private_destinations(self):
        current = objects()
        current += [{"id": 9, "type": runner.base.inventory.NODE, "info": {"props": {"object.serial": 99, "media.class": "Audio/Sink"}}}]
        current += [{"id": i, "type": "PipeWire:Interface:Link", "info": {"props": {"object.serial": i * 10, "link.output.node": 6, "link.input.node": 9}}} for i in (10, 11)]
        with self.assertRaises(RuntimeError): runner.owned_inventory(current, 123, "owned-source", 1)
        result = runner.owned_inventory(current, 123, "owned-source", 1, {9: 99})
        self.assertEqual(len(result["applications"]), 1)
        with self.assertRaises(RuntimeError): runner.owned_inventory(current, 123, "owned-source", 1, {9: 100})
        with self.assertRaises(RuntimeError): runner.owned_inventory(current[:-1], 123, "owned-source", 1, {9: 99})
        current[-1]["info"]["props"]["link.output.node"] = 1234
        with self.assertRaises(RuntimeError): runner.owned_inventory(current, 123, "owned-source", 1, {9: 99})

    def test_c_string_names_refuse_unselected_or_nonhardware_modes(self):
        for stage, extra in (("run", []), ("run", ["--cycles", "3", "--observed-startup", "--observe-rejected-polls"]), ("source-probe", ["--cycles", "3", "--observed-startup", "--observe-rejected-polls", "--initialization-diagnostic"]), ("prepare-connected", ["--initialization-diagnostic"])):
            argv = ["test", stage, "--evidence-dir", "unused", "--executor", "unused", "--windows-c-string-names", *extra]
            with patch.object(sys, "argv", argv), patch.object(runner.subprocess, "Popen") as child, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit): runner.main()
                child.assert_not_called()

    def test_initialization_diagnostic_requires_explicit_marker_policy(self):
        argv = ["test", "run", "--evidence-dir", "unused", "--executor", "unused", "--cycles", "3", "--observed-startup", "--initialization-diagnostic"]
        with patch.object(sys, "argv", argv), patch.object(runner.subprocess, "Popen") as child, contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit): runner.main()
            child.assert_not_called()

    def test_marker_policy_requires_explicit_periodic_hardware_startup(self):
        for extra in ([], ["--cycles", "3"], ["--observed-startup"], ["--cycles", "3", "--observed-startup"]):
            stage = "source-probe" if len(extra) == 3 else "run"
            argv = ["test", stage, "--evidence-dir", "unused", "--executor", "unused", "--observe-rejected-polls", *extra]
            with patch.object(sys, "argv", argv), patch.object(runner.subprocess, "Popen") as child, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit): runner.main()
                child.assert_not_called()

    def test_missing_completion_journal_blocks_both_comparison_modes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / "run-started.json").write_text("{}")
            for stage in ("compare", "compare-connected"):
                argv = ["test", stage, "--evidence-dir", str(root), "--executor", "unused"]
                with patch.object(sys, "argv", argv), patch.object(runner.base, "main") as shared, contextlib.redirect_stderr(io.StringIO()):
                    with self.assertRaises(SystemExit): runner.main()
                    shared.assert_not_called()

    def test_observed_startup_is_refused_for_default_and_source_only_modes(self):
        for stage, cycles in (("run", "0"), ("source-probe", "3"), ("prepare", "3")):
            argv = ["test", stage, "--evidence-dir", "unused", "--executor", "unused", "--observed-startup", "--cycles", cycles]
            with patch.object(sys, "argv", argv), patch.object(runner.subprocess, "Popen") as child, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit): runner.main()
                child.assert_not_called()

    def test_periodic_ipc_encoding_is_bounded_and_preserves_generation(self):
        inventory = runner.owned_inventory(objects(), 123, "owned-source", 7)
        message = runner.inventory_message(inventory)
        header, text = message.split(b"\n", 1)
        self.assertEqual(header, b"7\t" + str(len(text)).encode())
        inventory["applications"][0]["streams"] *= 1000
        with self.assertRaises(RuntimeError): runner.inventory_message(inventory)

    @unittest.skipUnless(sys.platform == "linux", "Linux pipe supervisor")
    def test_periodic_supervisor_services_fresh_requests_and_stops_bad_sequence(self):
        for bad in (False, True):
            requests = []
            def fresh(n):
                requests.append(n)
                return runner.owned_inventory(objects(), 123, "owned-source", n + 1)
            code = "import sys;print('INVENTORY 0',flush=True);h=sys.stdin.buffer.readline();n=int(h.split(b'\\t')[1]);sys.stdin.buffer.read(n);print('INVENTORY 2' if " + repr(bad) + " else 'done',flush=True)"
            with patch.object(runner.base, "snapshot", return_value={}), patch.object(runner, "WIRE_NAME", "test"):
                result = runner.execute_periodic([sys.executable, "-c", code], fresh, 1, timeout=2)
            self.assertTrue(result["child_exited"])
            self.assertEqual(requests, [0])
            self.assertEqual("supervisor_error" in result, bad)

    @unittest.skipUnless(sys.platform == "linux", "Linux pipe supervisor")
    def test_periodic_supervisor_kills_only_its_child_on_timeout_or_inventory_failure(self):
        for failure in (False, True):
            def fresh(n): raise RuntimeError("owned source disappeared")
            code = "import time;print('INVENTORY 0',flush=True);time.sleep(10)" if failure else "import time;time.sleep(10)"
            with patch.object(runner.base, "snapshot", return_value={}):
                result = runner.execute_periodic([sys.executable, "-c", code], fresh, 1, timeout=0.2)
            self.assertTrue(result["child_exited"])
            self.assertIn("supervisor_error", result)
            self.assertEqual(result["timed_out"], not failure)

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
        for stage in ("prepare", "compare", "prepare-connected", "compare-connected"):
            argv = ["test", stage, "--evidence-dir", "not-existing-test-path", "--executor", "unused"]
            with patch.object(sys, "argv", argv), patch.object(runner.base, "main", return_value=0) as shared:
                self.assertEqual(runner.main(), 0); shared.assert_called_once_with(); self.assertIs(sys.argv, argv)


if __name__ == "__main__": unittest.main()
