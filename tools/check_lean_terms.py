#!/usr/bin/env python3
"""Real Init terms: dynamic inputs, typed rejection, fallback, policy and replay."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--lean-bin", required=True, type=Path)
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--quiet", "-p", "nmlt-cli"], cwd=ROOT, check=True)
    binary = ROOT / "target/debug" / ("nmlt.exe" if os.name == "nt" else "nmlt")
    parent = ROOT / "target/r2-lean-terms"
    parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    source = evidence / "main.nmlt"
    shutil.copyfile(ROOT / "examples/pivot/lean_terms.nmlt", source)
    for name, entry, statement, proof, expected in [
        ("fallback", "main", "forall n : Nat, n + 0 = n", "fun n => Nat.add_zero n", "ok"),
        ("logic", "check", "And True True", "And.intro True.intro True.intro", "ok"),
        ("axiom", "check", "forall p : Prop, forall q : Prop, Iff p q -> p = q", "fun p => fun q => propext", "err"),
    ]:
        record_path = evidence / f"{name}.json"
        jobs = evidence / name
        command = [str(binary), "run", str(source), "--entry", entry, "--max-steps", "100",
                   "--arg", "statement=" + json.dumps(statement), "--arg", "proof=" + json.dumps(proof),
                   "--emit-run", str(record_path), "--jobs-dir", str(jobs), "--job-slots", "1",
                   "--max-jobs", "2", "--job-timeout-ms", "30000", "--lean-bin", str(args.lean_bin.resolve())]
        run = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=600)
        (evidence / f"{name}.stderr.txt").write_text(run.stderr, encoding="utf-8")
        if run.returncode:
            raise SystemExit(f"{name} failed: {run.stderr}; {evidence}")
        record = json.loads(record_path.read_text(encoding="utf-8"))
        value = record["execution"]["stop"]["value"]
        assert value["kind"] == expected, value
        if name == "axiom":
            assert "axiom" in value["value"], value
        lean = record["snapshot"]["manifest"]["configuration"]["lean"]
        assert lean["executable_sha256"] == hashlib.sha256(args.lean_bin.read_bytes()).hexdigest()
        assert len(lean["installation_sha256"]) == 64
        observations = record["snapshot"]["observations"]
        assert len(observations) == (2 if name == "fallback" else 1)
        if name == "fallback":
            assert observations[0]["completion"]["Ok"]["exit_code"] == 1
        before = (jobs / "journal.jsonl").read_bytes()
        replay = subprocess.run([str(binary), "replay", str(record_path), "--source", str(source)], capture_output=True, text=True, timeout=60)
        (evidence / f"{name}.replay.json").write_text(replay.stdout, encoding="utf-8")
        assert replay.returncode == 0, replay.stderr
        assert (jobs / "journal.jsonl").read_bytes() == before
        if name == "logic":
            resumed_path = evidence / "logic-resumed.json"
            resumed = subprocess.run([str(binary), "jobs-resume", str(jobs), "--emit-run", str(resumed_path), "--lean-bin", str(args.lean_bin.resolve())], capture_output=True, text=True, timeout=600)
            (evidence / "logic.resume.stderr.txt").write_text(resumed.stderr, encoding="utf-8")
            assert resumed.returncode == 0, resumed.stderr
            restored = json.loads(resumed_path.read_text(encoding="utf-8"))
            assert restored["execution"] == record["execution"]
            assert (jobs / "journal.jsonl").read_bytes() == before
        print(f"ok: {name}, source inputs, exact installation, replay", flush=True)
    shutil.copyfile(binary, evidence / binary.name)
    print(f"evidence: {evidence.relative_to(ROOT)}", flush=True)

if __name__ == "__main__":
    main()
