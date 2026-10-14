import sys
from pathlib import Path
import unittest
from unittest.mock import patch
import subprocess

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tests_native"))
import studio_native_preflight as preflight


class NativePreflightTests(unittest.TestCase):
    def test_virtualized_unknown_or_unreleased_context_cannot_be_ready(self):
        summary = {key: True for key in ("native_environment", "released_usb_profile",
            "unique_selected_card", "all_four_endpoints_verified", "isolated_graph",
            "alsa_playback_closed", "alsa_capture_closed", "alsa_access", "aplay_available")}
        self.assertTrue(preflight.ready(summary))
        for key in summary:
            changed = dict(summary); changed[key] = False
            with self.subTest(key=key):
                self.assertFalse(preflight.ready(changed))
        self.assertFalse(preflight.ready({}))

    def test_failed_virtualization_detection_is_unknown_and_cannot_claim_native(self):
        with patch.object(preflight.shutil, "which", return_value=None):
            self.assertEqual(preflight.environment(), "unknown")
        with patch.object(preflight.shutil, "which", return_value="detector"), \
             patch.object(preflight.subprocess, "run", return_value=subprocess.CompletedProcess([], 2, "", "error")):
            self.assertEqual(preflight.environment(), "unknown")
        with patch.object(preflight.shutil, "which", return_value="detector"), \
             patch.object(preflight.subprocess, "run", return_value=subprocess.CompletedProcess([], 1, "none\n", "")):
            self.assertEqual(preflight.environment(), "none")


if __name__ == "__main__":
    unittest.main()
