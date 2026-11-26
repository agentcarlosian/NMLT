#!/usr/bin/env python3
"""Frozen, finite discovery experiments. Executable evidence has assurance:none.

This model deliberately makes no claim about host processes, elapsed CPU time,
unbounded liveness, or the verified translation of the Python implementation.
"""
from __future__ import annotations

import argparse
from dataclasses import asdict, dataclass
import hashlib
import itertools
import json
import os
from pathlib import Path
import platform
import subprocess

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "examples/discovery/contract.json"


def cli_binary() -> Path:
    suffix = ".exe" if os.name == "nt" else ""
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", str(ROOT / "target")))
    if not target_dir.is_absolute():
        target_dir = (ROOT / target_dir).resolve()
    binary = target_dir / "debug" / ("nmlt" + suffix)
    if not binary.is_file():
        raise RuntimeError(f"missing nmlt binary at {binary}; build it first with `cargo build -p nmlt-cli`")
    return binary


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(value: object) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


def unique_object(pairs: list[tuple[str, object]]) -> dict:
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON field: {key}")
        value[key] = item
    return value


def read(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)


@dataclass(frozen=True)
class Case:
    # 0 is a required typed failure; 1 and 2 require successful values 0 and 1.
    inputs: tuple[int, ...]
    cancel: str  # none, before dispatch, or after dispatch of the final generation
    deliveries: tuple[int, ...]


@dataclass(frozen=True)
class Observation:
    status: str
    value: int | None
    generation: int | None
    completions: int
    ignored: int
    spent: int
    ready: bool


@dataclass(frozen=True)
class Execution:
    observation: Observation
    bookkeeping: tuple[str, ...]


def source_identity() -> dict:
    paths = [Path(__file__), ROOT / "tools/experiments/check_discovery.py",
             ROOT / "tools/experiments/check_evaluation.py", ROOT / "mechanization/lean/NMLT.lean",
             ROOT / "mechanization/lean/lean-toolchain", ROOT / "mechanization/lean/lakefile.toml",
             *sorted((ROOT / "mechanization/lean/NMLT").rglob("*.lean"))]
    return {"python": platform.python_version(), "platform": platform.platform(),
            "source_sha256": {p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                              for p in paths}}


def lean_identity(lean_dir: Path | None = None) -> dict:
    """Actual compiler and Lake launcher used for the local elaboration checks."""
    cwd = (lean_dir or (ROOT / "mechanization/lean")).resolve()
    prefix = subprocess.run(["lake", "env", "lean", "--print-prefix"], cwd=cwd,
                            check=True, capture_output=True, text=True, timeout=60).stdout.strip()
    compiler = Path(prefix) / "bin" / ("lean.exe" if platform.system() == "Windows" else "lean")
    lake = compiler.parent / ("lake.exe" if platform.system() == "Windows" else "lake")
    version = subprocess.run([str(compiler), "--version"], check=True, capture_output=True,
                             text=True, timeout=30).stdout.strip()
    return {"version": version, "compiler_sha256": hashlib.sha256(compiler.read_bytes()).hexdigest(),
            "lake_sha256": hashlib.sha256(lake.read_bytes()).hexdigest()}


def cases() -> list[Case]:
    """Environment words do not depend on a candidate's enabled actions."""
    return [
        Case(inputs, cancel, deliveries)
        for count in (1, 2)
        for inputs in itertools.product(range(3), repeat=count)
        for cancel in ("none", "before", "after")
        for length in range(4)
        for deliveries in itertools.product(range(count), repeat=length)
    ]


def required(case: Case) -> Observation:
    """Declarative contract, independent of the candidate event loop."""
    latest = len(case.inputs) - 1
    delivered = case.cancel == "none" and latest in case.deliveries
    status = ("cancelled" if case.cancel != "none" else
              "pending" if not delivered else
              "failure" if case.inputs[-1] == 0 else "success")
    return Observation(
        status, case.inputs[-1] - 1 if delivered and case.inputs[-1] else None,
        latest if delivered else None, int(delivered),
        len(case.deliveries) - int(delivered),
        len(case.inputs) - int(case.cancel == "before"),
        status == "pending",
    )


