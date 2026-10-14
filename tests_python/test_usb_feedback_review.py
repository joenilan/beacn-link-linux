import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import usb_feedback_review as feedback


def output(maps):
    records = [{"type": "attached_probes", "data": {"probes": 5}},
               {"error": "expected bounded playback stall"}]
    records.extend({"type": "map", "data": {name: data}} for name, data in maps.items())
    return ("\n".join(json.dumps(v) for v in records) + "\n").encode()


class FeedbackReviewTests(unittest.TestCase):
    def test_current_eight_packet_bound_is_enforced_without_reinterpreting_legacy_evidence(self):
        raw = output({"@completions": {"0,130,0": 1}, "@packets": {"0,130,3": 9}})
        feedback.review(raw)  # Legacy records had an explicit 64-packet bound.
        with self.assertRaises(ValueError):feedback.review(raw, max_packets=8)

    def test_urb_success_does_not_hide_frame_overflow_or_empty_feedback(self):
        raw = output({"@completions": {"0,130,0": 2, "0,130,5": 1},
                      "@packets": {"0,130,3": 10, "0,130,0": 6},
                      "@empty_packets": {"0,130": 6}, "@sync_callbacks": {"0": 2}})
        result = feedback.review(raw)
        self.assertEqual(result["endpoints"]["130"]["packet_buckets"][3], 10)
        self.assertEqual(result["endpoints"]["130"]["successful_bytes"], 0)
        self.assertEqual(sum(result["endpoints"]["2"]["completion_buckets"]), 0)
        self.assertFalse(result["needs_review"])

    def test_preserves_warning_and_incomplete_observation_flags(self):
        raw = b"stdin: WARNING: signed comparison\n" + output({"@unsupported_urbs": {"0,130": 1}, "@window_limit": 1})
        result = feedback.review(raw)
        self.assertTrue(result["needs_review"])
        self.assertEqual(result["compiler_warnings"], 1)
        self.assertEqual(result["flags"]["@unsupported_urbs"], 1)

    def test_refuses_inconsistent_counts_foreign_endpoints_and_noninteger_values(self):
        cases = [{"@completions": {"0,130,0": 1}, "@packets": {"0,130,3": 65}},
                 {"@packets": {"0,130,0": 1}, "@empty_packets": {"0,130": 2}},
                 {"@completions": {"0,131,0": 1}}, {"@completions": {"0,130,0": True}},
                 {"@completions": {"0,130,0": -1}}, {"@surprise": {}},
                 {"@successful_bytes": {"0,130": 1}}]
        for maps in cases:
            with self.subTest(maps=maps), self.assertRaises(ValueError):
                feedback.review(output(maps))
        for raw in (b'{}\n', b'{"type":"map","type":"map"}\n', b'x' * (feedback.MAX_BYTES + 1)):
            with self.assertRaises(ValueError):feedback.review(raw)

    def test_retains_requested_lengths_and_separate_source_sink_configuration(self):
        result = feedback.review(output({"@completions": {"0,130,0": 1}, "@packets": {"0,130,3": 8},
                                         "@sync_callbacks": {"0": 1}, "@requested_lengths": {"130,3,288": 8},
                                         "@sync_configuration": {"48000,288,416,96000,416,416": 1}}))
        self.assertEqual(result["requested_lengths"][0]["requested_bytes"], 288)
        self.assertEqual(result["sync_configurations"][0]["sink_rate"], 96000)
        self.assertEqual(result["sync_configurations"][0]["source_rate"], 48000)

    def test_device_clock_return_is_separate_from_configured_rate_and_unknown_is_explicit(self):
        raw = output({"@device_clock_rates": {"96000": 2}, "@clock_unknown": 1}).replace(b'"probes": 5', b'"probes": 7')
        result = feedback.review(raw)
        self.assertEqual(result["device_clock_rates"], [{"rate": 96000, "count": 2}])
        self.assertTrue(result["needs_review"])
        with self.assertRaises(ValueError):feedback.review(output({"@device_clock_rates": {"192000": 1}}))

    def test_submit_entries_remain_separate_from_enqueue_giveback_and_completions(self):
        raw = output({"@handoff_stages": {"0,1,2": 2, "0,2,2": 2, "0,3,2": 1, "2,5,2": 1},
                      "@submission_geometry": {"2,8,3072,1": 2}}).replace(b'"probes": 5', b'"probes": 12')
        result = feedback.review(raw, max_packets=8)
        self.assertTrue(result["has_usb_handoff"])
        counts = result["usb_handoff_entries"]["2"]
        self.assertEqual(counts["usb_submit_entry"], 2)
        self.assertEqual(counts["ehci_enqueue_entry"], 1)
        self.assertEqual(counts["usb_hcd_giveback_entry"], 0)
        self.assertEqual(sum(result["endpoints"]["2"]["completion_buckets"]), 0)
        self.assertFalse(result["needs_review"])
        self.assertFalse(feedback.review(output({}))["has_usb_handoff"])

    def test_handoff_probe_coverage_geometry_bounds_and_endpoint_accounting(self):
        maps = {"@handoff_stages": {"0,1,2": 1}, "@submission_geometry": {"2,8,3072,1": 1}}
        with self.assertRaises(ValueError):feedback.review(output(maps))
        for geometry in ("130,8,3072,1", "2,65,3072,1", "2,8,3330,1", "2,8,3072,3", "2,8,0,1"):
            raw = output({"@handoff_stages": maps["@handoff_stages"], "@submission_geometry": {geometry: 1}})
            with self.subTest(geometry=geometry), self.assertRaises(ValueError):
                feedback.review(raw.replace(b'"probes": 5', b'"probes": 12'))
        raw = output({"@handoff_stages": maps["@handoff_stages"], "@invalid_submission_geometry": {"2": 1}})
        self.assertTrue(feedback.review(raw.replace(b'"probes": 5', b'"probes": 12'))["needs_review"])
        self.assertTrue(feedback.review(output({}).replace(b'"probes": 5', b'"probes": 12'))["needs_review"])


if __name__ == "__main__":
    unittest.main()
