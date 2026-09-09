#!/usr/bin/env python3
"""Reproduce the local project loop with retained acceptance evidence."""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lean-bin", type=Path)
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--quiet", "-p", "nmlt-cli"], cwd=ROOT, check=True)
    binary = ROOT / "target" / "debug" / ("nmlt.exe" if os.name == "nt" else "nmlt")
    parent = ROOT / "target" / "r2-projects"
    parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    sequence = 0

    def run(*arguments: object) -> dict:
        nonlocal sequence
        sequence += 1
        result = subprocess.run([str(binary), *map(str, arguments)], cwd=ROOT, capture_output=True, text=True, timeout=600)
        (evidence / f"{sequence:02d}.stdout.json").write_text(result.stdout, encoding="utf-8")
        (evidence / f"{sequence:02d}.stderr.txt").write_text(result.stderr, encoding="utf-8")
        if result.returncode:
            raise SystemExit(f"project command {arguments} failed; {evidence}\n{result.stderr}")
        return json.loads(result.stdout)

    project = evidence / "worker"
    run("init", project)
    run("fmt", project)
    run("fmt", project, "--check")
    run("check-project", project)
    fallback = run("run", project)
    changed = run("run", project, "--arg", "input=4")
    if fallback["value"] != {"kind": "ok", "value": {"kind": "int", "value": 50}} or changed["value"] != {"kind": "ok", "value": {"kind": "int", "value": 32}}:
        raise SystemExit("changed-input/fallback/reuse contract failed")
    tested = run("test", project)
    if not tested["passed"] or len(tested["tests"]) != 2:
        raise SystemExit("initialized project tests did not pass")
    record = Path(fallback["record"])
    jobs = record.parent / "jobs"
    journal = (jobs / "journal.jsonl").read_bytes()
    resumed = run("resume", record.parent, "--project", project)
    if resumed["value"] != fallback["value"] or (jobs / "journal.jsonl").read_bytes() != journal:
        raise SystemExit("completed project resumption changed its result or journal")
    jobs.rename(record.parent / "saved-jobs")
    original = (project / "main.nmlt").read_bytes()
    (project / "main.nmlt").write_text("an unfinished edit", encoding="utf-8")
    replay = run("replay", record, "--project", project)
    (project / "main.nmlt").write_bytes(original)
    if not replay["matched"] or jobs.exists() or (record.parent / "saved-jobs" / "journal.jsonl").read_bytes() != journal:
        raise SystemExit("project replay did not preserve saved evidence")
    print("ok: init, fmt/check, typed project check, changed input, fallback/reuse, test, snapshot replay", flush=True)

    if args.lean_bin:
        project = evidence / "lean"
        run("init", project)
        (project / "main.nmlt").write_text('fn main() -> Bool {\n  let job = job_start_lean("existing_lemma");\n  let result = job_collect(job);\n  match result { Ok(proof) => true, Err(problem) => false }\n}\n', encoding="utf-8")
        (project / "nmlt.toml").write_text('schema = "nmlt-project-v1"\nsource = "main.nmlt"\nentry = "main"\nmax_steps = 100\ninputs = {}\n[jobs]\nslots = 1\nmax_attempts = 1\ntimeout_ms = 30000\n[tools]\nlean = ' + json.dumps(args.lean_bin.resolve().as_posix()) + '\n[[tests]]\nname = "real pinned Lean"\nentry = "main"\ninputs = {}\nexpect = true\n', encoding="utf-8")
        locked = run("lock", project)
        if locked["lean_files"] == 0:
            raise SystemExit("Lean installation lock is empty")
        result = run("test", project)
        if not result["passed"]:
            raise SystemExit("real locked Lean project test failed")
        lean_record = Path(result["tests"][0]["record"])
        run("replay", lean_record, "--project", project)
        lean_journal = lean_record.parent / "jobs" / "journal.jsonl"
        original_journal = lean_journal.read_bytes()
        resumed = run("resume", lean_record.parent, "--project", project)
        if resumed["value"] != {"kind": "bool", "value": True} or lean_journal.read_bytes() != original_journal:
            raise SystemExit("Lean project resumption changed the completed result or journal")
        print(f"ok: real Lean project, {locked['lean_files']} locked installation files, source result and replay", flush=True)
    print(f"evidence: {evidence.relative_to(ROOT)}")
    print("scope: executable-only project loop and saved-result resumption; finite invariants and interruption boundaries have separate gates")


if __name__ == "__main__":
    main()
