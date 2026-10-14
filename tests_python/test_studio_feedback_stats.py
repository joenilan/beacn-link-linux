import os
from pathlib import Path
import shlex
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tests_native"))
import studio_feedback_stats as stats


class FeedbackStatsTests(unittest.TestCase):
    def test_refuses_unpinned_identifiers_and_source_injection(self):
        for bus, device in ((0, 1), (1000, 1), (1, 0), (1, 128), (True, 1), (1, False), ("1; exit()", 1)):
            with self.subTest(bus=bus, device=device), self.assertRaises(ValueError):
                stats.probe_program(bus, device)

    def test_generated_observer_reads_only_bounded_metadata_and_aggregates(self):
        program = stats.probe_program(1, 127)
        for forbidden in ("transfer_buffer", "setup_packet", "buf(", "str(", "printf(", "system(", "signal("):
            self.assertNotIn(forbidden, program)
        self.assertIn("number_of_packets", program)
        self.assertIn("actual_length", program)
        self.assertIn("unroll(8)", program)
        self.assertIn("$packets > 8", program)
        self.assertIn("interval:s:30", program)
        self.assertIn("0x33ae", program)
        self.assertIn("0x4003", program)
        self.assertIn("$ep == 2 || $ep == 130", program)

    def test_attachment_check_uses_exact_runtime_load_flags_and_child_context(self):
        program = stats.probe_program(1, 2)
        child = stats.child_command(1000, 1000, "/tmp/home", "/tmp/source", "/tmp/device", "/tmp/evidence")
        run = stats.tracer_command(program, child)
        check = stats.tracer_command(program, child, attachment_check=True)
        self.assertEqual(check[:-1], run)
        self.assertEqual(check[-1], "--dry-run")
        self.assertIn("-k", check)
        self.assertEqual(check[check.index("-c") + 1], child)

    def test_optional_handoff_probes_filter_initialized_pipe_without_payload_reads(self):
        normal = stats.probe_program(1, 2)
        program = stats.probe_program(1, 2, observe_usb_handoff=True)
        extra = program[len(normal):]
        self.assertNotIn("@handoff_stages", normal)
        self.assertEqual(extra.count("kprobe:"), 5)
        self.assertNotIn("$u->ep", extra)  # Not yet initialized at usb_submit_urb entry.
        self.assertIn("($u->pipe >> 15) & 15", extra)
        self.assertIn("($u->pipe >> 30) == 0", extra)
        self.assertIn("kprobe:ehci_urb_enqueue", extra)
        self.assertIn("(struct urb *)arg1", extra)
        for forbidden in ("transfer_buffer;", "transfer_buffer[", "setup_packet", "buf(", "str(", "printf(", "system(", "signal("):
            self.assertNotIn(forbidden, extra)
        self.assertIn("$packets <= 64", extra)
        self.assertEqual(program.count("unroll(8)"), 1)

    def test_failed_collector_exit_is_error_and_partial_audio_outcome_stays_unknown(self):
        failed = stats.observer_outcome(255, False, None)
        self.assertIsNotNone(failed["error"])
        self.assertFalse(failed["audio_started"])
        self.assertIsNone(stats.observer_outcome(255, True, None)["audio_started"])
        completed = stats.observer_outcome(0, True, {"owned_pid": 123, "error": "clock stalled",
                                                   "child_exited": False})
        self.assertIsNone(completed["error"])  # Collector success is independent of playback success.
        self.assertTrue(completed["audio_started"])
        self.assertEqual(completed["playback_error"], "clock stalled")
        self.assertFalse(completed["playback_child_exited"])

    def test_matched_clock_is_explicit_and_preserves_the_nonroot_child(self):
        normal = shlex.split(stats.child_command(1000, 1000, "/tmp/home", "/tmp/source", "/tmp/usb", "/tmp/out"))
        matched = shlex.split(stats.child_command(1000, 1000, "/tmp/home", "/tmp/source", "/tmp/usb", "/tmp/out",
                                                 matched_device_clock=True))
        self.assertNotIn("--matched-device-clock-96000", normal)
        self.assertEqual(matched[:-1], normal)
        self.assertEqual(matched[-1], "--matched-device-clock-96000")

    def test_audio_child_drops_root_and_preserves_literal_paths(self):
        with self.assertRaises(ValueError):
            stats.child_command(0, 0, "/tmp", "/tmp/source", "/tmp/usb", "/tmp/out")
        source = Path("/tmp/a '$() source")
        with patch.dict(os.environ, {"XDG_STATE_HOME": "/tmp/state $(secret)"}):
            command = shlex.split(stats.child_command(1000, 1000, "/tmp/home", source, "/tmp/usb", "/tmp/out"))
        self.assertEqual(command[:7], ["/usr/bin/setpriv", "--reuid", "1000", "--regid", "1000", "--init-groups", "/usr/bin/env"])
        self.assertIn(str(source / "tests_native/studio_alsa_playback.py"), command)
        self.assertIn("XDG_STATE_HOME=/tmp/state $(secret)", command)
        self.assertNotIn("/bin/sh", command)
        self.assertNotIn("--unsafe", command)


if __name__ == "__main__":
    unittest.main()
