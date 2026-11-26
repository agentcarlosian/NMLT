"""Fail-closed corpus and observed-outcome controls."""
from copy import deepcopy
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "tools/experiments"))
from check_evaluation import MANIFEST, validate_manifest
from laboratory import read
from replay_evaluation import source_response


class CorpusTests(unittest.TestCase):
    def test_missing_source_identity_is_not_a_frozen_corpus(self):
        for keys in ([], ["examples/evaluation/dispatch.nmlt"]):
            manifest = read(MANIFEST)
            manifest["source_sha256"] = {k: manifest["source_sha256"][k] for k in keys}
            with self.assertRaises(ValueError):
                validate_manifest(manifest)

    def test_changed_path_or_outcome_needs_explicit_revision(self):
        for field, value in (("actions", "Worker.finish"), ("expected_endpoint", {})):
            manifest = read(MANIFEST)
            manifest["behavior_scenarios"][0][field] = value
            with self.assertRaises(ValueError):
                validate_manifest(manifest)

    def test_current_manifest_has_complete_identity(self):
        validate_manifest(read(MANIFEST))


class ResponseTests(unittest.TestCase):
    def setUp(self):
        self.request = {"schema": "nmlt-project-proof-request-v1", "alias": "corpus",
                        "task_sha256": "a" * 64, "proof": "True.intro"}

    def observation(self, outcome):
        return {"dispatch": {"input": {"type": "text", "value": json.dumps(self.request)}},
                "completion": {"Ok": {"exit_code": 0,
                                      "stdout": list(json.dumps({"outcome": outcome}).encode())}}}

    def test_real_rejection_is_distinct_from_tool_failure(self):
        valid = self.observation({"kind": "failed", "message": "NMLTProof rejected the task/proof:\nNMLTProof.lean:4:49: Type mismatch"})
        self.assertIsNone(source_response(valid, self.request, True))
        for outcome in ({"kind": "failed", "message": "could not create workspace"},
                        {"kind": "failed", "message": "Corpus rejected the task/proof: out of memory"},
                        {"kind": "failed", "message": "NMLTProof rejected the task/proof: timeout"},
                        {"kind": "cancelled"}):
            with self.assertRaises(ValueError):
                source_response(self.observation(outcome), self.request, True)
        failed = deepcopy(valid)
        failed["completion"] = {"Err": "timeout"}
        with self.assertRaises(ValueError):
            source_response(failed, self.request, True)

    def test_candidate_order_and_task_binding_cannot_drift(self):
        observed = self.observation({"kind": "failed", "message": "NMLTProof rejected the task/proof:\nNMLTProof.lean:4:49: Type mismatch"})
        for field, value in (("proof", "NMLT.Evaluation.Corpus.bundle"),
                             ("task_sha256", "b" * 64), ("alias", "other")):
            expected = dict(self.request, **{field: value})
            with self.assertRaises(ValueError):
                source_response(observed, expected, True)

    def test_completed_receipt_must_match_selected_task(self):
        receipt = {"alias": "corpus", "task_sha256": "a" * 64}
        observed = self.observation({"kind": "completed", "value": {"type": "text", "value": json.dumps(receipt)}})
        self.assertEqual(source_response(observed, self.request, False), receipt)
        receipt["task_sha256"] = "b" * 64
        observed = self.observation({"kind": "completed", "value": {"type": "text", "value": json.dumps(receipt)}})
        with self.assertRaises(ValueError):
            source_response(observed, self.request, False)


if __name__ == "__main__":
    unittest.main()
