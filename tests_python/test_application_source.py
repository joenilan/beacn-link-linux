import copy
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import MagicMock
import wave

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
spec = importlib.util.spec_from_file_location('application_source', ROOT / 'tests_native/application_source.py')
app = importlib.util.module_from_spec(spec); spec.loader.exec_module(app)


class OrdinaryApplicationTests(unittest.TestCase):
    def test_explicit_backend_remote_and_wave_are_bounded_to_authored_media(self):
        args = app.command('/usr/bin/mpv', '/private/authored.wav', 96000, 'beacn-test-owned')
        self.assertIn('--ao=pipewire', args)
        self.assertIn('--pipewire-remote=beacn-test-owned', args)
        self.assertIn('--audio-fallback-to-null=no', args)
        self.assertIn('--no-config', args)
        with tempfile.TemporaryDirectory() as folder:
            p = Path(folder) / 'media.wav'
            app.write_wave(p, b'\x01\x00\x02\x00' * 20, 96000)
            with wave.open(str(p), 'rb') as media:
                self.assertEqual((media.getnchannels(), media.getsampwidth(), media.getframerate(), media.getnframes()), (2, 2, 96000, 20))
                self.assertEqual(media.readframes(20), b'\x01\x00\x02\x00' * 20)

    @unittest.skipUnless(sys.platform == 'linux', 'real retained process/executable metadata')
    def test_name_peer_process_generation_and_private_scope_must_all_match(self):
        pid = os.getpid(); process = MagicMock(pid=pid); process.poll.return_value = None
        retained = app.RetainedMpv(process, sys.executable, 'owned', 'token')
        objects = [
            {'id': 4, 'type': app.inventory.CLIENT, 'info': {'props': {'object.serial': 44, 'application.process.id': pid, 'pipewire.sec.pid': pid}}},
            {'id': 6, 'type': app.inventory.NODE, 'info': {'state': 'running', 'props': {'object.serial': 66, 'client.id': 4,
                'media.class': 'Stream/Output/Audio', 'application.name': 'mpv', 'node.name': 'owned',
                'beacn.test.token': 'token', 'node.autoconnect': False}}}]
        self.assertEqual(retained.validate(objects), 'mpv')
        for index, key, value in ((0, 'pipewire.sec.pid', pid+1), (1, 'application.name', 'other'), (1, 'beacn.test.token', 'other'),
                                  (1, 'node.name', 'other'), (1, 'node.autoconnect', True), (1, 'device.id', 9)):
            changed = copy.deepcopy(objects); changed[index]['info']['props'][key] = value
            with self.assertRaises(RuntimeError): retained.validate(changed)
        retained.start += 1
        with self.assertRaises(RuntimeError): retained.validate(objects)
        retained.start -= 1; retained.stamp = (0, 0, 0, 0)
        with self.assertRaises(RuntimeError): retained.validate(objects)
        process.poll.return_value = 0
        with self.assertRaises(RuntimeError): retained.validate(objects)


if __name__ == '__main__': unittest.main()
