import sys
from pathlib import Path
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "tools/experiments"))
from materialize_task import materialize
from summarize_pilot import summarize


class PilotTests(unittest.TestCase):
    def test_one_identity_cannot_fill_independent_audience_slots(self):
        rows = self.observations([("library", "nmlt"), ("nmlt", "library")])
        for row in rows:
            row["participant"] = "same-synthetic-operator"
        with self.assertRaises(ValueError):
            summarize(rows)

    def test_empty_template_cannot_claim_a_successful_pilot(self):
        result = summarize([])
        self.assertEqual(result["participants"], 0)
        self.assertIsNone(result["median_reduction"])
        self.assertFalse(result["descriptive_target_met"])

    def test_unrechecked_success_and_incomplete_pairs_are_not_accepted(self):
        row = dict(participant="synthetic-test", audience="Lean user", task="ordinary01", repetition="0",
                   arm="library", arm_position="1", setup_seconds="1", hands_on_seconds="1", accepted="true", rechecked="false",
                   invalid_controls_expected="1", invalid_controls_rejected="1", diagnosis_seconds="0",
                   maintenance_seconds="0", reuse_explained="true", cpu_seconds="1", peak_memory_mb="1", model_tokens="0")
        with self.assertRaises(ValueError):
            summarize([row])
        with self.assertRaises(ValueError):
            summarize([dict(row, rechecked="true")])

    @staticmethod
    def observations(orders):
        rows = []
        for index, audience in enumerate(("mathematician", "Lean user", "agent-system engineer")):
            for repetition, order in enumerate(orders):
                for position, arm in enumerate(order, 1):
                    rows.append(dict(participant=f"synthetic-{index}", audience=audience, task="ordinary01",
                                     repetition=str(repetition), arm=arm, arm_position=str(position),
                                     setup_seconds="1", hands_on_seconds="100" if arm == "library" else "70",
                                     accepted="true", rechecked="true", invalid_controls_expected="1",
                                     invalid_controls_rejected="1", diagnosis_seconds="0", maintenance_seconds="0",
                                     reuse_explained="true", cpu_seconds="1", peak_memory_mb="1", model_tokens="0"))
        return rows

    def test_one_pair_per_audience_retains_statistics_without_meeting_the_protocol(self):
        result = summarize(self.observations([("library", "nmlt")]))
        self.assertEqual(result["participants"], 3)
        self.assertAlmostEqual(result["median_reduction"], 0.3)
        self.assertFalse(result["paired_protocol_complete"])
        self.assertFalse(result["descriptive_target_met"])
        self.assertEqual(len(result["unresolved"]), 3)

    def test_repeating_the_same_order_does_not_satisfy_counterbalancing(self):
        result = summarize(self.observations([("library", "nmlt")] * 2))
        self.assertEqual(result["paired_observations"], 6)
        self.assertFalse(result["paired_protocol_complete"])
        self.assertFalse(result["descriptive_target_met"])

    def test_two_orders_per_participant_task_satisfy_only_the_descriptive_gate(self):
        result = summarize(self.observations([("library", "nmlt"), ("nmlt", "library")]))
        self.assertTrue(result["paired_protocol_complete"])
        self.assertTrue(result["descriptive_target_met"])
        self.assertEqual(result["status"], "manual_observations_require_review")
        self.assertEqual(result["participants"], 3)
        self.assertEqual(result["paired_observations"], 6)

    def test_each_pair_requires_valid_complementary_positions(self):
        rows = self.observations([("library", "nmlt")])
        for position in ("1", "3", ""):
            invalid = [dict(row) for row in rows]
            invalid[1]["arm_position"] = position
            with self.assertRaises(ValueError):
                summarize(invalid)

    def test_materializer_creates_an_unproved_context_without_reference_solutions(self):
        with tempfile.TemporaryDirectory() as scratch:
            output = Path(scratch) / "task"
            card = materialize("context02", output)
            self.assertEqual(card["assurance"], "none")
            self.assertIn("by sorry", (output / "Goal.lean").read_text())
            self.assertNotIn("theorem context02", (output / "Definitions.lean").read_text())
            with self.assertRaises(FileExistsError):
                materialize("context02", output)


if __name__ == "__main__":
    unittest.main()