def execute(policy: int, case: Case) -> Execution:
    """Six switches: fusion, stale, duplicate, cancel, drop, cheap failure.

    All but fusion intentionally remove a contract obligation. They are search
    candidates, not permitted changes to the production runtime.
    """
    if not 0 <= policy < 64:
        raise ValueError("policy outside frozen search space")
    stale, duplicate, ignore_cancel, drop, cheap = (
        bool(policy & (1 << bit)) for bit in range(1, 6))
    latest = len(case.inputs) - 1
    cancelled = case.cancel != "none" and not ignore_cancel
    if cheap:
        return Execution(Observation("failure", None, latest, 1, len(case.deliveries), 0, False), ())
    # The environment starts delivering only AFTER this setup phase. Splitting
    # or fusing these concrete bookkeeping transitions is therefore a candidate
    # optimization in this bounded model, not a claim about concurrent hosts.
    reserved, published, bookkeeping = set(), set(), []
    for generation in range(len(case.inputs)):
        if generation == latest and case.cancel == "before" and not ignore_cancel:
            continue
        if policy & 1:
            bookkeeping.append(f"reserve_publish:{generation}")
            reserved.add(generation)
            published.add(generation)
        else:
            bookkeeping.append(f"reserve:{generation}")
            reserved.add(generation)
            bookkeeping.append(f"publish:{generation}")
            if generation not in reserved:
                raise ValueError("publishing an unreserved generation")
            published.add(generation)
    spent = len(reserved)
    status, value, generation, completed, ignored = (
        "cancelled" if cancelled else "pending", None, None, 0, 0)
    for delivery in case.deliveries:
        if (cancelled or drop or delivery not in published or (delivery != latest and not stale)
                or (completed > 0 and not duplicate)):
            ignored += 1
            continue
        generation = delivery
        completed += 1
        token = case.inputs[delivery]
        status = "failure" if token == 0 else "success"
        value = token - 1 if token else None
    return Execution(Observation(status, value, generation, completed, ignored, spent,
                     status == "pending" and not drop and latest in published), tuple(bookkeeping))


def simulate(policy: int, case: Case) -> Observation:
    return execute(policy, case).observation


def worker_inventory() -> list[dict]:
    return [{"case": asdict(case), "required": asdict(required(case))} for case in cases()]


def worker_search() -> dict:
    frozen = cases()
    candidates = []
    for policy in range(64):
        rejected = []
        measured_bookkeeping = 0
        for index, case in enumerate(frozen):
            execution, expected = execute(policy, case), required(case)
            observed = execution.observation
            measured_bookkeeping += len(execution.bookkeeping)
            if observed != expected:
                rejected.append({"case_index": index, "case": asdict(case),
                                 "expected": asdict(expected), "observed": asdict(observed)})
        # Deliberately score only after the complete, candidate-independent set.
        candidate = {"policy": policy, "checked_cases": len(frozen),
                     "rejected_cases": len(rejected), "valid": not rejected,
                     "first_counterexample": rejected[0] if rejected else None}
        if not rejected:
            candidate["bookkeeping_steps"] = measured_bookkeeping
        candidates.append(candidate)
    valid = [candidate for candidate in candidates if candidate["valid"]]
    winner = min(valid, key=lambda c: (c["bookkeeping_steps"], c["policy"]))
    return {"assurance": "none", "status": "bounded_computational_evidence",
            "cases": len(frozen), "candidate_case_checks": len(frozen) * 64,
            "valid_policies": [candidate["policy"] for candidate in valid],
            "winner": winner["policy"], "candidates": candidates,
            "score_unit": "executed setup bookkeeping transitions; not measured CPU or host work",
            "environment_phase": "deliveries begin after setup; no between-reserve-and-publish interleaving",
            "claim_scope": "one/two generations; 0..3 deliveries; bounded model only"}


def adjacency(size: int, step: int) -> list[list[bool]]:
    return [[a != b and ((a + step) % size == b or (b + step) % size == a)
             for b in range(size)] for a in range(size)]


