import sys
import signal
import subprocess
import tempfile
import time
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tests_native"))
import studio_alsa_playback as alsa


class StudioAlsaTests(unittest.TestCase):
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

    def test_clock_refuses_foreign_owner_stopped_missing_or_oversized_metadata(self):
        good = "state: RUNNING\nowner_pid   : 123\nhw_ptr      : 298\n"
        self.assertEqual(alsa.clock(good, 123), 298)
        for raw in (good.replace("123", "456"), good.replace("RUNNING", "SETUP"),
                    good.replace("hw_ptr", "appl_ptr"), good + "x" * 16384):
            with self.subTest(raw=raw[:70]), self.assertRaises(RuntimeError):
                alsa.clock(raw, 123)


if __name__ == "__main__":
    unittest.main()
