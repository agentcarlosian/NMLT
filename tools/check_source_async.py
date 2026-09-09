#!/usr/bin/env python3
"""Exercise scoped .nmlt controls, captured-session replay, and real Lean."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lean-bin", type=Path)
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--quiet", "-p", "nmlt-cli"], cwd=ROOT, check=True)
    binary = ROOT / "target" / "debug" / ("nmlt.exe" if os.name == "nt" else "nmlt")
    parent = ROOT / "target" / "r2-source-async"
    parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    source = evidence / "main.nmlt"
    shutil.copyfile(ROOT / "examples" / "pivot" / "async_fallback.nmlt", source)
    checks = [("worker", "main", ["--arg", "input=-3", "--arg", "fallback=5"]),
              ("cancel", "cancel", ["--arg", "input=5"])]
    if args.lean_bin:
        checks.append(("lean", "lean", ["--lean-bin", str(args.lean_bin.resolve())]))
    for name, entry, extra in checks:
        output = evidence / f"{name}.json"
        jobs = evidence / name
        command = [str(binary), "run", str(source), "--entry", entry, "--max-steps", "100",
                   "--emit-run", str(output), "--jobs-dir", str(jobs), "--job-slots", "2",
                   "--max-jobs", "2", "--job-timeout-ms", "30000", *extra]
        run = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=600, check=False)
        (evidence / f"{name}.stderr.txt").write_text(run.stderr, encoding="utf-8")
        if run.returncode:
            raise SystemExit(f"source {name} failed; evidence: {evidence}\n{run.stderr}")
        record = json.loads(output.read_text(encoding="utf-8"))
        if record["schema"] != "nmlt-async-source-run-v2" or record["context"]["assurance"] != "none":
            raise SystemExit("wrong source profile or assurance")
        result = record["execution"]["stop"]
        if result["kind"] != "returned":
            raise SystemExit(f"source {name} did not return")
        if name == "worker" and result["value"] != {"kind": "ok", "value": {"kind": "int", "value": 50}}:
            raise SystemExit("source fallback/value reuse failed")
        if name != "cancel" and [e["through"] for e in record["events"][:2]] != [2, 4]:
            raise SystemExit("both jobs were not dispatched before observation")
        if name == "lean":
            if result["value"]["kind"] != "ok" or result["value"]["value"]["kind"] != "text":
                raise SystemExit("source Lean receipt missing")
            identity = record["snapshot"]["manifest"]["configuration"]["lean"]
            if identity["executable_sha256"] != hashlib.sha256(args.lean_bin.read_bytes()).hexdigest():
                raise SystemExit("source Lean executable identity mismatch")
        journal = jobs / "journal.jsonl"
        before = journal.read_bytes()
        for operation, command in [
            ("recover", [str(binary), "jobs-recover", str(jobs)]),
            ("replay", [str(binary), "replay", str(output), "--source", str(source)]),
        ]:
            checked = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=60, check=False)
            (evidence / f"{name}.{operation}.json").write_text(checked.stdout, encoding="utf-8")
            if checked.returncode or journal.read_bytes() != before:
                raise SystemExit(f"source {name} {operation} failed or changed a completed journal: {checked.stderr}")
        print(f"ok: source {name}, collected outcome, replay, unchanged recovered journal")
    # Retain the exact executable required by these records.
    shutil.copyfile(binary, evidence / binary.name)
    print(f"evidence: {evidence.relative_to(ROOT)}")
    print("scope: executable-only source control; captured Lean receipts are not fresh replay checks")


if __name__ == "__main__":
    main()
