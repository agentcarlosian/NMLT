#!/usr/bin/env python3
"""Bound declaration lookup, native diagnostics and an independently checked repair."""
import argparse
import copy
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from check_lean_tasks import ROOT, digest, read, write


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lean-bin", type=Path, required=True)
    parser.add_argument("--exporter", type=Path, required=True)
    parser.add_argument("--nanoda", type=Path, required=True)
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--quiet", "-p", "nmlt-cli"], cwd=ROOT, check=True)
    binary = ROOT / "target/debug" / ("nmlt.exe" if os.name == "nt" else "nmlt")
    parent = ROOT / "target/r3-lean-inspection"
    parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    project = evidence / "project"
    shutil.copytree(ROOT / "examples/lean-project", project)
    tool_args = ["--lean-bin", str(args.lean_bin.resolve()), "--exporter", str(args.exporter.resolve()),
                 "--nanoda", str(args.nanoda.resolve())]
    rejected = []

    def command(label, parameters, expected=True, contains=None, executable=None):
        result = subprocess.run([str(executable or binary), "lean-task", *map(str, parameters)],
                                cwd=ROOT, capture_output=True, text=True, encoding="utf-8", timeout=600)
        (evidence / f"{label}.stdout.txt").write_text(result.stdout, encoding="utf-8")
        (evidence / f"{label}.stderr.txt").write_text(result.stderr, encoding="utf-8")
        assert (result.returncode == 0) == expected, (label, result.stdout, result.stderr, evidence)
        if contains:
            assert contains in result.stderr, (label, result.stderr)
        if not expected:
            assert not (evidence / label / "result.json").exists(), label
            assert not (evidence / label / "inspection.json").exists(), label
            rejected.append(label)
        print(f"ok: {label}", flush=True)

    command("bound", ["bind", "--project", project, "--lean-bin", args.lean_bin.resolve(),
                      "--output", evidence / "bound"])
    task_path = evidence / "bound/task.json"
    task = read(task_path)
    pin = (evidence / "bound/task.sha256").read_text(encoding="utf-8").strip()

    def inspect(label, prefix, limit="16", expected=True, contains=None, path=task_path, selected=pin, executable=None):
        command(label, ["inspect", "--task", path, "--task-sha256", selected, "--prefix", prefix,
                        "--limit", limit, "--lean-bin", args.lean_bin.resolve(), "--output", evidence / label],
                expected, contains, executable)
        if expected:
            record = read(evidence / label / "inspection.json")
            assert record["schema"] == "nmlt-lean-inspection-v1" and record["status"] == "context_only"
            assert record["assurance"] == "none" and record["task_sha256"] == pin
            assert not (evidence / label / "result.json").exists()
            assert not (evidence / label / "build/environment.ndjson").exists()
            return record

    lookup = inspect("lookup", "Example.")
    declarations = {declaration["name"]: declaration for declaration in lookup["context"]["declarations"]}
    assert lookup["context"]["matched"] == 7 and not lookup["context"]["has_more"]
    assert declarations["Example.offset_eq"]["module"] == "Example.Definitions"
    assert declarations["Example.offset_eq"]["kind"] == "theorem"
    assert declarations["Example.offset_eq"]["axioms"] == []
    assert declarations["Example.offset_eq"]["axioms_within_policy"]
    assert declarations["Example.borrowed"]["axioms"] == ["sorryAx"]
    assert not declarations["Example.borrowed"]["axioms_within_policy"]
    assert declarations["Example.polymorphic"]["level_params"] == ["u"]
    limited = inspect("limited", "Example.", "2")["context"]
    assert limited["has_more"] and limited["matched"] == 7 and len(limited["declarations"]) == 2
    assert limited["declarations"] == lookup["context"]["declarations"][:2]
    assert inspect("missing-prefix", "NotPresent.")["context"]["declarations"] == []
    library = inspect("toolchain-lookup", "Nat.zero_add", "1")["context"]["declarations"][0]
    assert library["name"] == "Nat.zero_add" and library["module"].startswith("Init")

    moved = evidence / "unavailable-project"
    assert project.resolve().is_relative_to(evidence.resolve()) and moved.parent.resolve() == evidence.resolve()
    project.rename(moved)
    unavailable_bound = evidence / "unavailable-bound"
    assert (evidence / "bound").resolve().is_relative_to(evidence.resolve()) and unavailable_bound.parent.resolve() == evidence.resolve()
    (evidence / "bound").rename(unavailable_bound)
    saved = inspect("saved-context", "Example.", path=evidence / "lookup/task.json", executable=evidence / "lookup" / binary.name)
    assert saved["context"] == lookup["context"]
    assert unavailable_bound.resolve().is_relative_to(evidence.resolve()) and (evidence / "bound").parent.resolve() == evidence.resolve()
    unavailable_bound.rename(evidence / "bound")
    assert moved.resolve().is_relative_to(evidence.resolve()) and project.parent.resolve() == evidence.resolve()
    moved.rename(project)

    candidate = evidence / "candidate.json"
    write(candidate, {"schema": "nmlt-lean-proof-candidate-v1", "task_sha256": pin, "proof": "fun n => n"})
    command("wrong-proof", ["prove", "--task", task_path, "--task-sha256", pin,
                            "--candidate", candidate, *tool_args, "--output", evidence / "wrong-proof"],
            False, "Structured Lean diagnostics:")

    def error_diagnostic(directory, expected_text):
        for path in directory.glob("diagnostics-*.json"):
            report = read(path)
            source = (directory / report["source_path"]).read_bytes()
            assert hashlib.sha256(source).hexdigest() == report["source_sha256"]
            assert (directory / report["stage_path"]).is_file()
            for diagnostic in report["diagnostics"]:
                if diagnostic["message"]["severity"] == "error" and expected_text in diagnostic["message"]["data"]:
                    assert report["schema"] == "nmlt-lean-diagnostics-v1"
                    assert report["position_encoding"] == "one-based-lines;zero-based-Unicode-scalar-columns;UTF-8-byte-ranges"
                    span = diagnostic["source_range"]
                    assert span is not None
                    return source[span["start_byte"]:span["end_byte"]].decode("utf-8")
        raise AssertionError((directory, expected_text))

    assert error_diagnostic(evidence / "wrong-proof", "Type mismatch").strip("() ") == "n"
    lemma = declarations["Example.offset_eq"]["name"]
    write(candidate, {"schema": "nmlt-lean-proof-candidate-v1", "task_sha256": pin,
                      "proof": f"fun n => {lemma} n"})
    command("repaired-proof", ["prove", "--task", task_path, "--task-sha256", pin,
                               "--candidate", candidate, *tool_args, "--output", evidence / "repaired-proof"])
    record = read(evidence / "repaired-proof/result.json")
    assert record["schema"] == "nmlt-lean-result-v4" and record["status"] == "independently_checked"
    assert lemma in record["proof"]["proof_references"]
    command("fresh-proof", ["recheck", "--record", evidence / "repaired-proof/result.json",
                            "--task-sha256", pin, *tool_args, "--output", evidence / "fresh-proof"])
    assert read(evidence / "fresh-proof/result.json")["export_sha256"] == record["export_sha256"]

    command("context-is-not-proof", ["recheck", "--record", evidence / "lookup/inspection.json",
                                     "--task-sha256", pin, *tool_args, "--output", evidence / "context-is-not-proof"],
            False, "context is not a proof acceptance record")
    inspect("changed-pin", "Example.", expected=False, contains="selected task hash", selected="0" * 64)
    inspect("command-prefix", "Example.\nrun_meta", expected=False, contains="bounded declaration prefix")
    inspect("excess-limit", "Example.", "17", expected=False, contains="limit from 1 to 16")

    changed = copy.deepcopy(task)
    changed["sources"][0]["text"] += "\n-- Changed dependency bytes\n"
    changed_path = evidence / "changed-task.json"
    write(changed_path, changed)
    inspect("changed-source", "Example.", expected=False, contains="selected task hash", path=changed_path)
    changed = copy.deepcopy(task)
    changed["target"]["type_repr"] = "Lean.Expr.const `False []"
    write(changed_path, changed)
    inspect("rehashed-wrong-target", "Example.", expected=False, contains="target identity", path=changed_path, selected=digest(changed))

    broken = evidence / "broken-project"
    shutil.copytree(project, broken)
    with (broken / "Example/Goals.lean").open("a", encoding="utf-8", newline="\n") as output:
        output.write('\ntheorem unicode : True := by let msg := "λ😀"; exact msg\n')
    command("unicode-build-error", ["bind", "--project", broken, "--lean-bin", args.lean_bin.resolve(),
                                    "--output", evidence / "unicode-build-error"], False, "Structured Lean diagnostics:")
    assert error_diagnostic(evidence / "unicode-build-error", "Type mismatch") == "exact msg"
    write(evidence / "summary.json", {"accepted": ["repaired-proof"], "fresh_rechecks": ["fresh-proof"],
          "inspections": ["lookup", "limited", "missing-prefix", "toolchain-lookup", "saved-context"],
          "rejected": rejected, "lookup_declarations": len(declarations),
          "checked_declarations": record["checked_declarations"], "export_sha256": record["export_sha256"]})
    print(f"evidence: {evidence.relative_to(ROOT)}", flush=True)
    print("scope: bound declaration context and native diagnostics; only prove/recheck can publish proof acceptance", flush=True)


if __name__ == "__main__":
    main()
