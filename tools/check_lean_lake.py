#!/usr/bin/env python3
"""Frozen native Lake/editor/automation/revision cases with fresh NanoDA checks."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from check_lean_lsp import check as check_lsp

ROOT = Path(__file__).resolve().parents[1]


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lean-bin", type=Path, required=True)
    parser.add_argument("--exporter", type=Path, required=True)
    parser.add_argument("--nanoda", type=Path, required=True)
    parser.add_argument("--mathlib", action="store_true", help="also rebuild the pinned Mathlib example three times")
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--quiet", "-p", "nmlt-cli"], cwd=ROOT, check=True)
    evidence = Path(tempfile.mkdtemp(prefix="run-", dir=ROOT / "target"))
    binary = evidence / ("nmlt.exe" if os.name == "nt" else "nmlt")
    shutil.copy2(ROOT / "target/debug" / binary.name, binary)
    tools = ["--lean-bin", args.lean_bin.resolve(), "--exporter", args.exporter.resolve(), "--nanoda", args.nanoda.resolve()]
    accepted, rejected = [], []
    project = evidence / "project"
    shutil.copytree(ROOT / "examples/lean-lake-project", project, ignore=shutil.ignore_patterns(".lake"))

    def command(label, params, success=True, contains=None):
        result = subprocess.run([str(binary), "lean-task", *map(str, params)], cwd=ROOT,
                                capture_output=True, text=True, encoding="utf-8", timeout=2400)
        (evidence / f"{label}.stdout.txt").write_text(result.stdout, encoding="utf-8")
        (evidence / f"{label}.stderr.txt").write_text(result.stderr, encoding="utf-8")
        assert (result.returncode == 0) == success, (label, result.stdout, result.stderr, evidence)
        if contains:
            assert contains in result.stderr, (label, result.stderr, evidence)
        if not success:
            assert not (evidence / label / "result.json").exists()
            rejected.append(label)
        print(f"ok: {label}", flush=True)

    def bind(label, selected=project, previous=None, success=True):
        output = evidence / label
        params = ["bind" if previous is None else "revise", "--project", selected,
                  "--lean-bin", args.lean_bin.resolve(), "--output", output]
        if previous:
            task, pin = previous
            reason = evidence / f"{label}-reason.txt"
            reason.write_text(f"Frozen R3 case: {label}.\n", encoding="utf-8")
            params += ["--task", task, "--task-sha256", pin, "--reason", reason]
        command(label, params, success)
        if not success:
            assert not (output / "task.json").exists(), "no-op revision must not publish a task"
            return None
        task = read(output / "task.json")
        pin = (output / "task.sha256").read_text().strip()
        assert task["schema"] == "nmlt-lean-task-v3" and task["discovery"] is None and task["lake"]
        assert hashlib.sha256(json.dumps(task, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest() == pin
        assert all(not f["path"].endswith((".olean", ".ilean", ".ir")) for f in task["lake"]["files"])
        if previous:
            assert task["revision"]["parent_task_sha256"] == previous[1] != pin
            assert read(output / "revision.json")["dependent_acceptance"] == "invalidated_for_revised_task"
        return output / "task.json", pin

    def candidate(label, selected, body, success=True):
        task, pin = selected
        source, output = evidence / f"{label}.lean", evidence / f"{label}.json"
        source.write_text(body, encoding="utf-8", newline="\n")
        command(f"candidate-{label}", ["candidate", "--task", task, "--task-sha256", pin,
                                     "--proof", source, "--output", output], success)
        if not success:
            assert not output.exists()
        return output

    def prove(label, selected, body=None, success=True, contains=None, supplied=None):
        task, pin = selected
        supplied = supplied or candidate(label, selected, body)
        output = evidence / label
        command(label, ["prove", "--task", task, "--task-sha256", pin, "--candidate", supplied,
                        *tools, "--output", output], success, contains)
        if success:
            record = read(output / "result.json")
            assert record["schema"] == "nmlt-lean-result-v5" and record["status"] == "independently_checked"
            assert record["task_sha256"] == pin and record["checked_declarations"] > 0
            assert record["task"]["target"] == read(task)["target"]
            exported = (output / "build/environment.ndjson").read_bytes()
            assert len(exported) == record["export_bytes"]
            assert hashlib.sha256(exported).hexdigest() == record["export_sha256"]
            accepted.append({"label": label, "task_sha256": pin, "export_sha256": record["export_sha256"],
                             "declarations": record["checked_declarations"], "axioms": record["proof"]["axioms"]})
        return output

    # The editor is a native Lake project for all input profiles. Its Lake and
    # Lean processes need the explicit project policy even for a local v1 task.
    explicit = evidence / "explicit-bound"
    command("explicit-bound", ["bind", "--project", ROOT / "examples/lean-project",
                               "--lean-bin", args.lean_bin.resolve(), "--output", explicit])
    explicit_pin = (explicit / "task.sha256").read_text().strip()
    explicit_editor = evidence / "explicit-editor"
    command("explicit-editor", ["workspace", "--task", explicit / "task.json", "--task-sha256", explicit_pin,
                                "--lean-bin", args.lean_bin.resolve(), "--output", explicit_editor])
    explicit_workspace = read(explicit_editor / "workspace.json")
    assert explicit_workspace["status"] == "context_only" and explicit_workspace["assurance"] == "none"
    assert explicit_workspace["process_policy"]["contract"].startswith("nmlt-contained-project-process-v1;")
    assert not (explicit_editor / "result.json").exists()

    base = bind("bound")
    task = read(base[0])
    assert len(task["lake"]["packages"]) == 2
    assert task["lake"]["target_source"] == "root/src/App/Goals.lean"
    assert any(f["path"] == "packages/p1/lib/Support/Core.lean" for f in task["lake"]["files"])
    direct = prove("direct", base, "fun n => Support.shift_eq n")
    prove("native-exact", base, "by\n  intro n\n  exact Support.shift_eq n\n")
    prove("wrong-proof", base, "by\n  intro n\n  exact True.intro\n", False, "rejected the task/proof")
    prove("forbidden-axiom", base, "by\n  intro n\n  simpa only [App.offset, Support.shift_eq]\n", False, "axiom outside")
    candidate("forbidden-tactic", base, "by\n  sorry\n", False)
    editor = evidence / "editor"
    command("editor", ["workspace", "--task", base[0], "--task-sha256", base[1],
                       "--lean-bin", args.lean_bin.resolve(), "--output", editor])
    assert read(editor / "workspace.json")["status"] == "context_only"
    assert not (editor / "result.json").exists()
    proof_file = editor / "build/NMLTProof.lean"
    full_source = proof_file.read_text(encoding="utf-8").replace("  sorry", "  intro n\n  exact Support.shift_eq n")
    proof_file.write_text(full_source, encoding="utf-8", newline="\n")
    editor_candidate = candidate("full-editor-file", base, full_source)
    prove("editor-proof", base, supplied=editor_candidate)
    candidate("changed-wrapper", base, full_source.replace("NMLTTask.target", "True"), False)
    check_lsp(editor / "build", args.lean_bin, evidence / "lsp")
    bind("no-op-revision", previous=base, success=False)
    goal_path = project / "src/App/Goals.lean"
    original_goal = goal_path.read_text(encoding="utf-8")
    goal_path.write_text(original_goal.replace("offset n = n", "offset n + 1 = n + 1"), encoding="utf-8", newline="\n")
    revised = bind("revised-statement", previous=base)
    prove("stale-candidate", revised, supplied=evidence / "direct.json", success=False, contains="selected task hash")
    command("stale-result", ["recheck", "--record", direct / "result.json", "--task-sha256", revised[1],
                             *tools, "--output", evidence / "stale-result"], False, "selected task mismatch")
    prove("revised-proof", revised, "fun n => congrArg Nat.succ (Support.shift_eq n)")
    definitions = project / "src/App/Definitions.lean"
    original_definitions = definitions.read_text(encoding="utf-8")
    definitions.write_text(original_definitions.replace("Support.shift n", "Support.shift n + 1"), encoding="utf-8", newline="\n")
    changed_context = bind("changed-context", previous=revised)
    prove("context-sensitive-rejection", changed_context, "fun n => congrArg Nat.succ (Support.shift_eq n)", False, "rejected the task/proof")
    definitions.write_text(original_definitions, encoding="utf-8", newline="\n")
    configuration = read(project / "nmlt-lean.json")
    configuration["permitted_axioms"] = ["propext", "Quot.sound", "Classical.choice"]
    write(project / "nmlt-lean.json", configuration)
    allowed = bind("revised-policy", previous=changed_context)
    prove("native-simp", allowed, "by\n  intro n\n  simp only [App.offset, Support.shift_eq]\n")
    goal_path.write_text("import App.Definitions\nnamespace App\ntheorem zero_offset : ∀ n : Nat, n + 1 > n := by sorry\nend App\n", encoding="utf-8", newline="\n")
    arithmetic = bind("arithmetic", previous=allowed)
    prove("native-omega", arithmetic, "by\n  intro n\n  omega\n")
    prove("native-grind", arithmetic, "by\n  intro n\n  grind\n")
    if args.mathlib:
        mathlib = bind("mathlib-bound", ROOT / "examples/lean-mathlib-project")
        snapshot = read(mathlib[0])["lake"]
        assert len(snapshot["packages"]) == 10 and len(snapshot["files"]) > 9000
        assert all(p["git_commit"] == p["origin"]["rev"] and p["git_clean"] for p in snapshot["packages"][1:])
        assert any(f["path"].endswith("widget/js/lake.trace") for f in snapshot["files"])
        prove("mathlib-proof", mathlib, "fun n => Nat.choose_zero_right n")
    # Remove the original local source and binding locations from every fresh
    # recheck's paths. The proof result retains its own exact input bundle.
    for path in [project, *(evidence / label for label in ["bound", "revised-statement", "changed-context", "revised-policy", "arithmetic"])]:
        target = path.with_name(path.name + "-unavailable")
        assert path.resolve().is_relative_to(evidence.resolve()) and target.resolve().is_relative_to(evidence.resolve())
        path.rename(target)
    for item in accepted:
        label = item["label"]
        output = evidence / label
        fresh = evidence / f"{label}-fresh"
        command(f"{label}-fresh", ["recheck", "--record", output / "result.json", "--task-sha256", item["task_sha256"], *tools, "--output", fresh])
        assert read(fresh / "result.json")["export_sha256"] == item["export_sha256"]
        item["fresh_record"] = f"{label}-fresh/result.json"
    source = direct / "lake-sources/packages/p1/lib/Support/Core.lean"
    original = source.read_bytes()
    source.write_bytes(original + b"\n-- changed retained source\n")
    command("mutated-retained-source", ["recheck", "--record", direct / "result.json", "--task-sha256", base[1],
                                        *tools, "--output", evidence / "mutated-retained-source"], False, "retained Lake input changed")
    source.write_bytes(original)
    write(evidence / "summary.json", {"schema": "nmlt-r3-lake-validation-v1", "accepted": accepted,
                                     "rejected": rejected, "lsp": "lsp/summary.json", "mathlib_included": args.mathlib})
    print(f"evidence: {evidence}", flush=True)


if __name__ == "__main__":
    main()
