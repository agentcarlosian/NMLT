"""Adversarial controls for the experiment contracts, not implementation mirrors."""
import copy
import importlib.util
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("laboratory", ROOT / "tools/experiments/laboratory.py")
lab = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = lab
spec.loader.exec_module(lab)


class ProtocolTests(unittest.TestCase):
    def test_fusion_executes_a_distinct_handoff_trace(self):
        case = lab.Case((1,), "none", ())
        split, fused = lab.execute(0, case), lab.execute(1, case)
        self.assertEqual(split.bookkeeping, ("reserve:0", "publish:0"))
        self.assertEqual(fused.bookkeeping, ("reserve_publish:0",))
        self.assertEqual(split.observation, fused.observation)
        self.assertTrue(split.observation.ready)

    def test_complete_frozen_inventory_and_search(self):
        lab.validate_contract(lab.read(lab.CONTRACT))
        report = lab.worker_search()
        self.assertEqual(report["cases"], 441)
        self.assertEqual(report["candidate_case_checks"], 28224)
        self.assertEqual(report["valid_policies"], [0, 1])
        self.assertEqual(report["winner"], 1)
        self.assertEqual(report["candidates"][0]["bookkeeping_steps"],
                         2 * report["candidates"][1]["bookkeeping_steps"])
        self.assertTrue(all("bookkeeping_steps" not in row for row in report["candidates"][2:]))

    def test_old_response_with_equal_value_is_still_stale(self):
        case = lab.Case((1, 1), "none", (0,))
        self.assertEqual(lab.required(case).status, "pending")
        self.assertNotEqual(lab.simulate(2, case), lab.required(case))

    def test_duplicate_success_is_not_two_completions(self):
        case = lab.Case((2,), "none", (0, 0))
        self.assertEqual(lab.required(case).completions, 1)
        self.assertEqual(lab.simulate(4, case).completions, 2)

    def test_cancel_before_and_after_preserve_distinct_charges(self):
        before = lab.Case((1,), "before", (0,))
        after = lab.Case((1,), "after", (0,))
        self.assertEqual((lab.required(before).spent, lab.required(after).spent), (0, 1))
        self.assertNotEqual(lab.simulate(8, before), lab.required(before))

    def test_no_delivery_does_not_allow_a_candidate_to_remove_enabled_work(self):
        case = lab.Case((1,), "none", ())
        self.assertTrue(lab.required(case).ready)
        self.assertFalse(lab.simulate(16, case).ready)

    def test_required_success_cannot_be_replaced_by_cheap_failure(self):
        case = lab.Case((2,), "none", (0,))
        self.assertEqual(lab.required(case).value, 1)
        self.assertEqual(lab.simulate(32, case).status, "failure")
        self.assertNotEqual(lab.simulate(32, case), lab.required(case))

    def test_manifest_cannot_silently_remove_a_case(self):
        contract = lab.read(lab.CONTRACT)
        contract["worker_cases"] -= 1
        with self.assertRaises(ValueError):
            lab.validate_contract(contract)


class ConstructionTests(unittest.TestCase):
    def test_triangle_disconnection_and_degree_controls(self):
        self.assertFalse(lab.graph_predicate(lab.adjacency(3, 1)))
        self.assertFalse(lab.graph_predicate(lab.adjacency(6, 2)))
        self.assertFalse(lab.graph_predicate(lab.adjacency(6, 3)))
        # Two disjoint 4-cycles meet local degree/triangle conditions, not connectivity.
        two_cycles = [[False] * 8 for _ in range(8)]
        for offset in (0, 4):
            for a in range(4):
                for b in range(4):
                    two_cycles[offset + a][offset + b] = lab.adjacency(4, 1)[a][b]
        self.assertFalse(lab.graph_predicate(two_cycles))

    def test_representation_is_not_permissive(self):
        cycle = lab.adjacency(4, 1)
        for row, column, value in [(0, 0, True), (0, 1, False), (0, 1, 1)]:
            changed = copy.deepcopy(cycle)
            changed[row][column] = value
            self.assertFalse(lab.graph_predicate(changed))
        self.assertFalse(lab.graph_predicate(cycle[:-1]))

    def test_all_accepted_objects_have_checked_isomorphism(self):
        report = lab.graph_search()
        self.assertEqual((report["checked_candidates"], report["accepted_candidate_rows"]), (18, 14))
        self.assertEqual((report["distinct_labeled_matrices"], report["cycle_isomorphism_classes"]), (7, 4))
        self.assertTrue(all(row["cycle_isomorphism"] is not None
                            for row in report["candidates"] if row["predicate"]))
        self.assertFalse(report["parameterized_family_theorem"])

    def test_duplicate_manifest_fields_are_rejected(self):
        with self.assertRaises(ValueError):
            json.loads('{"worker_cases":441,"worker_cases":1}', object_pairs_hook=lab.unique_object)


if __name__ == "__main__":
    unittest.main()
