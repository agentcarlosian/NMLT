#!/usr/bin/env python3
"""Real local Lean targets, independent artifacts, revisions and fresh rechecks."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]

def read(path):
    return json.loads(path.read_text(encoding="utf-8"))

def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")

def digest(task):
    return hashlib.sha256(json.dumps(task, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lean-bin", type=Path, required=True)
    parser.add_argument("--exporter", type=Path, required=True)
    parser.add_argument("--nanoda", type=Path, required=True)
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--quiet", "-p", "nmlt-cli"], cwd=ROOT, check=True)
    binary = ROOT / "target/debug" / ("nmlt.exe" if os.name == "nt" else "nmlt")
    parent = ROOT / "target/r3-lean-tasks"
    parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    project = evidence / "project"
    shutil.copytree(ROOT / "examples/lean-project", project)
    tool_args = ["--lean-bin", str(args.lean_bin.resolve()), "--exporter", str(args.exporter.resolve()), "--nanoda", str(args.nanoda.resolve())]
    rejected = []

    def command(label, parameters, expected=True, contains=None, executable=None):
        result = subprocess.run([str(executable or binary), "lean-task", *map(str, parameters)], cwd=ROOT,
                                capture_output=True, text=True, encoding="utf-8", timeout=600)
        (evidence / f"{label}.stdout.txt").write_text(result.stdout, encoding="utf-8")
        (evidence / f"{label}.stderr.txt").write_text(result.stderr, encoding="utf-8")
        assert (result.returncode == 0) == expected, (label, result.stdout, result.stderr, evidence)
        if contains:
            assert contains in result.stderr, (label, result.stderr)
        if not expected:
            assert not (evidence / label / "result.json").exists(), label
            rejected.append(label)
        print(f"ok: {label}", flush=True)

    def bind(label):
        output = evidence / label
        command(label, ["bind", "--project", project, "--lean-bin", args.lean_bin.resolve(), "--output", output])
        task = read(output / "task.json")
        assert task["schema"] == "nmlt-lean-task-v2" and task["discovery"] is None
        pin = (output / "task.sha256").read_text(encoding="utf-8").strip()
        assert digest(task) == pin
        return output / "task.json", pin

    def prove(label, task, pin, term, expected=True, contains=None):
        candidate = evidence / f"{label}-candidate.json"
        write(candidate, {"schema": "nmlt-lean-proof-candidate-v1", "task_sha256": pin, "proof": term})
        command(label, ["prove", "--task", task, "--task-sha256", pin, "--candidate", candidate,
                        *tool_args, "--output", evidence / label], expected, contains)
        if expected:
            result = read(evidence / label / "result.json")
            assert result["schema"] == "nmlt-lean-result-v4"
            assert result["status"] == "independently_checked"
            assert result["task_sha256"] == pin
            assert result["checked_declarations"] == len(result["exported_declarations"]) > 0
            export_file = evidence / label / "build/environment.ndjson"
            assert hashlib.sha256(export_file.read_bytes()).hexdigest() == result["export_sha256"]
            assert export_file.stat().st_size == result["export_bytes"]
            assert result["proof"]["root"] == "NMLTChecked.result"
            assert "sorryAx" not in result["proof"]["axioms"]
            assert "NMLTTask.target" in result["exported_declarations"]
            assert (evidence / label / "proof.patch").is_file()
            assert read(evidence / label / "proof-dependencies.json") == result["proof_dependencies"]
            assert [node["name"] for node in result["proof_dependencies"]["nodes"]] == result["exported_declarations"]
            return result

    task, pin = bind("bound")
    original = prove("definition", task, pin, "fun n => Example.offset_eq n")
    assert original["proof"]["axioms"] == []
    # The original project is deliberately unavailable during fresh rechecking.
    moved = evidence / "unavailable-project"
    assert project.resolve().is_relative_to(evidence.resolve()) and moved.parent.resolve() == evidence.resolve()
    project.rename(moved)
    command("fresh", ["recheck", "--record", evidence / "definition/result.json", "--task-sha256", pin,
                      *tool_args, "--output", evidence / "fresh"],
            executable=evidence / "definition" / binary.name)
    assert read(evidence / "fresh/result.json")["export_sha256"] == original["export_sha256"]
    assert read(evidence / "fresh/result.json")["proof_dependencies"] == original["proof_dependencies"]
    assert moved.resolve().is_relative_to(evidence.resolve()) and project.parent.resolve() == evidence.resolve()
    moved.rename(project)

    prove("wrong-term", task, pin, "fun n => n", False, "rejected")
    prove("unresolved", task, pin, "Example.zero_offset", False, "axiom")
    prove("transitive-unresolved", task, pin, "Example.borrowed", False, "axiom")

    # Revisions invalidate the selected pin, including a definition whose name stays fixed.
    changed = read(task)
    changed["sources"][0]["text"] = changed["sources"][0]["text"].replace("0 + n", "1 + n")
    changed_path = evidence / "changed-definition-task.json"
    write(changed_path, changed)
    prove("changed-definition", changed_path, pin, "fun n => Example.offset_eq n", False, "selected task hash")

    # Recompute the envelope hash: Lean still has to confirm the claimed formal target.
    changed = read(task)
    changed["target"]["type_repr"] = "Lean.Expr.const `False []"
    changed_path = evidence / "changed-type-task.json"
    write(changed_path, changed)
    prove("rehashed-wrong-type", changed_path, digest(changed), "fun n => Example.offset_eq n", False, "target identity")

    for label, field, value in [("changed-proof-source", "proof_source", "theorem altered : True := True.intro"),
                                ("changed-checker", "tools", {**original["tools"], "nanoda_sha256": "0" * 64})]:
        changed = copy.deepcopy(original)
        changed[field] = value
        path = evidence / f"{label}-record.json"
        write(path, changed)
        command(label, ["recheck", "--record", path, "--task-sha256", pin, *tool_args,
                        "--output", evidence / label], False, "altered" if field == "proof_source" else "changed")

    # An invalid exported proof must be rejected by the independent kernel itself.
    corrupt = evidence / "invalid-kernel-proof"
    corrupt.mkdir()
    rows = [json.loads(line) for line in (evidence / "definition/build/environment.ndjson").read_text(encoding="utf-8").splitlines()]
    assert "thm" in rows[-1]
    rows[-1]["thm"]["value"] = rows[-1]["thm"]["type"]
    (corrupt / "environment.ndjson").write_text("\n".join(json.dumps(row, separators=(",", ":")) for row in rows) + "\n", encoding="utf-8", newline="\n")
    config = read(evidence / "definition/build/nanoda-config.json")
    write(corrupt / "config.json", config)
    result = subprocess.run([str(args.nanoda.resolve()), "config.json"], cwd=corrupt, capture_output=True, text=True, timeout=30)
    assert result.returncode != 0, result.stdout
    (corrupt / "stderr.txt").write_text(result.stderr, encoding="utf-8")
    rejected.append("invalid-kernel-proof")
    print("ok: independent kernel rejects an invalid proof value", flush=True)

    manifest = read(project / "nmlt-lean.json")
    for label, target, term, permitted in [
        ("assumption", "Example.use_assumption", "fun n => fun m => fun h => Eq.trans (Example.offset_eq n) h", []),
        ("polymorphic", "Example.polymorphic", "fun alpha => fun x => Eq.refl x", []),
        ("permitted-axiom", "Example.equivalence", "fun p => fun q => fun h => propext h", ["propext"]),
    ]:
        manifest["target"] = target
        manifest["permitted_axioms"] = permitted
        write(project / "nmlt-lean.json", manifest)
        bound, selected = bind(f"bound-{label}")
        result = prove(label, bound, selected, term)
        assert result["proof"]["axioms"] == permitted
        if label == "polymorphic":
            assert result["task"]["target"]["level_params"] == ["u"]

    manifest["permitted_axioms"] = []
    write(project / "nmlt-lean.json", manifest)
    bound, selected = bind("bound-forbidden-axiom")
    prove("forbidden-axiom", bound, selected, "fun p => fun q => fun h => propext h", False, "axiom")
    summary = {"accepted": ["definition", "assumption", "polymorphic", "permitted-axiom"],
               "fresh_rechecks": ["fresh"], "rejected": rejected}
    write(evidence / "summary.json", summary)
    print(f"evidence: {evidence.relative_to(ROOT)}", flush=True)
    print("scope: bounded trusted local Lean projects; independent exported proofs; R3 integration remains in progress", flush=True)

if __name__ == "__main__":
    main()
