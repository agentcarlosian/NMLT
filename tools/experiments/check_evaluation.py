#!/usr/bin/env python3
"""Recheck the frozen R5 engineering corpus, without inventing pilot outcomes."""
from __future__ import annotations

import argparse
from copy import deepcopy
import hashlib
import itertools
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

from laboratory import (ROOT, Case, adjacency, cli_binary, cycle_isomorphism, digest, execute, graph_predicate,
                        lean_identity, read, required, source_identity)

MANIFEST = ROOT / "examples/evaluation/manifest.json"
CORPUS = ROOT / "mechanization/lean/NMLT/Evaluation/Corpus.lean"


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")


def validate_manifest(manifest: dict) -> None:
    if manifest["schema"] != "nmlt-evaluation-corpus-v2":
        raise ValueError("unknown evaluation corpus")
    expected_sources = {"mechanization/lean/NMLT/Evaluation/Corpus.lean",
                        "examples/evaluation/dispatch.nmlt", "examples/evaluation/replay.nmlt"}
    if set(manifest["source_sha256"]) != expected_sources:
        raise ValueError("frozen source inventory is incomplete or contains unexpected paths")
    for relative, expected in manifest["source_sha256"].items():
        path = (ROOT / relative).resolve()
        if not re.fullmatch(r"[0-9a-f]{64}", expected) or not path.is_relative_to(ROOT) or sha(path) != expected:
            raise ValueError(f"frozen source changed: {relative}")
    tasks = manifest["proof_tasks"]
    names = [f"{group}{index:02}" for group in
             ("ordinary", "dependency", "repair", "context", "interrupted") for index in range(1, 7)]
    if [row["id"] for row in tasks] != names:
        raise ValueError("expected exactly 30 ordered, distinct Lean tasks")
    headers = {m[0]: m[1].strip() for m in re.findall(r"^theorem (\w+)(.*?) :=",
               CORPUS.read_text(encoding="utf-8"), re.M | re.S)}
    for row in tasks:
        if (row["header"] != headers[row["id"]]
                or row["declaration"] != "NMLT.Evaluation.Corpus." + row["id"]
                or row["invalid_proof"] != "True.intro"):
            raise ValueError(f"task/reference disagreement: {row['id']}")
    scenarios = manifest["behavior_scenarios"]
    if len(scenarios) != 12 or len({row["id"] for row in scenarios}) != 12:
        raise ValueError("expected 12 distinct finite scenarios")
    if len(manifest["discovery_exercises"]) != 3:
        raise ValueError("expected three discovery cards")
    comparison = manifest["comparison"]
    if (comparison["candidates"] != ["True.intro", "NMLT.Evaluation.Corpus.bundle"]
            or comparison["orders"] != [["library", "nmlt"], ["nmlt", "library"]]
            or comparison["repeats"] != 2 or comparison["model_calls"] != 0
            or comparison["permitted_axioms"] != ["propext"]):
        raise ValueError("matched replay contract changed")
    if digest(manifest) != (MANIFEST.parent / "manifest.sha256").read_text().strip():
        raise ValueError("frozen evaluation inventory changed; an explicit corpus revision is required")


def mutate(witness: dict, kind: str) -> dict:
    result = deepcopy(witness)
    if kind == "wrong_initial_owner":
        result["states"][0]["authority"]["permit"] = "Worker"
    elif kind == "missing_permit":
        result["states"][0]["authority"] = {}
    elif kind in ("retained_sender", "lost_transfer"):
        result["states"][1]["authority"]["permit"] = "Producer" if kind == "retained_sender" else None
    elif kind == "unmatched_handoff":
        result["actions"][0]["right"] = None
    elif kind == "repeat_terminal":
        result["states"].append(deepcopy(result["states"][-1]))
        result["actions"].append(deepcopy(result["actions"][-1]))
    elif kind == "early_finish":
        result["actions"][1] = {"left": None, "right": "finish"}
    elif kind == "unknown_owner":
        result["states"][1]["authority"]["permit"] = "Unbound"
    elif kind == "outside_domain":
        result["states"][1]["right"] = 99999
    elif kind == "stale_core":
        result["artifact_sha256"] = "0" * 64
    elif kind == "missing_last_state":
        result["states"].pop()
    elif kind == "no_endpoint":
        result["actions"][0] = {"left": None, "right": None}
    else:
        raise ValueError(f"unknown invalid control: {kind}")
    return result


