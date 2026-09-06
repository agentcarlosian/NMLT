"""Runner/acceptance controls; fake checker results here are not proof evidence."""

import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("r0_baselines", ROOT / "tools/baselines/run_baselines.py")
r0 = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(r0)


class FakeChecker:
    def __init__(self, forced=None):
        self.forced = forced
        self.sources = []

    def identity(self):
        return {"kind": "unit-test fake; not proof evidence"}

    def check(self, source):
        self.sources.append(source)
        outcome = self.forced or ("rejected" if "\n  exact n\n" in source else "accepted")
        return {"outcome": outcome, "elapsed_seconds": 0.25,
                "source_sha256": r0.digest(source.encode()), "evidence": "test double"}


class BaselineTests(unittest.TestCase):
    def setUp(self):
        self.manifest = r0.load_manifest()
        self.temporary = tempfile.TemporaryDirectory()
        self.directory = Path(self.temporary.name)

    def tearDown(self):
        self.temporary.cleanup()

    def test_partial_proof_resumes_without_recharging_cached_strategy(self):
        checker = FakeChecker()
        partial = r0.run_proof(self.directory, self.manifest, checker, False, 1)
        self.assertEqual(partial["status"], "incomplete")
        self.assertEqual(partial["strategy_attempts_charged"], 1)
        complete = r0.run_proof(self.directory, self.manifest, checker, True, None)
        self.assertEqual(complete["status"], "complete")
        self.assertEqual(complete["run_id"], partial["run_id"])
        self.assertEqual(complete["strategy_attempts_charged"], 3)
        self.assertEqual(complete["checker_invocations_recorded"], 4)
        self.assertEqual(complete["wall_seconds_recorded"], 1.0)

    def test_cached_acceptance_is_rechecked_and_can_fail(self):
        r0.run_proof(self.directory, self.manifest, FakeChecker(), False, None)
        checker = FakeChecker("timeout")
        replay = r0.run_proof(self.directory, self.manifest, checker, True, None)
        self.assertEqual(len(checker.sources), 3)
        self.assertEqual(replay["status"], "failed")
        self.assertTrue(all(a["current_outcome"] == "timeout" for a in replay["attempts"]))
        self.assertEqual(replay["strategy_attempts_charged"], 3)
        self.assertEqual(replay["checker_invocations_recorded"], 6)

    def test_record_hash_matches_exact_saved_lean_bytes(self):
        record = r0.run_proof(self.directory, self.manifest, FakeChecker(), False, 1)
        checked = record["attempts"][0]["check"]
        saved = (self.directory / checked["source_file"]).read_bytes()
        self.assertEqual(checked["source_sha256"], r0.digest(saved))
        self.assertNotIn(b"\r\n", saved)

    def test_resume_rejects_changed_context_and_fresh_run_preserves_file(self):
        identity = {"config": "one", "checker": "one"}
        path = self.directory / "record.json"
        first = r0.record_for(path, "proof", identity, False)
        r0.write_json(path, first)
        with self.assertRaisesRegex(ValueError, "identity mismatch"):
            r0.record_for(path, "proof", {"config": "two"}, True)
        with self.assertRaisesRegex(ValueError, "already exists"):
            r0.record_for(path, "proof", identity, False)
        self.assertEqual(json.loads(path.read_text()), first)

    def test_fresh_runs_have_distinct_instances_with_same_context(self):
        first = r0.record_for(self.directory / "one.json", "worker", {"config": "one"}, False)
        second = r0.record_for(self.directory / "two.json", "worker", {"config": "one"}, False)
        self.assertNotEqual(first["run_id"], second["run_id"])
        self.assertEqual(first["context_sha256"], second["context_sha256"])

    def test_one_process_keeps_loaded_implementation_identity(self):
        expected = r0.identity(self.manifest, None)
        with patch.object(r0.Path, "read_bytes", return_value=b"file changed after loading"):
            self.assertEqual(r0.identity(self.manifest, None), expected)

    def test_complete_manifest_freezes_budgets_and_coverage(self):
        changes = [
            ("proof", "max_strategy_attempts", 0),
            ("discovery", "max_candidates", 0),
            ("worker", "max_slots", 2),
            ("worker", "max_generations_per_scenario", 1),
            ("worker", "scenarios", []),
        ]
        for section, field, value in changes:
            with self.subTest(field=field):
                changed = copy.deepcopy(self.manifest)
                changed[section][field] = value
                path = self.directory / "manifest.json"
                r0.write_json(path, changed)
                with self.assertRaisesRegex(ValueError, "frozen corpus"):
                    r0.load_manifest(path)

    def test_discovery_failure_stays_unknown_and_history_survives_replay(self):
        failed = r0.run_discovery(self.directory, self.manifest, FakeChecker("timeout"), False)
        self.assertEqual(failed["status"], "failed")
        self.assertEqual([a["outcome"] for a in failed["attempts"]], ["unknown", "unknown"])
        replay = r0.run_discovery(self.directory, self.manifest, FakeChecker(), True)
        self.assertEqual(replay["status"], "complete")
        self.assertEqual(replay["checker_invocations_recorded"], 4)
        self.assertEqual(replay["replay_history"][0][0]["check"]["outcome"], "timeout")
        self.assertEqual(replay["wall_seconds_recorded"], 1.0)
        false_candidate = replay["attempts"][0]
        self.assertEqual(tuple(false_candidate["first_counterexample"]), (0, 1))
        self.assertEqual(false_candidate["checked_target"], "Not (forall a b : Nat, a + b = a)")
        self.assertEqual(replay["attempts"][1]["finite_pairs_checked"], 25)

    def test_worker_covers_all_required_scenarios_and_real_outputs(self):
        record = r0.run_worker(self.directory, self.manifest, False)
        self.assertEqual(record["status"], "complete")
        self.assertEqual(len(record["attempts"]), 4)
        self.assertEqual([a["observed"]["status"] for a in record["attempts"]],
                         ["succeeded", "failed", "cancelled", "succeeded"])
        self.assertEqual(record["attempts"][0]["observed"]["result"], 9)
        self.assertEqual(record["attempts"][-1]["observed"],
                         {"status": "succeeded", "result": 25, "spent": 2, "ignored": 2})

    def test_worker_cannot_dispatch_before_receive_or_twice(self):
        worker = r0.WorkerCase("run", "task")
        worker.reserve(3)
        with self.assertRaisesRegex(ValueError, "received unused authority"):
            worker.dispatch()
        worker.receive()
        worker.dispatch()
        with self.assertRaisesRegex(ValueError, "received unused authority"):
            worker.dispatch()
        self.assertEqual(worker.spent, 1)
        self.assertIsNone(worker.owner)

    def test_late_and_duplicate_results_cannot_settle_new_attempt_or_refund(self):
        worker = r0.WorkerCase("run", "task")
        worker.reserve(4)
        worker.receive()
        late = worker.dispatch()
        worker.cancel()
        worker.reserve(5)
        worker.receive()
        current = worker.dispatch()
        worker.deliver(late)
        self.assertEqual((worker.status, worker.spent, worker.result), ("running", 2, None))
        worker.deliver(current)
        worker.deliver(current)
        self.assertEqual((worker.status, worker.spent, worker.result, worker.ignored),
                         ("succeeded", 2, 25, 2))
        with self.assertRaisesRegex(ValueError, "generation budget"):
            worker.reserve(6)

    def test_cross_run_and_ill_typed_result_binding_is_rejected(self):
        first = r0.WorkerCase("run-a", "task")
        second = r0.WorkerCase("run-b", "task")
        for worker in (first, second):
            worker.reserve(1)
            worker.receive()
        wrong_run = first.dispatch()
        response = second.dispatch()
        second.deliver(wrong_run)
        self.assertEqual(second.status, "running")
        self.assertEqual(second.ignored, 1)
        response["result"] = True  # Python equality alone conflates True and 1.
        with self.assertRaisesRegex(ValueError, "exact output contract"):
            second.deliver(response)

    def test_lean_checker_requires_exact_version_and_command_array(self):
        with self.assertRaisesRegex(ValueError, "JSON array"):
            r0.LeanChecker("lean", self.manifest)
        process = subprocess.CompletedProcess([], 0, "Lean (version 4.30.0, x)", "")
        with patch.object(r0.subprocess, "run", return_value=process):
            with self.assertRaisesRegex(ValueError, "version mismatch"):
                r0.LeanChecker(["lean"], self.manifest)

    def test_zero_exit_without_axiom_report_is_not_acceptance(self):
        checker = object.__new__(r0.LeanChecker)
        checker.command, checker.timeout = ["lean"], 1
        process = subprocess.CompletedProcess([], 0, "", "")
        with patch.object(r0.subprocess, "run", return_value=process):
            self.assertEqual(checker.check("example source")["outcome"], "axiom_policy_failure")
        with patch.object(r0.subprocess, "run", side_effect=subprocess.TimeoutExpired("lean", 1)):
            self.assertEqual(checker.check("example source")["outcome"], "timeout")


if __name__ == "__main__":
    unittest.main()
