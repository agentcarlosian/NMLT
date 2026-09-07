#!/usr/bin/env python3
"""Exercise asynchronous control with a real worker and optional pinned Lean."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lean-bin", type=Path)
    args = parser.parse_args()
    parent = ROOT / "target" / "r2-async"
    parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    session = evidence / "session"
    command = ["cargo", "run", "--quiet", "-p", "nmlt-runtime", "--example", "async_jobs", "--"]
    tool = str(args.lean_bin.resolve()) if args.lean_bin else "--without-lean"
    run = subprocess.run(command + [str(session), tool], cwd=ROOT, capture_output=True, text=True, timeout=180, check=False)
    (evidence / "stdout.json").write_text(run.stdout, encoding="utf-8")
    (evidence / "stderr.txt").write_text(run.stderr, encoding="utf-8")
    if run.returncode:
        raise SystemExit(f"async adapter exercise failed; evidence: {evidence}\n{run.stderr}")
    report = json.loads(run.stdout)
    expected = 6 if args.lean_bin else 3
    if (report["assurance"] != "none" or report["with_lean"] != bool(args.lean_bin)
            or report["accounting"]["charged_work"] != expected
            or report["accounting"]["allocated_attempts"] != expected
            or report["accounting"]["reserved_work"] != 0
            or report["events_before_poll"] != 4
            or report["reused_square"] != [25, 25]):
        raise SystemExit("incorrect scope, concurrent dispatch, accounting, or value reuse")
    if args.lean_bin:
        checks = report["lean_checks"]
        if [c["kind"] for c in checks] != ["failed", "completed", "completed"]:
            raise SystemExit("Lean admission control or positive proofs failed")
        if "axiom" not in checks[0]["message"] or "no conclusion" not in report["failure"]["message"]:
            raise SystemExit("Lean rejection or axiom policy was promoted")
        if report["manifest"]["configuration"]["lean"]["executable_sha256"] != hashlib.sha256(args.lean_bin.read_bytes()).hexdigest():
            raise SystemExit("wrong Lean executable identity")
    journal = session / "journal.jsonl"
    before = journal.read_bytes()
    for name, arguments in [("replay", ["--replay", str(session / "snapshot.json")]),
                            ("recover", ["--recover", str(session)])]:
        result = subprocess.run(command + arguments, cwd=ROOT, capture_output=True, text=True, timeout=120, check=False)
        (evidence / f"{name}.json").write_text(result.stdout, encoding="utf-8")
        if result.returncode:
            raise SystemExit(f"{name} failed: {result.stderr}")
        if journal.read_bytes() != before:
            raise SystemExit(f"{name} changed a fully collected journal")
    print(f"ok: async {'Lean and worker' if args.lean_bin else 'worker'} exercise; {expected} attempts; {evidence.relative_to(ROOT)}")
    print("scope: Rust host API and local process observations; replay is consistency only")


if __name__ == "__main__":
    main()
