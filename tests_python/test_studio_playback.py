import copy
import os
from pathlib import Path
import struct
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tests_native"))
import studio_playback as playback
from test_pipewire_graph import objects


def fixture():
    result = objects()
    props = result[3]["info"]["props"]
    props.update({"device.id": 40, "api.alsa.pcm.card": 1, "api.alsa.pcm.stream": "playback",
                  "api.alsa.split.position": "[AUX0,AUX1]", "audio.position": "[FL,FR]",
                  "api.alsa.split.hw-position": "[AUX0,AUX1,AUX2,AUX3,AUX4,AUX5,AUX6,AUX7]",
                  "api.alsa.split.name": "hardware", "node.link-group": "split"})
    result[2]["info"]["props"].update(playback.stream_properties("test", {"serial": 200}))
    def obj(ident, kind, props, **info):
        return {"id": ident, "type": "PipeWire:Interface:" + kind, "info": dict(info, props=props)}
    result.extend([
        obj(40, "Device", {"object.serial": 400, "device.vendor.id": "0x33ae",
                           "device.product.id": "0x4003", "api.alsa.card": 1}),
        obj(50, "Node", {"object.serial": 500, "media.class": "Stream/Output/Audio/Internal",
                         "node.link-group": "split", "target.object": "hardware", "audio.position": "[AUX0,AUX1]"}),
        obj(60, "Node", {"object.serial": 600, "media.class": "Audio/Sink/Internal", "device.id": 40,
                         "node.name": "hardware", "api.alsa.pcm.card": 1,
                         "audio.position": "[AUX0,AUX1,AUX2,AUX3,AUX4,AUX5,AUX6,AUX7]"})])
    for i in range(2):
        result.append(obj(51 + i, "Port", {"object.serial": 510 + i, "node.id": 50,
                          "port.direction": "out", "audio.channel": "AUX" + str(i)}, direction="output"))
    for i in range(8):
        result.append(obj(61 + i, "Port", {"object.serial": 610 + i, "node.id": 60,
                          "port.direction": "in", "audio.channel": "AUX" + str(i)}, direction="input"))
    for i in range(2):
        result.append(obj(71 + i, "Link", {"object.serial": 710 + i}, **{
            "output-node-id": 50, "output-port-id": 51 + i, "input-node-id": 60, "input-port-id": 61 + i}))
    return result


class StudioPlaybackTests(unittest.TestCase):
    def test_exact_pair_and_hardware_links_verified_without_ownership_claims(self):
        source = fixture()
        pin = playback.endpoint(source, 1, 1)
        _, baseline = playback.graph.project_graph([o for o in source if o["id"] not in (7, 10, 11, 12, 30, 31)], 1, {})
        proof = playback.verify(source, pin, 1, 1, "test", 2001, baseline)
        self.assertEqual(len(proof["links"]), 2)
        self.assertTrue(all(l["owner"] is None for l in proof["links"]))
        self.assertEqual(pin["pair"], ["AUX0", "AUX1"])

    def test_missing_pair_ambiguous_device_wrong_card_and_internal_mapping_refused(self):
        for change in ("missing", "duplicate-device", "card", "wrong-channel", "wrong-hardware-link"):
            source = fixture()
            if change == "missing": source[3]["info"]["props"]["api.alsa.split.position"] = "[AUX2,AUX3]"
            if change == "duplicate-device":
                duplicate = copy.deepcopy(next(o for o in source if o["id"] == 40))
                duplicate["id"] = 41; duplicate["info"]["props"]["object.serial"] = 410; source.append(duplicate)
            if change == "card": source[3]["info"]["props"]["api.alsa.pcm.card"] = 2
            if change == "wrong-channel": next(o for o in source if o["id"] == 50)["info"]["props"]["audio.position"] = "[AUX2,AUX3]"
            if change == "wrong-hardware-link": source[-1]["info"]["input-port-id"] = 63
            with self.subTest(change=change), self.assertRaises(ValueError):
                playback.endpoint(source, 1, 1)

    def test_rebinding_fallback_wrong_channel_and_foreign_route_changes_refused(self):
        original = fixture(); pin = playback.endpoint(original, 1, 1)
        _, baseline = playback.graph.project_graph([o for o in original if o["id"] not in (7, 10, 11, 12, 30, 31)], 1, {})
        for change in ("serial", "fallback", "wrong-channel", "foreign-link", "pid", "core"):
            source = copy.deepcopy(original)
            if change == "serial": source[3]["info"]["props"]["object.serial"] = 201
            if change == "fallback": source[2]["info"]["props"]["node.dont-fallback"] = False
            if change == "wrong-channel": source[8]["info"]["input-port-id"] = 22
            if change == "foreign-link": source.pop()
            if change == "pid": source[2]["info"]["props"]["application.process.id"] = 2002
            if change == "core": source[0]["info"]["cookie"] = 124
            with self.subTest(change=change), self.assertRaises((ValueError, LookupError)):
                playback.verify(source, pin, 1, 1, "test", 2001, baseline)

    def test_pcm_is_finite_silent_until_lead_and_peak_bounded(self):
        silent = playback.pcm(False); audible = playback.pcm(True)
        self.assertEqual(len(audible), playback.RATE * playback.DURATION * 4)
        self.assertEqual(set(silent), {0})
        self.assertEqual(audible[:playback.RATE * playback.LEAD * 4], silent[:playback.RATE * playback.LEAD * 4])
        samples = struct.unpack("<" + "h" * (len(audible) // 2), audible)
        self.assertGreater(max(samples), 2000)
        self.assertLessEqual(max(abs(s) for s in samples), 2068)
        self.assertEqual(samples[::2], samples[1::2])

    def test_format_cache_classification_does_not_hide_settings_or_identity_changes(self):
        source = fixture(); pin = playback.endpoint(source, 1, 1)
        before = {"desktop": playback.owner.core.filtered(source), "state_files": {}}
        internal = next(n for n in before["desktop"]["Node"] if n["id"] == 50)
        internal["params"]["Format"] = []
        after = copy.deepcopy(before)
        node = next(n for n in after["desktop"]["Node"] if n["id"] == 50)
        node["params"]["Format"] = [{"mediaType": "audio", "mediaSubtype": "raw", "format": "F32P",
                                      "channels": 2, "position": ["AUX0", "AUX1"]}]
        result = playback.compare_settings(before, after, pin)
        self.assertFalse(result["desktop_exact_equal"])
        self.assertTrue(result["settings_routes_exact_equal"])
        self.assertEqual(len(result["negotiated_format_cache_changes"]), 1)
        node["params"]["Props"] = [{"mute": True}]
        self.assertFalse(playback.compare_settings(before, after, pin)["settings_routes_exact_equal"])
        node["params"].pop("Props"); node["serial"] = 501
        self.assertFalse(playback.compare_settings(before, after, pin)["settings_routes_exact_equal"])

    def test_actual_existing_planner_checks_the_verified_foreign_route(self):
        binary = os.environ.get("BEACN_TEST_PLANNER_BIN")
        if not binary:
            self.skipTest("explicit built CLI unavailable")
        source = fixture(); pin = playback.endpoint(source, 1, 1)
        _, baseline = playback.graph.project_graph([o for o in source if o["id"] not in (7, 10, 11, 12, 30, 31)], 1, {})
        proof = playback.verify(source, pin, 1, 1, "test", 2001, baseline)
        self.assertEqual(playback.planner_check(proof, binary, pin["target"], "test", 1)["operations"], 0)


if __name__ == "__main__":
    unittest.main()
