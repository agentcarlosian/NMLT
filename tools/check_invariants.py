#!/usr/bin/env python3
"""Check source safety evidence and independent Lean rejection controls."""
from __future__ import annotations
import argparse
from copy import deepcopy
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--checker", type=Path)
    args = parser.parse_args()
    subprocess.run(["cargo", "build", "--quiet", "-p", "nmlt-cli"], cwd=ROOT, check=True)
    suffix = ".exe" if os.name == "nt" else ""
    binary = ROOT / "target" / "debug" / f"nmlt{suffix}"
    checker = args.checker
    if checker is None:
        subprocess.run(["lake", "build", "nmlt-invariant-check"], cwd=ROOT / "mechanization" / "lean", check=True)
        checker = ROOT / "mechanization" / "lean" / ".lake" / "build" / "bin" / f"nmlt-invariant-check{suffix}"
    checker = checker.resolve()
    parent = ROOT / "target" / "r2-invariants"
    parent.mkdir(parents=True, exist_ok=True)
    evidence = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    source = ROOT / "examples" / "pivot" / "safety_invariant.nmlt"

    for label, name, verdict, steps in [
        ("positive", "UseAfterReceive", "invariant", 0),
        ("later", "NeverUsed", "counterexample", 2),
        ("initial", "AlreadyReceived", "counterexample", 0),
    ]:
        directory = evidence / label
        result = subprocess.run([str(binary), "check-invariant", str(source), "--behavior", "Network", "--property", f"Receiver.{name}", "--max-states", "32", "--checker", str(checker), "--emit-evidence", str(directory)], cwd=ROOT, capture_output=True, text=True, timeout=60)
        (evidence / f"{label}.stdout.json").write_text(result.stdout, encoding="utf-8")
        (evidence / f"{label}.stderr.txt").write_text(result.stderr, encoding="utf-8")
        if result.returncode != (0 if verdict == "invariant" else 1):
            raise SystemExit(f"unexpected {label} exit: {result.stderr}")
        report = json.loads((directory / "result.json").read_text(encoding="utf-8"))
        if report["verdict"] != verdict or report["assurance"] != "lean_checked_finite_model" or report["checker"]["path_steps"] != steps:
            raise SystemExit(f"wrong {label} meaning: {report}")
        print(f"ok: Lean {verdict}, {name}, {steps} path steps", flush=True)

    incomplete = evidence / "incomplete"
    result = subprocess.run([str(binary), "check-invariant", str(source), "--behavior", "Network", "--property", "Receiver.UseAfterReceive", "--max-states", "1", "--checker", str(checker), "--emit-evidence", str(incomplete)], cwd=ROOT, capture_output=True, text=True, timeout=60)
    report = json.loads((incomplete / "result.json").read_text(encoding="utf-8"))
    if result.returncode == 0 or report["assurance"] != "none" or report["verdict"] != "incomplete" or (incomplete / "observation.json").exists():
        raise SystemExit("truncated exploration was not rejected before checking")

    controls: list[dict] = []
    positive_states = json.loads((evidence / "positive" / "witness.json").read_text())["claim"]["states"]
    positive_artifact = json.loads((evidence / "positive" / "invariant.json").read_text())

    def control(name: str, base: str, change, expected: str) -> None:
        directory = evidence / name
        directory.mkdir()
        for filename in ["source.nmlt", "core.json", "invariant.json", "witness.json"]:
            shutil.copyfile(evidence / base / filename, directory / filename)
        core = json.loads((directory / "core.json").read_text())
        artifact = json.loads((directory / "invariant.json").read_text())
        witness = json.loads((directory / "witness.json").read_text())
        change(core, artifact, witness)
        (directory / "core.json").write_text(json.dumps(core), encoding="utf-8")
        artifact["core_sha256"] = digest(directory / "core.json")
        witness["core_sha256"] = artifact["core_sha256"]
        if witness["claim"]["kind"] == "counterexample":
            witness["claim"]["path"]["artifact_sha256"] = artifact["core_sha256"]
        (directory / "invariant.json").write_text(json.dumps(artifact), encoding="utf-8")
        witness["invariant_sha256"] = digest(directory / "invariant.json")
        (directory / "witness.json").write_text(json.dumps(witness), encoding="utf-8")
        result = subprocess.run([str(checker), *[str(directory / f) for f in ["core.json", "source.nmlt", "invariant.json", "witness.json"]]], cwd=ROOT, capture_output=True, text=True, timeout=45)
        (directory / "stdout.txt").write_text(result.stdout, encoding="utf-8")
        (directory / "stderr.txt").write_text(result.stderr, encoding="utf-8")
        if result.returncode == 0 or expected not in result.stderr:
            raise SystemExit(f"{name} did not reject at its intended boundary: {result.stdout} {result.stderr}")
        controls.append({"name": name, "expected": expected, "rejected": True})

    control("missing-initial", "positive", lambda c, a, w: w["claim"]["states"].pop(0), "no admitted initial state")
    control("missing-successor", "positive", lambda c, a, w: w["claim"]["states"].pop(1), "step-preservation obligation failed")
    control("false-initial-obligation", "initial", lambda c, a, w: w.update(claim={"kind": "invariant", "states": deepcopy(positive_states)}), "initialization obligation failed")
    control("forged-true-tree", "positive", lambda c, a, w: a["property"].update(predicate={"kind": "boolean", "term": {"kind": "bool", "type": "Bool", "value": True}}), "tree differs from independently parsed")
    control("wrong-expression-span", "positive", lambda c, a, w: a["property"].update(expression_start=a["property"]["expression_start"] + 1), "expression bytes differ")
    control("wrong-property-name", "positive", lambda c, a, w: a["property"].update(name="Other"), "does not match the source declaration")
    control("wrong-term-type", "positive", lambda c, a, w: a["property"]["predicate"]["left"]["term"].update(type="Unit"), "encoded term type differs")
    control("wrong-subject", "positive", lambda c, a, w: a["property"].update(system="Missing"), "not a selected leaf")
    control("extra-property-field", "positive", lambda c, a, w: a["property"].update(trusted=True), "unexpected object fields")
    control("oversized-set", "positive", lambda c, a, w: w["claim"].update(states=deepcopy(positive_states) * 100), "state/obligation bound exceeded")
    control("invalid-control-index", "positive", lambda c, a, w: w["claim"]["states"][0].update(left=999), "outside the finite domain")
    control("invalid-path-step", "later", lambda c, a, w: w["claim"]["path"]["actions"][1].update(left="unreachable_branch"), "not an actual unified step")
    control("uninitialized-path", "later", lambda c, a, w: w["claim"]["path"]["states"][0]["authority"].update(permit="Receiver"), "does not satisfy the unified initializer")
    control("nonviolating-path", "later", lambda c, a, w: a.update(property=deepcopy(positive_artifact["property"])), "does not falsify the predicate")
    control("stale-source", "positive", lambda c, a, w: a.update(source_sha256="0" * 64), "stale source/core/predicate digest")
    (evidence / "controls.json").write_text(json.dumps(controls, indent=2), encoding="utf-8")
    print(f"ok: {len(controls)} independent Lean rejection controls, including recomputed envelope hashes", flush=True)
    print(f"evidence: {evidence.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
