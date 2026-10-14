import sys
import signal
import subprocess
import tempfile
import time
import math
import struct
from pathlib import Path
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tests_native"))
import studio_alsa_playback as alsa


class StudioAlsaTests(unittest.TestCase):
    def test_completed_setup_requires_full_progress_and_the_retained_owner(self):
        done = "state: SETUP\nowner_pid   : 123\nhw_ptr      : 384000\n"
        self.assertTrue(alsa.completed_pcm(done, 123, 384000))
        for raw in (done.replace("384000", "383999"), done.replace("123", "456"),
                    done.replace("SETUP", "PREPARED"), done.replace("SETUP", "XRUN"),
                    done.replace("hw_ptr", "appl_ptr"), done + "x" * 16384, "closed"):
            with self.subTest(raw=raw[:70]):
                self.assertFalse(alsa.completed_pcm(raw, 123, 384000))

    def test_audible_authorization_is_separate_and_exact(self):
        alsa.check_authorization(True, False, False)
        alsa.check_authorization(False, True, True)
        for args in ((False, False, False), (True, False, True), (False, True, False), (True, True, True)):
            with self.subTest(args=args), self.assertRaises(ValueError):
                alsa.check_authorization(*args)

    def test_quiet_tones_are_finite_first_pair_only_with_silent_lead_and_tail(self):
        raw = alsa.pcm(96000, audible=True)
        self.assertEqual(len(raw), 96000 * 8 * 4 * 4)
        self.assertFalse(any(raw[:96000 * 8 * 4]))
        self.assertFalse(any(raw[(3 * 96000 + 33600) * 8 * 4:]))
        heard = False
        for second in (1, 2, 3):
            for i in range(0, 33600, 137):
                frame = struct.unpack_from("<8f", raw, (second * 96000 + i) * 8 * 4)
                self.assertEqual(frame[0], frame[1])
                self.assertTrue(all(v == 0 for v in frame[2:]))
                self.assertLessEqual(abs(frame[0]), 10 ** (-24 / 20) + 1e-8)
                heard = heard or frame[0] != 0
        self.assertTrue(heard)

    def test_native_mode_refuses_virtual_or_unknown_environments(self):
        for code, output in ((0, "vmware"), (0, "none"), (1, ""), (2, "none")):
            with self.subTest(code=code, output=output), patch.object(
                    alsa.subprocess, "run", return_value=subprocess.CompletedProcess([], code, output, "")):
                with self.assertRaises(RuntimeError):
                    alsa.require_native_environment()
        with patch.object(alsa.subprocess, "run", return_value=subprocess.CompletedProcess([], 1, "none\n", "")):
            alsa.require_native_environment()

    def test_native_card_accepts_raw_card_with_an_unrelated_physical_device(self):
        def device(ident, serial, card, vid="0x33ae", pid="0x4003"):
            return {"id": ident, "type": "PipeWire:Interface:Device", "info": {"props": {
                "object.serial": serial, "device.vendor.id": vid, "device.product.id": pid,
                "api.alsa.card": card}}}
        selected = device(84, 123, "2")
        builtin = device(68, 124, "0", "0x8086", "0x0000")
        pinned = alsa.native_card_endpoint([builtin, selected], 2)
        self.assertEqual(pinned["device"], {"id": 84, "serial": 123})
        self.assertEqual(pinned["direct_alsa_card"], 2)
        for objects, card in (([builtin], 2), ([selected], 0), ([selected, selected], 2)):
            with self.subTest(card=card, objects=objects), self.assertRaises(RuntimeError):
                alsa.native_card_endpoint(objects, card)
        changed = device(84, 125, "2")
        self.assertNotEqual(alsa.native_card_endpoint([changed], 2), pinned)

    def test_native_card_comparison_allows_no_negotiated_format_exceptions(self):
        before = {"desktop": {"Node": [{"id": 5, "serial": 10, "params": {"Format": []}}]},
                  "state_files": {"default-nodes": "unchanged"}}
        same = alsa.playback.compare_settings(before, before, {"direct_alsa_card": 2})
        self.assertTrue(same["settings_routes_exact_equal"])
        after = {"desktop": {"Node": [{"id": 5, "serial": 10, "params": {"Format": [{"channels": 2}]}}]},
                 "state_files": before["state_files"]}
        changed = alsa.playback.compare_settings(before, after, {"direct_alsa_card": 2})
        self.assertFalse(changed["settings_routes_exact_equal"])
        self.assertEqual(changed["negotiated_format_cache_changes"], [])

    @unittest.skipUnless(sys.platform == "linux", "POSIX signal delivery")
    def test_termination_signal_leaves_owned_child_cleanup_reachable(self):
        native = Path(__file__).resolve().parents[1] / "tests_native"
        with tempfile.TemporaryDirectory() as directory:
            ready = Path(directory) / "ready"
            script = r'''
import pathlib,signal,subprocess,sys,time
sys.path.insert(0,sys.argv[1]);import studio_alsa_playback as alsa
cancellation=alsa.Cancellation();signal.signal(signal.SIGTERM,cancellation.request)
child=subprocess.Popen([sys.executable,'-c','import time;time.sleep(20)'])
pathlib.Path(sys.argv[2]).write_text('ready')
try:
    while True:cancellation.check();time.sleep(.02)
except RuntimeError:
    pass
finally:
    child.terminate();child.wait(timeout=2)
    time.sleep(.15)  # A second signal during cleanup must not throw.
assert child.poll() is not None
print('owned child exited')
'''
            parent = subprocess.Popen([sys.executable, "-c", script, str(native), str(ready)],
                                      stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                deadline = time.monotonic() + 3
                while not ready.exists() and parent.poll() is None and time.monotonic() < deadline:
                    time.sleep(.02)
                self.assertTrue(ready.exists(), "signal fixture did not become ready")
                parent.send_signal(signal.SIGTERM)
                time.sleep(.05)
                parent.send_signal(signal.SIGTERM)
                out, err = parent.communicate(timeout=3)
                self.assertEqual(parent.returncode, 0, err)
                self.assertIn("owned child exited", out)
            finally:
                if parent.poll() is None:
                    parent.kill();parent.wait(timeout=3)

    def test_direct_probe_authors_only_finite_eight_channel_silence(self):
        raw = alsa.pcm()
        self.assertEqual(len(raw), 48000 * 8 * 4 * 4)
        self.assertFalse(any(raw))

    def test_refuses_rate_format_channel_conversion_and_missing_params(self):
        good = "format: FLOAT_LE\nchannels: 8\nrate: 48000 (48000/1)\n"
        alsa.parameters(good)
        for raw in (good.replace("48000", "96000"), good.replace("FLOAT_LE", "S32_LE"),
                    good.replace("channels: 8", "channels: 2"), "closed"):
            with self.subTest(raw=raw), self.assertRaises(RuntimeError):
                alsa.parameters(raw)

    def test_measured_clock_mode_authors_silence_at_the_selected_rate(self):
        raw = alsa.pcm(96000)
        self.assertEqual(len(raw), 96000 * 8 * 4 * 4)
        self.assertFalse(any(raw))
        good = "format: FLOAT_LE\nchannels: 8\nrate: 96000 (96000/1)\n"
        alsa.parameters(good, 96000)
        with self.assertRaises(RuntimeError):
            alsa.parameters(good.replace("96000", "48000"), 96000)
        with self.assertRaises(RuntimeError):
            alsa.parameters(good, 48000)

    def test_unsupported_or_noninteger_rates_are_refused_before_pcm_allocation(self):
        for rate in (0, -1, 44100, 192000, True, 96000.0, "96000"):
            with self.subTest(rate=rate):
                with self.assertRaises(ValueError):
                    alsa.pcm(rate)
                with self.assertRaises(ValueError):
                    alsa.parameters("", rate)

    def test_held_vendor_identity_requires_both_positive_exact_arguments(self):
        alsa.check_held_arguments(None, None)
        alsa.check_held_arguments(123, "456")
        for pid, start in ((None, "456"), (123, None), (0, "456"), (True, "456"),
                           (123, "0"), (123, "456x"), (123, "1" * 21)):
            with self.subTest(pid=pid, start=start), self.assertRaises(ValueError):
                alsa.check_held_arguments(pid, start)

    def test_clock_refuses_foreign_owner_stopped_missing_or_oversized_metadata(self):
        good = "state: RUNNING\nowner_pid   : 123\nhw_ptr      : 298\n"
        self.assertEqual(alsa.clock(good, 123), 298)
        for raw in (good.replace("123", "456"), good.replace("RUNNING", "SETUP"),
                    good.replace("hw_ptr", "appl_ptr"), good + "x" * 16384):
            with self.subTest(raw=raw[:70]), self.assertRaises(RuntimeError):
                alsa.clock(raw, 123)

    def test_draining_is_accepted_only_after_the_caller_has_verified_progress(self):
        draining = "state: DRAINING\nowner_pid   : 123\nhw_ptr      : 384000\n"
        with self.assertRaises(RuntimeError):
            alsa.clock(draining, 123)
        self.assertEqual(alsa.clock(draining, 123, allow_draining=True), 384000)
        for raw in (draining.replace("123", "456"), draining.replace("DRAINING", "SETUP"),
                    draining.replace("DRAINING", "PREPARED")):
            with self.subTest(raw=raw), self.assertRaises(RuntimeError):
                alsa.clock(raw, 123, allow_draining=True)


if __name__ == "__main__":
    unittest.main()
