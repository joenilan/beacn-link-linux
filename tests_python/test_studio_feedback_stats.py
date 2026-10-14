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
        self.assertIn("unroll(64)", program)
        self.assertIn("interval:s:30", program)
        self.assertIn("0x33ae", program)
        self.assertIn("0x4003", program)
        self.assertIn("$ep == 2 || $ep == 130", program)

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
