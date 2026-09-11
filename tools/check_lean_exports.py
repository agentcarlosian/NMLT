#!/usr/bin/env python3
"""Large real Lean exports, exact file receipts and independent rejection controls."""
import argparse
import copy
import hashlib
import json
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
    parent = ROOT / "target/r3-lean-exports"
    parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    project = evidence / "project"
    shutil.copytree(ROOT / "examples/lean-project-large-export", project)
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
            rejected.append(label)
        print(f"ok: {label}", flush=True)

    command("bound", ["bind", "--project", project, "--lean-bin", args.lean_bin.resolve(),
                      "--output", evidence / "bound"])
    task = read(evidence / "bound/task.json")
    pin = (evidence / "bound/task.sha256").read_text(encoding="utf-8").strip()
    assert digest(task) == pin and task["schema"] == "nmlt-lean-task-v2"
    term = "fun a => fun b => fun c => Nat.mul_assoc a b c"
    candidate = evidence / "candidate.json"
    write(candidate, {"schema": "nmlt-lean-proof-candidate-v1", "task_sha256": pin, "proof": term})
    command("proof", ["prove", "--task", evidence / "bound/task.json", "--task-sha256", pin,
                      "--candidate", candidate, *tool_args, "--output", evidence / "proof"])
    record = read(evidence / "proof/result.json")
    export = evidence / "proof/build/environment.ndjson"
    raw = export.read_bytes()
    assert record["schema"] == "nmlt-lean-result-v3" and record["status"] == "independently_checked"
    assert 65536 < len(raw) == record["export_bytes"] <= 16 * 1024 * 1024
    assert hashlib.sha256(raw).hexdigest() == record["export_sha256"]
    assert record["checked_declarations"] == len(record["exported_declarations"]) > 0
    assert record["proof"]["axioms"] == ["propext"]
    files = [stage for stage in record["stages"] if stage["stdout_file"] is not None]
    assert len(files) == 1
    stage = files[0]
    assert stage["output"] == {"exit_code": 0, "stdout": [], "stderr": []}
    assert stage["stdout_file"]["path"] == "build/environment.ndjson"
    assert stage["stdout_file"]["receipt"] == {"bytes": len(raw), "sha256": record["export_sha256"]}
    assert stage["stdout_file"]["policy"]["max_stdout_bytes"] == 16 * 1024 * 1024
    assert stage["stdout_file"]["policy"]["max_stderr_bytes"] == 65536

    moved = evidence / "unavailable-project"
    assert project.resolve().is_relative_to(evidence.resolve()) and moved.parent.resolve() == evidence.resolve()
    project.rename(moved)
    command("fresh", ["recheck", "--record", evidence / "proof/result.json", "--task-sha256", pin,
                      *tool_args, "--output", evidence / "fresh"], executable=evidence / "proof" / binary.name)
    fresh = read(evidence / "fresh/result.json")
    assert fresh["export_bytes"] == len(raw) and fresh["export_sha256"] == record["export_sha256"]
    assert (evidence / "fresh/build/environment.ndjson").read_bytes() == raw
    assert moved.resolve().is_relative_to(evidence.resolve()) and project.parent.resolve() == evidence.resolve()
    moved.rename(project)

    def recheck(label, changed, contains):
        path = evidence / f"{label}-record.json"
        write(path, changed)
        command(label, ["recheck", "--record", path, "--task-sha256", pin,
                        *tool_args, "--output", evidence / label], False, contains)

    def file_stage(changed):
        return next(s for s in changed["stages"] if s["stdout_file"] is not None)

    changed = copy.deepcopy(record)
    file_stage(changed)["stdout_file"]["receipt"]["sha256"] = "0" * 64
    recheck("changed-file-hash", changed, "capture does not match")

    changed = copy.deepcopy(record)
    file_stage(changed)["stdout_file"]["receipt"]["bytes"] += 1
    recheck("changed-file-size", changed, "capture does not match")

    changed = copy.deepcopy(record)
    changed["export_bytes"] += 1
    file_stage(changed)["stdout_file"]["receipt"]["bytes"] += 1
    recheck("coherent-false-size", changed, "fresh proof artifacts differ")

    changed = copy.deepcopy(record)
    file_stage(changed)["stdout_file"]["policy"]["max_stdout_bytes"] += 1
    recheck("changed-file-policy", changed, "file capture limits")

    changed = copy.deepcopy(record)
    file_stage(changed)["stdout_file"]["path"] = "build/another.ndjson"
    recheck("changed-file-path", changed, "capture does not match")

    changed = copy.deepcopy(record)
    changed["stages"].append(copy.deepcopy(file_stage(changed)))
    recheck("duplicate-file-receipt", changed, "capture does not match")

    changed = copy.deepcopy(record)
    file_stage(changed)["stdout_file"] = None
    recheck("missing-file-receipt", changed, "missing retained export capture")

    changed = copy.deepcopy(record)
    file_stage(changed)["output"]["exit_code"] = 1
    recheck("failed-export-status", changed, "capture does not match")

    changed = copy.deepcopy(record)
    changed["schema"] = "nmlt-lean-result-v2"
    recheck("old-result-format", changed, "retained original CLI executable")

    # Changing to a stricter selected policy does not inherit old acceptance.
    strict = copy.deepcopy(task)
    strict["manifest"]["permitted_axioms"] = []
    strict["discovery"]["configuration"]["permitted_axioms"] = []
    strict_pin = digest(strict)
    write(evidence / "strict-task.json", strict)
    write(evidence / "strict-candidate.json", {"schema": "nmlt-lean-proof-candidate-v1",
                                               "task_sha256": strict_pin, "proof": term})
    command("forbidden-axiom", ["prove", "--task", evidence / "strict-task.json",
                                "--task-sha256", strict_pin, "--candidate", evidence / "strict-candidate.json",
                                *tool_args, "--output", evidence / "forbidden-axiom"], False, "axiom")

    corrupt = evidence / "invalid-large-proof"
    corrupt.mkdir()
    rows = [json.loads(line) for line in raw.decode("utf-8").splitlines()]
    assert "thm" in rows[-1]
    rows[-1]["thm"]["value"] = rows[-1]["thm"]["type"]
    (corrupt / "environment.ndjson").write_text("\n".join(json.dumps(row, separators=(",", ":")) for row in rows) + "\n",
                                               encoding="utf-8", newline="\n")
    write(corrupt / "config.json", read(evidence / "proof/build/nanoda-config.json"))
    result = subprocess.run([str(args.nanoda.resolve()), "config.json"], cwd=corrupt,
                            capture_output=True, text=True, timeout=30)
    assert result.returncode != 0, result.stdout
    (corrupt / "stderr.txt").write_text(result.stderr, encoding="utf-8")
    rejected.append("invalid-large-proof")
    print("ok: independent kernel rejects an invalid large proof", flush=True)

    write(evidence / "summary.json", {"accepted": ["proof"], "fresh_rechecks": ["fresh"],
          "rejected": rejected, "checked_declarations": record["checked_declarations"],
          "export_bytes": len(raw), "export_sha256": record["export_sha256"]})
    print(f"evidence: {evidence.relative_to(ROOT)}", flush=True)
    print("scope: byte-exact proof exports up to 16 MiB; R3 integration remains in progress", flush=True)


if __name__ == "__main__":
    main()
