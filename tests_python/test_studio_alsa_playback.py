import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tests_native"))
import studio_alsa_playback as alsa


class StudioAlsaTests(unittest.TestCase):
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

    def test_clock_refuses_foreign_owner_stopped_missing_or_oversized_metadata(self):
        good = "state: RUNNING\nowner_pid   : 123\nhw_ptr      : 298\n"
        self.assertEqual(alsa.clock(good, 123), 298)
        for raw in (good.replace("123", "456"), good.replace("RUNNING", "SETUP"),
                    good.replace("hw_ptr", "appl_ptr"), good + "x" * 16384):
            with self.subTest(raw=raw[:70]), self.assertRaises(RuntimeError):
                alsa.clock(raw, 123)


if __name__ == "__main__":
    unittest.main()
