import copy
import array
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import MagicMock, patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests_native"))
spec = importlib.util.spec_from_file_location("physical_bridge", ROOT / "tests_native/studio_pipewire_bridge.py")
bridge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bridge)


def graph():
    def obj(id, kind, p, state="running"):
        return {"id": id, "type": "PipeWire:Interface:" + kind, "info": {"state": state, "props": {"object.serial": id + 100, **p}}}
    data = [obj(i, "Client", {"application.process.id": i * 11}) for i in (2, 3, 4)]
    source = obj(5, "Node", {"client.id": 2, "media.class": "Stream/Output/Audio"})
    target = obj(6, "Node", {"client.id": 3, "media.class": "Audio/Sink", "node.virtual": True, "beacn.test.token": "owned"})
    data += [source, target]
    for id, node, direction, channel in ((10, 5, "out", "FL"), (11, 5, "out", "FR"), (12, 6, "in", "FL"), (13, 6, "in", "FR")):
        data.append(obj(id, "Port", {"node.id": node, "port.direction": direction, "audio.channel": channel}))
    for id, output, input_ in ((20, 10, 12), (21, 11, 13)):
        data.append(obj(id, "Link", {"client.id": 4, "link.output.node": 5, "link.input.node": 6, "link.output.port": output, "link.input.port": input_}, "active"))
    return data, copy.deepcopy(target)


class PhysicalBridgeTests(unittest.TestCase):
    @unittest.skipUnless(sys.platform == "linux", "retained Linux process identity")
    def test_malformed_private_metadata_is_journaled_before_hardware_creation(self):
        binding = {"instance": [1], "attributes": {}, "descriptor_sha256": "same",
                   "interfaces": [{"alternate": 0, "driver": None}]}
        held = {**binding, "interfaces": [{"alternate": 1, "driver": "usbfs"}]}
        native = MagicMock(pid=os.getpid())
        before = {"card": 2, "snapshot": {}, "binding": binding}
        with tempfile.TemporaryDirectory() as directory, patch.object(bridge.owner, "snapshot", return_value={}), patch.object(bridge, "pcms_closed", return_value=True), patch.object(bridge.alsa.playback, "binding", return_value=held), patch.object(bridge.subprocess, "Popen") as hardware:
            root = Path(directory) / "stage"
            dump = MagicMock(side_effect=KeyError("incomplete metadata"))
            with self.assertRaises(RuntimeError):
                bridge.run(Path("unused"), before, root, {}, "private", dump, MagicMock(), MagicMock(), native, {}, 1, "token", MagicMock())
            hardware.assert_not_called()
            journal = json.loads((root / "journal.json").read_text())
            self.assertTrue(journal["error"].startswith("KeyError:"))
            self.assertFalse(journal["physical_progress_verified"])

    def test_authored_source_is_silent_by_default_and_quiet_stereo_when_selected(self):
        silence = bridge.stereo_pcm()
        self.assertEqual(len(silence), bridge.RATE * 4 * 120)
        self.assertEqual(silence.count(b"\0"), len(silence))
        tone = bridge.stereo_pcm(True)
        block = bridge.RATE * 4 * 4
        self.assertEqual(len(tone), len(silence))
        self.assertEqual(tone[:block], tone[block:2 * block])
        self.assertEqual(tone[:bridge.RATE * 4], bytes(bridge.RATE * 4))
        samples = array.array('h', tone[:block])
        if sys.byteorder != 'little':samples.byteswap()
        self.assertEqual(samples[::2], samples[1::2])
        peak = max(abs(v) for v in samples)
        self.assertGreater(peak, 2050)
        self.assertLessEqual(peak, 2067)

    def test_only_complete_active_native_stereo_route_enters_physical_stage(self):
        objects, target = graph()
        self.assertEqual(bridge.assignment_one(objects, target, 33, 22, 44, "owned")["id"], 6)
        variants = []
        for id, key, value in ((6, "object.serial", 999), (6, "client.id", 2), (6, "device.id", 9), (6, "node.virtual", False), (20, "client.id", 2), (20, "link.input.node", 999), (12, "node.id", 999), (12, "port.direction", "out"), (13, "audio.channel", "FL"), (10, "object.serial", 0)):
            changed = copy.deepcopy(objects)
            next(o for o in changed if o["id"] == id)["info"]["props"][key] = value
            variants.append(changed)
        changed = copy.deepcopy(objects); changed[-1]["info"]["state"] = "paused"; variants.append(changed)
        changed = copy.deepcopy(objects); next(o for o in changed if o["id"] == 5)["info"]["state"] = "suspended"; variants.append(changed)
        variants.extend((objects[:-1], objects + [copy.deepcopy(objects[-1])]))
        for changed in variants:
            with self.subTest(changed=changed), self.assertRaises(RuntimeError):
                bridge.assignment_one(changed, target, 33, 22, 44, "owned")


if __name__ == "__main__":
    unittest.main()
