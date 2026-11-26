"""Synthetic evidence-index tests; these rows are never user observations."""
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "tools/experiments"))
from laboratory import digest, read
from run_solo import run_command
from summarize_solo import PACKET, summarize


class SoloTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        packet = read(PACKET)
        tools = dict(zip(("nmlt", "lean", "exporter", "nanoda", "lake"), (c * 64 for c in "cdef0"), strict=True))
        tool_paths = {name: str(self.directory / name) for name in tools}
        tool_args = [value for name, option in (("lean", "--lean-bin"), ("exporter", "--exporter"), ("nanoda", "--nanoda"))
                     for value in (option, tool_paths[name])]
        self.report = {"schema": "nmlt-solo-validation-v1", "packet_sha256": digest(packet),
            "independent_human_participants": 0, "live_model_calls": 0, "receipts": [],
            "roles": {role: {op: ["command"] for op in ops} for role, ops in read(PACKET)["roles"].items()},
            "proof_rechecks": [], "negative_controls": [], "tool_sha256": tools, "tool_paths": tool_paths,
            "second_uses": [{"role": role, "first": "first", "second": "second", "receipt": "command", "explanation": "synthetic fixture"} for role in read(PACKET)["roles"]],
            "comparison": None, "local_value": "synthetic fixture only", "limitations": ["not an execution observation"]}
        task = {"implementation_sha256": tools["nmlt"], "lean": {"executable_sha256": tools["lean"]}}
        candidate = {"schema": "nmlt-lean-proof-candidate-v1", "task_sha256": "a" * 64, "proof": "synthetic"}
        export = b"synthetic export: not checked mathematics\n"
        result = {"schema": "nmlt-lean-result-v5", "status": "independently_checked", "checked_declarations": 1,
                  "task_sha256": "a" * 64, "task": task, "candidate": candidate,
                  "tools": {"exporter_sha256": tools["exporter"], "nanoda_sha256": tools["nanoda"]},
                  "export_sha256": hashlib.sha256(export).hexdigest(), "export_bytes": len(export),
                  "exported_declarations": ["NMLTChecked.result"]}
        self.put("command", "synthetic command")
        self.put("task", json.dumps(task)); self.put("pin", "a" * 64); self.put("candidate", json.dumps(candidate))
        for label in packet["proof_rechecks"]:
            for suffix in ("accepted", "fresh"):
                base = self.directory / f"{label}-{suffix}"
                (base / "build").mkdir(parents=True)
                (base / "build/environment.ndjson").write_bytes(export)
                self.put(f"{label}-{suffix}", json.dumps(result), f"{base.name}/result.json")
            accepted = self.directory / f"{label}-accepted/result.json"
            fresh = self.directory / f"{label}-fresh"
            self.command(label, [tool_paths["nmlt"], "lean-task", "prove", "--task", str(self.directory / "task.json"),
                "--task-sha256", "a" * 64, "--candidate", str(self.directory / "candidate.json"),
                *tool_args, "--output", str(accepted.parent)], 0)
            self.command(label + "-fresh-command", [tool_paths["nmlt"], "lean-task", "recheck", "--record", str(accepted),
                "--task-sha256", "a" * 64, *tool_args, "--output", str(fresh)], 0)
            self.report["proof_rechecks"].append({"label": label, "accepted": label + "-accepted", "fresh": label + "-fresh",
                "task": "task", "task_pin": "pin", "candidate": "candidate", "accepted_command": label, "fresh_command": label + "-fresh-command"})
        self.command("math-reuse", [tool_paths["lake"], "env", "lean", "--run", "tests/SoloReuse.lean"], 0, stdout="nested reuse: synthetic")
        for identifier, label, diagnostic in (
                ("wrong-term", "wrong", "NMLTProof rejected the task/proof:\nNMLTProof.lean:4:49: Type mismatch"),
                ("stale-candidate", "stale", "candidate differs from selected task hash")):
            self.command(label, ["nmlt", "lean-task", "prove", "--output", str(self.directory / label)], 1, stderr=diagnostic)
            self.put(label + "-diagnostic", diagnostic, label + ".stderr.txt")
            self.report["negative_controls"].append({"id": identifier, "expected": packet["negative_controls"][identifier],
                "observed": packet["negative_controls"][identifier], "receipt": label + "-diagnostic", "command": label})
        self.report["negative_controls"].append({"id": "math-bound-and-formation", "expected": "checked_refutation",
            "observed": "checked_refutation", "receipt": "math-reuse", "command": "math-reuse"})

    def put(self, identifier, content, relative=None):
        path = self.directory / (relative or identifier + ".json")
        path.write_text(content, encoding="utf-8")
        self.report["receipts"] = [r for r in self.report["receipts"] if r["id"] != identifier]
        self.report["receipts"].append({"id": identifier, "path": path.relative_to(self.directory).as_posix(),
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "kind": "synthetic"})

    def command(self, label, argv, code, stdout="", stderr=""):
        record = {"label": label, "command": argv, "exit_code": code, "timed_out": False}
        for stream, content in (("stdout", stdout), ("stderr", stderr)):
            path = self.directory / f"{label}.{stream}.txt"
            path.write_text(content, encoding="utf-8")
            record[stream] = path.name
            record[stream + "_sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        self.put(label, json.dumps(record))

    def test_complete_index_is_not_milestone_or_human_validation(self):
        result = summarize(self.report, self.directory)
        self.assertTrue(result["structural_index_complete"])
        self.assertFalse(result["operations_reviewed"])
        self.assertNotIn("operational_evidence_complete", result)
        self.assertFalse(result["milestone_complete"])
        self.assertEqual(result["independent_human_participants"], 0)

    def test_empty_and_missing_role_operation_control_or_reuse_fail(self):
        for field in ["receipts", "roles", "proof_rechecks", "negative_controls", "second_uses"]:
            report = deepcopy(self.report); report[field] = {} if field == "roles" else []
            with self.assertRaises(ValueError): summarize(report, self.directory)
        report = deepcopy(self.report); del report["roles"]["Lean developer"]["context_revision"]
        with self.assertRaises(ValueError): summarize(report, self.directory)

    def test_recheck_must_be_distinct_and_match_accepted_identity(self):
        report = deepcopy(self.report); report["proof_rechecks"][0]["fresh"] = "repair-accepted"
        with self.assertRaises(ValueError): summarize(report, self.directory)
        result = read(self.directory / "repair-fresh/result.json"); result["task_sha256"] = "b" * 64
        self.put("repair-fresh", json.dumps(result), "repair-fresh/result.json")
        with self.assertRaises(ValueError): summarize(self.report, self.directory)

    def test_host_failure_cannot_be_relabeled_as_type_rejection(self):
        self.put("wrong-diagnostic", "NMLTProof rejected the task/proof: host Io timeout", "wrong.stderr.txt")
        command = read(self.directory / "wrong.json")
        command["stderr_sha256"] = hashlib.sha256((self.directory / "wrong.stderr.txt").read_bytes()).hexdigest()
        self.put("wrong", json.dumps(command))
        with self.assertRaises(ValueError): summarize(self.report, self.directory)

    def test_modified_receipt_and_missing_receipt_fail(self):
        (self.directory / "repair-accepted/result.json").write_text("{}")
        with self.assertRaises(ValueError): summarize(self.report, self.directory)
        (self.directory / "repair-accepted/result.json").unlink()
        with self.assertRaises(ValueError): summarize(self.report, self.directory)

    def test_same_question_is_not_second_use(self):
        self.report["second_uses"][0]["second"] = "first"
        with self.assertRaises(ValueError): summarize(self.report, self.directory)

    def test_comparison_requires_both_orders(self):
        self.report["comparison"] = {"orders": [["library", "nmlt"], ["library", "nmlt"]], "matched_contract": True, "receipt": "command"}
        with self.assertRaises(ValueError): summarize(self.report, self.directory)

    def test_same_path_and_hardlink_aliases_are_not_fresh_artifacts(self):
        report = deepcopy(self.report)
        next(r for r in report["receipts"] if r["id"] == "repair-fresh")["path"] = "repair-accepted/./result.json"
        with self.assertRaises(ValueError): summarize(report, self.directory)
        fresh = self.directory / "repair-fresh/result.json"
        fresh.unlink()
        try:
            os.link(self.directory / "repair-accepted/result.json", fresh)
        except OSError as error:
            self.skipTest(f"hardlinks unavailable: {error}")
        with self.assertRaises(ValueError): summarize(self.report, self.directory)

    def test_changed_export_and_tool_identity_fail(self):
        export = self.directory / "repair-fresh/build/environment.ndjson"
        original = export.read_bytes(); export.write_bytes(b"changed")
        with self.assertRaises(ValueError): summarize(self.report, self.directory)
        export.write_bytes(original)
        self.report["tool_sha256"]["nanoda"] = "0" * 64
        with self.assertRaises(ValueError): summarize(self.report, self.directory)

    def test_unrelated_fresh_command_fails(self):
        record = read(self.directory / "repair-fresh-command.json")
        record["command"][record["command"].index("--record") + 1] = str(self.directory / "revised-proof-accepted/result.json")
        self.put("repair-fresh-command", json.dumps(record))
        with self.assertRaises(ValueError): summarize(self.report, self.directory)

    def test_accepted_command_and_fresh_pin_tools_must_match(self):
        report = deepcopy(self.report); report["proof_rechecks"][0]["accepted_command"] = "math-reuse"
        with self.assertRaises(ValueError): summarize(report, self.directory)
        original = read(self.directory / "repair-fresh-command.json")
        for flag in ("--task-sha256", "--nanoda", "--lean-bin", "--exporter"):
            command = deepcopy(original)
            command["command"][command["command"].index(flag) + 1] = "changed"
            self.put("repair-fresh-command", json.dumps(command))
            with self.assertRaises(ValueError): summarize(self.report, self.directory)

    def test_control_subset_or_substitution_fails(self):
        for controls in (self.report["negative_controls"][:1], self.report["negative_controls"] * 2):
            report = deepcopy(self.report); report["negative_controls"] = controls
            with self.assertRaises(ValueError): summarize(report, self.directory)
        self.report["negative_controls"][0]["id"] = "invented"
        with self.assertRaises(ValueError): summarize(self.report, self.directory)

    def test_panic_or_timeout_with_type_diagnostic_is_not_semantic_rejection(self):
        for code, timed_out in ((101, False), (-9, False), (1, True)):
            command = read(self.directory / "wrong.json")
            command.update(exit_code=code, timed_out=timed_out)
            self.put("wrong", json.dumps(command))
            with self.assertRaises(ValueError): summarize(self.report, self.directory)

    def test_negative_control_cannot_retain_success(self):
        (self.directory / "wrong").mkdir()
        (self.directory / "wrong/result.json").write_text("{}")
        with self.assertRaises(ValueError): summarize(self.report, self.directory)

    def test_command_runner_requires_exact_exit_and_preserves_failure(self):
        for code in (101, -9):
            with patch("run_solo.subprocess.run", return_value=subprocess.CompletedProcess(["mock"], code, "", "Type mismatch")):
                with self.assertRaises(RuntimeError):
                    run_command(self.directory, [], "failure", ["mock"], self.directory, 1, expected=1)
            self.assertEqual(read(self.directory / "failure.command.json")["exit_code"], code)

    def test_command_timeout_preserves_partial_output_and_failure_record(self):
        expired = subprocess.TimeoutExpired(["mock"], 1, output=b"partial stdout", stderr=b"partial stderr")
        with patch("run_solo.subprocess.run", side_effect=expired):
            with self.assertRaises(subprocess.TimeoutExpired):
                run_command(self.directory, [], "timeout", ["mock"], self.directory, 1, expected=1)
        self.assertEqual((self.directory / "timeout.stdout.txt").read_bytes(), b"partial stdout")
        self.assertEqual((self.directory / "timeout.stderr.txt").read_bytes(), b"partial stderr")
        record = read(self.directory / "timeout.command.json")
        self.assertTrue(record["timed_out"])
        self.assertIsNone(record["exit_code"])


if __name__ == "__main__":
    unittest.main()