def graph_predicate(matrix: list[list[bool]]) -> bool:
    """Connected, simple, undirected, triangle-free, exactly degree two."""
    size = len(matrix)
    if size < 4 or any(len(row) != size for row in matrix):
        return False
    if any(type(entry) is not bool for row in matrix for entry in row):
        return False
    if any(matrix[a][a] or sum(matrix[a]) != 2 for a in range(size)):
        return False
    if any(matrix[a][b] != matrix[b][a] for a in range(size) for b in range(size)):
        return False
    if any(matrix[a][b] and matrix[b][c] and matrix[c][a]
           for a in range(size) for b in range(size) for c in range(size)):
        return False
    reachable = {0}
    for _ in range(size):
        reachable |= {b for a in reachable for b in range(size) if matrix[a][b]}
    return len(reachable) == size


def cycle_isomorphism(size: int, step: int) -> list[int] | None:
    mapping = [(vertex * step) % size for vertex in range(size)]
    graph, cycle = adjacency(size, step), adjacency(size, 1)
    if len(set(mapping)) != size:
        return None
    if not all(cycle[a][b] == graph[mapping[a]][mapping[b]]
               for a in range(size) for b in range(size)):
        return None
    return mapping


def graph_search() -> dict:
    rows = []
    for size in (4, 5, 6, 7):
        for step in range(1, size):
            matrix = adjacency(size, step)
            valid = graph_predicate(matrix)
            rows.append({"size": size, "step": step,
                         "partition": "development" if size < 6 else "size_holdout",
                         "matrix": matrix, "predicate": valid,
                         "cycle_isomorphism": cycle_isomorphism(size, step) if valid else None,
                         "novelty": "known cycle isomorphism" if valid else "no claim"})
    accepted = [row for row in rows if row["predicate"]]
    return {"assurance": "none", "status": "bounded_computational_evidence",
            "candidate_family": "undirected circulants with one step, 1 <= step < size",
            "checked_candidates": len(rows), "accepted_candidate_rows": len(accepted),
            "distinct_labeled_matrices": len({digest(row["matrix"]) for row in accepted}),
            "cycle_isomorphism_classes": len({row["size"] for row in accepted}),
            "novelty_review": "not performed by a mathematical reviewer",
            "parameterized_family_theorem": False, "candidates": rows}


def validate_contract(contract: dict) -> None:
    expected = {
        "schema": "nmlt-discovery-contract-v1", "worker_cases": 441,
        "worker_inventory_sha256": digest(worker_inventory()),
        "worker_policies": list(range(64)),
        "graph_development_sizes": [4, 5], "graph_size_holdouts": [6, 7],
        "graph_candidate_pairs": [[n, k] for n in (4, 5, 6, 7) for k in range(1, n)],
        "graph_predicate": "simple-undirected-connected-triangle-free-degree-two-v1",
        "composition_bits": ["refinement", "wiring", "concreteFormation", "abstractFormation"],
        "contamination": "developer-exposed calibration; size holdout is not a blind benchmark",
    }
    if contract != expected:
        raise ValueError("frozen discovery contract differs from implemented inventory")


def run() -> dict:
    contract = read(CONTRACT)
    validate_contract(contract)
    return {"schema": "nmlt-discovery-evidence-v1", "assurance": "none",
            "execution_identity": source_identity(),
            "contract_sha256": digest(contract), "worker": worker_search(), "graphs": graph_search(),
            "proof_note": "This JSON is not a proof receipt. Lean/NanoDA records are separate."}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = run()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    # Refuse overwrite: partial, old, and fresh runs remain distinguishable.
    with args.output.open("x", encoding="utf-8", newline="\n") as handle:
        json.dump(report, handle, indent=2)
        handle.write("\n")
    print(f"ok: 64 worker policies x 441 cases; {len(report['worker']['valid_policies'])} valid")
    print(f"ok: {report['graphs']['checked_candidates']} graph candidates; "
          f"{report['graphs']['accepted_candidate_rows']} accepted rows; "
          f"{report['graphs']['cycle_isomorphism_classes']} cycle isomorphism classes")
    print(f"evidence (assurance:none): {args.output}")


if __name__ == "__main__":
    main()