def larger_bound_exercises(manifest: dict) -> dict:
    worker, graph = manifest["discovery_exercises"][1:]
    if (worker["generations"] != 3 or worker["inputs"] != [0, 1, 2]
            or worker["cancel"] != ["none", "before", "after"] or worker["max_deliveries"] != 4
            or worker["selected_policies"] != [0, 1] or worker["required_cases"] != 9801
            or graph["sizes"] != [8, 9] or graph["required_candidates"] != 15):
        raise ValueError("larger-bound contract changed")
    rows, scores = [], {0: 0, 1: 0}
    for inputs in itertools.product(range(3), repeat=3):
        for cancellation in ("none", "before", "after"):
            for length in range(5):
                for deliveries in itertools.product(range(3), repeat=length):
                    case = Case(inputs, cancellation, deliveries)
                    expected = required(case)
                    for policy in (0, 1):
                        execution = execute(policy, case)
                        if execution.observation != expected:
                            raise ValueError(f"held-out bound failure: {policy}, {case}")
                        scores[policy] += len(execution.bookkeeping)
                    rows.append(case)
    if len(rows) != 9801:
        raise ValueError("incomplete environment coverage")
    objects = [{"size": n, "step": k, "valid": graph_predicate(adjacency(n, k)),
                "cycle_isomorphism": cycle_isomorphism(n, k)}
               for n in (8, 9) for k in range(1, n)]
    if any(row["valid"] != (row["cycle_isomorphism"] is not None) for row in objects):
        raise ValueError("accepted object lacks an explicit isomorphism")
    return {"assurance": "none", "worker_cases": len(rows), "worker_checks": 2 * len(rows),
            "bookkeeping_steps": scores, "graph_candidates": len(objects), "objects": objects,
            "composition_card": "explanatory calibration; shared R4 theorem, no new discovery trial",
            "blind_evaluation": False}


