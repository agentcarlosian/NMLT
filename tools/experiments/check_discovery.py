#!/usr/bin/env python3
"""Reproduce finite discovery evidence and honest, recorded Lean comparisons."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import subprocess
import tempfile
import time

from laboratory import ROOT, digest, graph_search, lean_identity, run, source_identity

LEAN = ROOT / "mechanization/lean"
PREMISES = ["refinement", "wiring", "concreteFormation", "abstractFormation"]


def selection(mask: int) -> str:
    return "[" + ", ".join("." + name for bit, name in enumerate(PREMISES) if mask & (1 << bit)) + "]"


def ordinary_candidate() -> str:
    # This fixed baseline tries the ORIGINAL four-premise theorem using ordinary
    # Lean `decide` for membership, then constructor/application tactics.
    return """by
  intro p retained
  obtain ⟨refinement⟩ := retained .refinement (by decide)
  exact NMLT.Behavior.ResourceBehavior.liftParallel refinement
    (retained .wiring (by decide))
    (retained .concreteFormation (by decide))
    (retained .abstractFormation (by decide))
"""


def lean_attempt(directory: Path, label: str, statement: str, proof: str, lean_dir: Path = LEAN) -> dict:
    source = directory / f"{label}.lean"
    source.write_text("import NMLT.Discovery.CompositionPremises\n"
                      "open NMLT.Discovery.CompositionPremises\n"
                      f"example : {statement} := {proof}\n", encoding="utf-8", newline="\n")
    start = time.monotonic()
    try:
        result = subprocess.run(["lake", "env", "lean", str(source)], cwd=lean_dir,
                                capture_output=True, text=True, encoding="utf-8", timeout=90)
        output = result.stdout + result.stderr
        at_candidate = re.search(re.escape(str(source)) + r":\d+:\d+: error:", output)
        candidate_diagnostic = any(message in output for message in (
            "Type mismatch", "Tactic `decide` proved that the proposition", "unsolved goals"))
        status = ("elaboration_passed" if result.returncode == 0 else
                  "candidate_rejected" if at_candidate and candidate_diagnostic else "unknown_tool_failure")
    except subprocess.TimeoutExpired as error:
        status, output = "unknown_timeout", str(error)
    (directory / f"{label}.log").write_text(output, encoding="utf-8")
    return {"label": label, "statement": statement, "proof_sha256": digest(proof),
            "status": status, "seconds": time.monotonic() - start,
            "assurance": "none", "note": "local elaboration observation, not a proof receipt"}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--lean-dir", type=Path, default=LEAN)
    args = parser.parse_args()
    lean_dir = args.lean_dir.resolve()
    directory = args.output_dir or Path(tempfile.mkdtemp(prefix="discovery-", dir=ROOT / "target"))
    if args.output_dir:
        directory.mkdir(parents=True, exist_ok=False)
    directory = directory.resolve()
    report = run()
    report["lean_identity"] = lean_identity(lean_dir)
    (directory / "computations.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    # Differential execution is a separate check, not a verified translation.
    parity = subprocess.run(["lake", "env", "lean", "--run", "tests/DiscoveryParity.lean"],
                            cwd=lean_dir, capture_output=True, text=True, encoding="utf-8", timeout=90)
    (directory / "graph-parity.log").write_text(parity.stdout + parity.stderr, encoding="utf-8")
    expected = [f"{r['size']}|{r['step']}|{str(r['predicate']).lower()}|"
                f"{str(r['cycle_isomorphism'] is not None).lower()}"
                for r in graph_search()["candidates"]]
    if parity.returncode != 0 or parity.stdout.splitlines() != expected:
        raise SystemExit(f"graph parity failed; evidence: {directory}")
    print("ok: all 18 Python/Lean graph verdicts and isomorphism checks agree", flush=True)

    attempts = []
    for mask in range(16):
        statement = f"Sufficient {selection(mask)}"
        ordinary = lean_attempt(directory, f"original-theorem-{mask:02}", statement, ordinary_candidate(), lean_dir)
        attempts.append(ordinary)
        expected_status = "elaboration_passed" if mask == 15 else "candidate_rejected"
        if ordinary["status"] != expected_status:
            raise SystemExit(f"unexpected original-theorem baseline: {ordinary}; evidence: {directory}")
        if mask not in (7, 11, 15):
            statement = "¬ " + statement
        recorded = lean_attempt(directory, f"recorded-{mask:02}", statement, f"case{mask:02}", lean_dir)
        attempts.append(recorded)
        if recorded["status"] != "elaboration_passed":
            raise SystemExit(f"recorded proof failed: {recorded}; evidence: {directory}")
        print(f"ok: composition subset {mask:02}; baseline={ordinary['status']}; recorded proof elaborates", flush=True)
    for mask in (0, 3):
        invalid = lean_attempt(directory, f"invalid-{mask:02}", f"Sufficient {selection(mask)}", "True.intro", lean_dir)
        attempts.append(invalid)
        if invalid["status"] != "candidate_rejected":
            raise SystemExit(f"invalid candidate not rejected: {invalid}")
    summary = {
        "schema": "nmlt-discovery-comparison-v1", "assurance": "none", "attempts": attempts,
        "deterministic_classification": {"sufficient_masks": [7, 11, 15], "subsets": 16,
            "basis": "case00..case15; independent audit requires the separate, current NMLT gate"},
        "original_theorem_baseline": {"attempts": 16, "elaborated": 1, "unknown_propositions": 15},
        "recorded_developer_candidates": {"attempts": 18, "elaborated": 16, "rejected": 2},
        "claims": "failed proof search is unknown, never a refutation; recorded solutions are exposed",
        "comparison_limit": "different goals for 13 masks; descriptive controls, not matched automation performance",
        "execution_identity": source_identity(),
        "lean_identity": report["lean_identity"],
        "live_model_runs": 0, "mathematical_reviewer_assessment": None,
    }
    (directory / "comparison.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"evidence: {directory}")


if __name__ == "__main__":
    main()