def check_endpoint(core: dict, witness: dict, expected: dict) -> None:
    """Independent finite-index decoding plus frozen endpoint/grade expectations.

    This is executable checking with assurance:none; Lean separately checks the
    complete initialized path. It does not turn source projection into a proof.
    """
    endpoints = {}
    for component, index_key in (("Producer", "left"), ("Worker", "right")):
        fields = core["systems"][component]["state"]
        keys = sorted(fields)
        domains = [[False, True] if fields[key]["type"] == "Bool"
                   else core["enums"][fields[key]["type"]] for key in keys]
        states = list(itertools.product(*domains))
        index = witness["states"][-1][index_key]
        endpoints[component] = dict(zip(keys, states[index]))
    endpoints["authority"] = witness["states"][-1]["authority"]
    work = 0
    for action in witness["actions"]:
        for component, key in (("Producer", "left"), ("Worker", "right")):
            if action[key] is not None:
                work += core["systems"][component]["actions"][action[key]]["resources"]["grade"].get("work", 0)
    endpoints["declared_work"] = work
    if endpoints != expected:
        raise ValueError(f"frozen endpoint/accounting mismatch: {endpoints} != {expected}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--checker", type=Path)
    parser.add_argument("--lean-dir", type=Path, default=ROOT / "mechanization/lean")
    args = parser.parse_args()
    manifest = read(MANIFEST)
    validate_manifest(manifest)
    lean_dir = args.lean_dir.resolve()
    directory = args.output_dir or Path(tempfile.mkdtemp(prefix="evaluation-", dir=ROOT / "target"))
    if args.output_dir:
        directory.mkdir(parents=True, exist_ok=False)
    directory = directory.resolve()
    suffix = ".exe" if os.name == "nt" else ""
    binary = cli_binary()
    checker = (args.checker or (lean_dir / ".lake/build/bin" / ("nmlt-artifact-check" + suffix))).resolve()
    records = []

    def command(label: str, params: list, success: bool = True, message: str | None = None,
                cwd: Path = ROOT) -> str:
        result = subprocess.run([p.as_posix() if isinstance(p, Path) else str(p) for p in params],
                                cwd=cwd, capture_output=True, text=True, encoding="utf-8", timeout=120)
        (directory / f"{label}.stdout.txt").write_text(result.stdout, encoding="utf-8")
        (directory / f"{label}.stderr.txt").write_text(result.stderr, encoding="utf-8")
        record = {"label": label, "exit_code": result.returncode}
        records.append(record)
        write(directory / "progress.json", records)
        if (result.returncode == 0) != success or (message and message not in result.stdout + result.stderr):
            raise SystemExit(f"unexpected result for {label}: {result.stdout[-2000:]} {result.stderr[-2000:]}; {directory}")
        return result.stdout

    # A direct Lean run must successfully check all references before negative
    # controls are counted; an import failure cannot masquerade as a rejection.
    positive = directory / "ReferenceChecks.lean"
    positive.write_text("import NMLT.Evaluation.Corpus\nopen NMLT.Evaluation.Corpus\n" + "\n".join(
        f"example {row['header']} := by apply {row['declaration']}" for row in manifest["proof_tasks"]),
        encoding="utf-8", newline="\n")
    command("lean-references", ["lake", "env", "lean", positive], cwd=lean_dir)
    for row in manifest["proof_tasks"]:
        invalid = directory / f"invalid-{row['id']}.lean"
        invalid.write_text("import NMLT.Evaluation.Corpus\nopen NMLT.Evaluation.Corpus\n"
                           f"example {row['header']} := {row['invalid_proof']}\n", encoding="utf-8", newline="\n")
        command(f"invalid-{row['id']}", ["lake", "env", "lean", invalid], False, "Type mismatch",
                lean_dir)
    print("ok: 30 mathematical references elaborate; 30 type-invalid candidates rejected", flush=True)

    source, core = ROOT / "examples/evaluation/dispatch.nmlt", directory / "core.json"
    command("elaborate", [binary, "elaborate", source, "--core-version", "v2", "--emit-core", core])
    for row in manifest["behavior_scenarios"]:
        name, path = row["id"], directory / (row["id"] + ".json")
        command(name + "-generate", [binary, "trace", "--behavior", row["behavior"], "--actions", row["actions"],
                "--emit-path", path, "--max-states", row["max_states"], core])
        command(name + "-check", [checker, core, source, path])
        command(name + "-recheck", [checker, core, source, path])
        check_endpoint(read(core), read(path), row["expected_endpoint"])
        invalid = directory / (name + "-invalid.json")
        write(invalid, mutate(read(path), row["invalid_control"]))
        command(name + "-invalid", [checker, core, source, invalid], False, row["expected_error"])
        print(f"ok: {name}; Lean accepted, rechecked, and rejected {row['invalid_control']}", flush=True)
    larger = larger_bound_exercises(manifest)
    parity = command("larger-graph-parity", ["lake", "env", "lean", "--run", "tests/EvaluationParity.lean"],
                     cwd=lean_dir)
    expected_rows = [f"{row['size']}|{row['step']}|{str(row['valid']).lower()}|"
                     f"{str(row['cycle_isomorphism'] is not None).lower()}" for row in larger["objects"]]
    if parity.splitlines() != expected_rows:
        raise SystemExit("larger-bound Python/Lean graph parity failed")
    write(directory / "larger-bounds.json", larger)
    write(directory / "summary.json", {
        "schema": "nmlt-evaluation-preflight-v1", "assurance": "none", "manifest_sha256": sha(MANIFEST),
        "execution_identity": source_identity(), "binary_sha256": sha(binary), "checker_sha256": sha(checker),
        "lean_identity": lean_identity(lean_dir),
        "mathematical_references": 30, "type_invalid_candidates_rejected": 30,
        "finite_paths_accepted": 12, "finite_paths_rechecked": 12, "finite_controls_rejected": 12,
        "operational_proof_task_trials": 0, "independent_participants": 0, "live_model_runs": 0,
        "note": "Mathematical preflight is not 30 repair/revision/interruption trials; native proof replay is separate.",
        "commands": records, "larger_bound_exercises": larger,
    })
    print("ok: 9801 larger-bound worker cases per surviving policy; 15 new graph candidates", flush=True)
    print(f"evidence: {directory}")


if __name__ == "__main__":
    main()
