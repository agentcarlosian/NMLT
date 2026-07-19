#!/usr/bin/env python3
"""Reproduce and schema-check the Phase 4 executable evidence fixture."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

from validate_benchmark_integrity import (
    artifact_id,
    benchmark_result_id,
    canonical_json,
    load_json,
    source_id,
    validate_instance,
    validate_schema_definition,
)
from check_model_reports import engine_source_set_id


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "benchmarks/results/temporal/phase4-evidence.json"
SCHEMA = ROOT / "schemas/temporal-evidence-v1.schema.json"
GRAPH_SPEC = {
    "semantics": "finite-graph-v1+universal-identity-stutter-v1",
    "states": [{"done": False, "internal": 0}, {"done": True, "internal": 0}],
    "initial": [0],
    "transitions": [
        {"from": 0, "action": "idle", "to": 0},
        {"from": 0, "action": "work", "to": 1},
    ],
}
PREDICATE_SPEC = {
    "language": "nmlt-fixture-predicate-v1",
    "formula": "eventually(done)",
    "state_test": {"field": "done", "equals": True},
}
UNFAIR_SET = []
WEAK_WORK_SET = [{"kind": "weak", "action": "work"}]
REFINEMENT_SPEC = {
    "concrete": {
        "states": [
            {"done": False, "internal": 0},
            {"done": False, "internal": 1},
            {"done": True, "internal": 1},
        ],
        "initial": [0],
        "transitions": [
            {"from": 0, "action": "cache", "to": 1},
            {"from": 1, "action": "publish", "to": 2},
        ],
    },
    "abstract": {
        "states": [
            {"done": False, "internal": 99},
            {"done": True, "internal": 99},
        ],
        "initial": [0],
        "transitions": [{"from": 0, "action": "commit", "to": 1}],
    },
    "state_map": [0, 0, 1],
    "concrete_observation": {"done": "done"},
    "abstract_observation": {"done": "done"},
    "action_map": {"cache": None, "publish": "commit"},
}
RUNTIME_SPEC = {
    "graph": GRAPH_SPEC,
    "mapping": {"done": "done"},
    "accepted_journal": [
        {"sequence": 40, "action": "initial", "observations": {"done": False}},
        {"sequence": 41, "action": "work", "observations": {"done": True}},
    ],
    "contradictory_journal": [
        {"sequence": 40, "action": "initial", "observations": {"done": False}},
        {"sequence": 41, "action": "work", "observations": {"done": False}},
    ],
}
PROVIDER_REFERENCE_GRAPH_SPEC = {
    "semantics": "finite-observation-quotient+universal-identity-stutter-v1",
    "states": [
        {"phase": "proposed", "dispatch_enabled": False},
        {"phase": "authorized", "dispatch_enabled": False},
        {"phase": "authorized", "dispatch_enabled": True},
        {"phase": "dispatched", "dispatch_enabled": False},
        {"phase": "responded", "dispatch_enabled": False},
        {"phase": "indeterminate", "dispatch_enabled": False},
        {"phase": "evaluated", "dispatch_enabled": False},
        {"phase": "evaluated", "dispatch_enabled": False},
        {"phase": "selected", "dispatch_enabled": False},
    ],
    "initial": [0],
    "transitions": [
        {"from": 0, "action": "authorize", "to": 1},
        {"from": 1, "action": "arm", "to": 2},
        {"from": 2, "action": "dispatch", "to": 3},
        {"from": 3, "action": "receive_response", "to": 4},
        {"from": 3, "action": "lose_response", "to": 5},
        {"from": 4, "action": "evaluate_fail", "to": 7},
        {"from": 4, "action": "evaluate_pass", "to": 6},
        {"from": 6, "action": "select", "to": 8},
    ],
}
PROVIDER_MUTANT_GRAPH_SPEC = {
    "semantics": "finite-observation-quotient+universal-identity-stutter-v1",
    "states": [{"phase": "indeterminate", "dispatch_enabled": True}],
    "initial": [0],
    "transitions": [{"from": 0, "action": "dispatch", "to": 0}],
}
PROVIDER_PROJECTION_SPEC = {
    "visible": ["phase", "enabled(dispatch)"],
    "hidden": [
        "armed",
        "dispatched",
        "dispatch_count",
        "response_intact",
        "passed",
    ],
    "enabledness_rule": "dispatch_enabled iff an outgoing dispatch action exists",
    "mutant_counter_abstraction": "hide dispatch_count; preserve phase and dispatch enabledness",
    "construction": "manual property-relevant observation graph; distinct hidden states retain distinct state IDs",
}
PROPERTY = ROOT / "benchmarks/provider-attempt/properties/no-blind-replay.json"
PROVIDER_RESULTS = ROOT / "benchmarks/results/provider-attempt"


def closed_transitions(graph: dict) -> list[dict]:
    transitions = [dict(item) for item in graph["transitions"]]
    transitions.extend(
        {"from": state, "action": None, "to": state}
        for state in range(len(graph["states"]))
    )
    return sorted(
        transitions,
        key=lambda edge: (
            edge["from"],
            edge["to"],
            1 if edge["action"] is None else 0,
            edge["action"] or "",
        ),
    )


def domain_id(domain: bytes, prefix: str, value: object) -> str:
    encoded = canonical_json(value)
    return prefix + hashlib.sha256(
        domain + len(encoded).to_bytes(8, "big") + encoded
    ).hexdigest()


def evidence_id(value: dict) -> str:
    payload = dict(value)
    payload.pop("evidence_id", None)
    return domain_id(
        b"NMLT-TEMPORAL-EVIDENCE\0v1\0",
        "nmlt-temporal-evidence-v1:sha256:",
        payload,
    )


def mapping_id(value: object) -> str:
    return domain_id(
        b"NMLT-OBSERVATION-ACTION-MAPPING\0v1\0",
        "nmlt-mapping-v1:sha256:",
        value,
    )


def replay_lasso(value: dict) -> None:
    witness = value["temporal"]["without_fairness"]
    transitions = closed_transitions(GRAPH_SPEC)
    stem_states = witness["stem_states"]
    stem_transitions = witness["stem_transitions"]
    loop_states = witness["loop_states"]
    loop_transitions = witness["loop_transitions"]
    if (
        not stem_states
        or len(stem_states) != len(stem_transitions) + 1
        or len(loop_states) < 2
        or len(loop_states) != len(loop_transitions) + 1
        or stem_states[0] not in GRAPH_SPEC["initial"]
        or stem_states[-1] != loop_states[0]
        or loop_states[0] != loop_states[-1]
    ):
        raise ValueError("independent replay rejected malformed lasso shape")

    def replay_path(states: list[int], transition_ids: list[int]) -> None:
        for index, transition_id in enumerate(transition_ids):
            if transition_id < 0 or transition_id >= len(transitions):
                raise ValueError("lasso names an out-of-range transition")
            edge = transitions[transition_id]
            if edge["from"] != states[index] or edge["to"] != states[index + 1]:
                raise ValueError("lasso transition endpoints do not match its states")

    replay_path(stem_states, stem_transitions)
    replay_path(loop_states, loop_transitions)
    if any(GRAPH_SPEC["states"][state]["done"] for state in stem_states + loop_states):
        raise ValueError("eventuality lasso visits the goal predicate")


def replay_weak_fair_holds() -> None:
    transitions = closed_transitions(GRAPH_SPEC)
    avoiding = {
        index for index, state in enumerate(GRAPH_SPEC["states"]) if not state["done"]
    }
    if avoiding != {0}:
        raise ValueError("fixture-specific fair replay expects one avoiding state")
    enabled_work = any(
        edge["from"] == 0 and edge["action"] == "work" for edge in transitions
    )
    internal_work = any(
        edge["from"] in avoiding
        and edge["to"] in avoiding
        and edge["action"] == "work"
        for edge in transitions
    )
    cyclic = any(
        edge["from"] == 0 and edge["to"] == 0 for edge in transitions
    )
    if not cyclic or not enabled_work or internal_work:
        raise ValueError("weak-fair eventuality replay no longer excludes every avoiding loop")


def replay_refinement(value: dict) -> None:
    report = value["refinement"]
    spec = REFINEMENT_SPEC
    concrete = spec["concrete"]
    abstract = spec["abstract"]
    state_map = spec["state_map"]
    if len(state_map) != len(concrete["states"]):
        raise ValueError("refinement map does not cover every concrete state")
    if any(state_map[state] not in abstract["initial"] for state in concrete["initial"]):
        raise ValueError("refinement map violates the initial-state obligation")
    for concrete_state, abstract_state in enumerate(state_map):
        concrete_done = concrete["states"][concrete_state]["done"]
        abstract_done = abstract["states"][abstract_state]["done"]
        if concrete_done != abstract_done:
            raise ValueError("refinement map violates observation equality")
    for edge in concrete["transitions"]:
        mapped_action = spec["action_map"].get(edge["action"], "missing")
        source = state_map[edge["from"]]
        target = state_map[edge["to"]]
        if mapped_action == "missing":
            raise ValueError("refinement action mapping is incomplete")
        if mapped_action is None:
            if source != target:
                raise ValueError("hidden refinement step changes abstract state")
        elif not any(
            candidate["from"] == source
            and candidate["to"] == target
            and candidate["action"] == mapped_action
            for candidate in abstract["transitions"]
        ):
            raise ValueError("visible refinement step has no abstract match")
    if report["checked_states"] != len(concrete["states"]) or report[
        "checked_transitions"
    ] != len(concrete["transitions"]):
        raise ValueError("refinement report coverage differs from independently replayed spec")


def replay_journal(journal: list[dict]) -> tuple[str, int | None, list[int], str | None]:
    transitions = closed_transitions(GRAPH_SPEC)
    if not journal:
        return "rejected", None, [], "empty journal"
    first = journal[0]
    if first["action"] != "initial":
        return "rejected", 0, [], "first record is not initial"

    def observations_match(state: int, record: dict) -> bool:
        return all(
            GRAPH_SPEC["states"][state].get(model_field)
            == record["observations"].get(journal_field)
            for model_field, journal_field in RUNTIME_SPEC["mapping"].items()
        )

    candidates = {
        state
        for state in GRAPH_SPEC["initial"]
        if observations_match(state, first)
    }
    if not candidates:
        return "rejected", 0, [], "initial observations match no state"
    for index, record in enumerate(journal[1:], start=1):
        previous = journal[index - 1]["sequence"]
        if previous >= 2**64 - 1 or record["sequence"] != previous + 1:
            return "rejected", index, sorted(candidates), "nonconsecutive sequence"
        next_states = {
            edge["to"]
            for edge in transitions
            if edge["from"] in candidates
            and edge["action"] == record["action"]
            and observations_match(edge["to"], record)
        }
        if not next_states:
            return "rejected", index, sorted(candidates), "no matching transition"
        candidates = next_states
    return "accepted", None, sorted(candidates), None


def graph_reachable(graph: dict) -> set[int]:
    transitions = closed_transitions(graph)
    reached = set(graph["initial"])
    frontier = list(sorted(reached))
    while frontier:
        state = frontier.pop(0)
        for edge in transitions:
            if edge["from"] == state and edge["to"] not in reached:
                reached.add(edge["to"])
                frontier.append(edge["to"])
    return reached


def replay_graph_lasso(graph: dict, witness: dict) -> None:
    transitions = closed_transitions(graph)
    stem_states = witness["stem_states"]
    stem_transitions = witness["stem_transitions"]
    loop_states = witness["loop_states"]
    loop_transitions = witness["loop_transitions"]
    if (
        not stem_states
        or len(stem_states) != len(stem_transitions) + 1
        or len(loop_states) < 2
        or len(loop_states) != len(loop_transitions) + 1
        or stem_states[0] not in graph["initial"]
        or stem_states[-1] != loop_states[0]
        or loop_states[0] != loop_states[-1]
    ):
        raise ValueError("provider temporal lasso shape is malformed")
    for states, transition_ids in (
        (stem_states, stem_transitions),
        (loop_states, loop_transitions),
    ):
        for index, transition_id in enumerate(transition_ids):
            if transition_id < 0 or transition_id >= len(transitions):
                raise ValueError("provider lasso transition is out of range")
            edge = transitions[transition_id]
            if edge["from"] != states[index] or edge["to"] != states[index + 1]:
                raise ValueError("provider lasso transition endpoints do not replay")


def replay_provider_temporal(value: dict) -> None:
    def check_enabled_projection(graph: dict) -> None:
        for state_index, state in enumerate(graph["states"]):
            derived = any(
                edge["from"] == state_index and edge["action"] == "dispatch"
                for edge in graph["transitions"]
            )
            if state["dispatch_enabled"] != derived:
                raise ValueError("provider projection does not preserve dispatch enabledness")

    def property_holds(state: dict) -> bool:
        return state["phase"] != "indeterminate" or not state["dispatch_enabled"]

    reference = value["reference_temporal"]
    mutant = value["mutant_temporal"]
    check_enabled_projection(PROVIDER_REFERENCE_GRAPH_SPEC)
    check_enabled_projection(PROVIDER_MUTANT_GRAPH_SPEC)
    reachable = graph_reachable(PROVIDER_REFERENCE_GRAPH_SPEC)
    if reference != {"result": "model_checked", "explored_states": len(reachable)}:
        raise ValueError("provider reference temporal coverage changed")
    if any(
        not property_holds(PROVIDER_REFERENCE_GRAPH_SPEC["states"][state])
        for state in reachable
    ):
        raise ValueError("provider reference projection violates NoBlindReplay")
    if mutant.get("result") != "refuted":
        raise ValueError("provider blind mutant was not temporally refuted")
    replay_graph_lasso(PROVIDER_MUTANT_GRAPH_SPEC, mutant)
    witnessed_states = mutant["stem_states"] + mutant["loop_states"]
    if not any(
        not property_holds(PROVIDER_MUTANT_GRAPH_SPEC["states"][state])
        for state in witnessed_states
    ):
        raise ValueError("provider temporal lasso contains no property violation")


def independent_replay(value: dict) -> dict:
    replay_lasso(value)
    replay_weak_fair_holds()
    replay_refinement(value)
    replay_provider_temporal(value["provider_blind_replay"])
    accepted = replay_journal(RUNTIME_SPEC["accepted_journal"])
    rejected = replay_journal(RUNTIME_SPEC["contradictory_journal"])
    if accepted[0] != value["runtime"]["accepted_trace"]:
        raise ValueError("independent replay disagrees on accepted runtime journal")
    if (
        rejected[0] != value["runtime"]["contradictory_trace"]
        or rejected[1] != value["runtime"]["rejected_record_index"]
        or rejected[2] != value["runtime"]["rejected_candidates_before"]
    ):
        raise ValueError("independent replay disagrees on contradictory runtime journal")
    return {
        "replayer": "nmlt-python-fixture-replay-v1",
        "lasso": True,
        "weak_fairness": True,
        "refinement": True,
        "accepted_journal": True,
        "contradictory_journal": True,
        "provider_no_blind_replay": True,
    }


def provider_blind_replay(temporal_output: dict) -> dict:
    property_document = load_json(PROPERTY)
    if property_document.get("property_id") != artifact_id(
        property_document, "property_id"
    ):
        raise ValueError("NoBlindReplay property identity is stale")

    def checked_result(filename: str, case_id: str, classification: str) -> tuple[dict, dict]:
        path = PROVIDER_RESULTS / filename
        result = load_json(path)
        if result.get("result_id") != benchmark_result_id(result):
            raise ValueError(f"{case_id} has a stale result identity")
        source_path = ROOT / result["source"]["path"]
        if result["source"].get("source_id") != source_id(source_path):
            raise ValueError(f"{case_id} has a stale source identity")
        if result.get("engine", {}).get("source_set_id") != engine_source_set_id():
            raise ValueError(f"{case_id} has a stale engine source-set identity")
        properties = [
            item
            for item in result.get("report", {}).get("properties", [])
            if item.get("property") == "NoBlindReplay"
        ]
        if len(properties) != 1 or properties[0].get("result") != classification:
            raise ValueError(
                f"{case_id} did not classify NoBlindReplay as {classification}"
            )
        binding = {
            "case_id": case_id,
            "result_path": str(path.relative_to(ROOT)),
            "source_id": result["source"]["source_id"],
            "result_id": result["result_id"],
            "classification": classification,
        }
        return binding, properties[0]

    reference, _ = checked_result(
        "provider-attempt-reference.json", "provider-attempt-reference", "model_checked"
    )
    mutant, mutant_property = checked_result(
        "blind-replay.json", "blind-replay", "refuted"
    )
    witness = mutant_property.get("witness")
    if not isinstance(witness, dict):
        raise ValueError("blind-replay refutation lacks a structured witness")
    steps = witness.get("steps", [])
    if (
        len(steps) != 1
        or steps[0].get("action") is not None
        or "dispatch" not in steps[0].get("enabled_actions", [])
    ):
        raise ValueError(
            "blind-replay witness must localize enabled dispatch in the initial state"
        )
    if temporal_output.get("semantics") != PROVIDER_REFERENCE_GRAPH_SPEC["semantics"]:
        raise ValueError("provider temporal checker semantics changed")
    reference_temporal = temporal_output.get("reference")
    mutant_temporal = temporal_output.get("mutant")
    if not isinstance(reference_temporal, dict) or not isinstance(mutant_temporal, dict):
        raise ValueError("provider temporal checker omitted reference or mutant output")
    mutant["witness"] = witness
    return {
        "property_id": property_document["property_id"],
        "formula": property_document["nmlt_formula"],
        "semantics": temporal_output["semantics"],
        "projection_id": mapping_id(PROVIDER_PROJECTION_SPEC),
        "projection": PROVIDER_PROJECTION_SPEC,
        "reference_graph_id": domain_id(
            b"NMLT-TEMPORAL-GRAPH\0v1\0",
            "nmlt-temporal-graph-v1:sha256:",
            PROVIDER_REFERENCE_GRAPH_SPEC,
        ),
        "reference_graph": PROVIDER_REFERENCE_GRAPH_SPEC,
        "mutant_graph_id": domain_id(
            b"NMLT-TEMPORAL-GRAPH\0v1\0",
            "nmlt-temporal-graph-v1:sha256:",
            PROVIDER_MUTANT_GRAPH_SPEC,
        ),
        "mutant_graph": PROVIDER_MUTANT_GRAPH_SPEC,
        "reference": reference,
        "mutant": mutant,
        "reference_temporal": reference_temporal,
        "mutant_temporal": mutant_temporal,
    }


def source_set_id() -> str:
    crate = ROOT / "crates/nmlt-temporal"
    paths = [crate / "Cargo.toml", *crate.rglob("*.rs")]
    entries = []
    for path in sorted(paths):
        data = path.read_bytes()
        entries.append(
            {
                "path": str(path.relative_to(ROOT)),
                "sha256": hashlib.sha256(data).hexdigest(),
            }
        )
    return domain_id(
        b"NMLT-TEMPORAL-SOURCE-SET\0v1\0",
        "nmlt-temporal-source-set-v1:sha256:",
        entries,
    )


def run() -> dict:
    target = ROOT / "target/evidence-temporal"
    environment = os.environ.copy()
    environment.update(
        {
            "CARGO_INCREMENTAL": "0",
            "CARGO_TARGET_DIR": str(target),
            "SOURCE_DATE_EPOCH": "0",
            "RUSTFLAGS": f"--remap-path-prefix={ROOT}=/nmlt -Cdebuginfo=0",
        }
    )
    build = subprocess.run(
        [
            "cargo",
            "build",
            "--quiet",
            "--release",
            "-p",
            "nmlt-temporal",
            "--example",
            "phase4_evidence",
        ],
        cwd=ROOT,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
    )
    if build.returncode != 0:
        raise ValueError(build.stderr.strip())
    executable = target / "release/examples/phase4_evidence"
    if not executable.is_file():
        raise ValueError(f"cargo build did not create {executable}")
    command = [str(executable)]
    first = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
    second = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
    if first.returncode != 0:
        raise ValueError(first.stderr.strip())
    if second.returncode != 0 or second.stdout != first.stdout:
        raise ValueError("temporal evidence execution is nondeterministic")
    value = json.loads(first.stdout)
    if not isinstance(value, dict):
        raise ValueError("temporal evidence must be an object")
    graph_id = domain_id(
        b"NMLT-TEMPORAL-GRAPH\0v1\0",
        "nmlt-temporal-graph-v1:sha256:",
        GRAPH_SPEC,
    )
    if value.get("graph_id") != graph_id:
        raise ValueError("temporal fixture graph identity is stale")
    value["graph"] = GRAPH_SPEC
    value["temporal"]["predicate_id"] = domain_id(
        b"NMLT-TEMPORAL-PREDICATE\0v1\0",
        "nmlt-temporal-predicate-v1:sha256:",
        PREDICATE_SPEC,
    )
    value["temporal"]["predicate"] = PREDICATE_SPEC
    value["temporal"]["without_fairness"]["fairness_set_id"] = domain_id(
        b"NMLT-FAIRNESS-SET\0v1\0",
        "nmlt-fairness-set-v1:sha256:",
        UNFAIR_SET,
    )
    value["temporal"]["with_fairness"]["fairness_set_id"] = domain_id(
        b"NMLT-FAIRNESS-SET\0v1\0",
        "nmlt-fairness-set-v1:sha256:",
        WEAK_WORK_SET,
    )
    value["refinement"]["refinement_id"] = domain_id(
        b"NMLT-REFINEMENT-FIXTURE\0v1\0",
        "nmlt-refinement-v1:sha256:",
        REFINEMENT_SPEC,
    )
    value["refinement"]["mapping_id"] = mapping_id(
        {
            "state_map": REFINEMENT_SPEC["state_map"],
            "concrete_observation": REFINEMENT_SPEC["concrete_observation"],
            "abstract_observation": REFINEMENT_SPEC["abstract_observation"],
            "action_map": REFINEMENT_SPEC["action_map"],
        }
    )
    value["refinement"]["spec"] = REFINEMENT_SPEC
    runtime_case_id = domain_id(
        b"NMLT-RUNTIME-CASE\0v1\0",
        "nmlt-runtime-case-v1:sha256:",
        RUNTIME_SPEC,
    )
    if value.get("runtime", {}).get("case_id") != "":
        raise ValueError("runtime fixture must leave canonical identity to the reproducer")
    value["runtime"]["case_id"] = runtime_case_id
    value["runtime"]["mapping_id"] = mapping_id(RUNTIME_SPEC["mapping"])
    value["runtime"]["accepted_journal_id"] = domain_id(
        b"NMLT-RUNTIME-JOURNAL\0v1\0",
        "nmlt-journal-v1:sha256:",
        RUNTIME_SPEC["accepted_journal"],
    )
    value["runtime"]["contradictory_journal_id"] = domain_id(
        b"NMLT-RUNTIME-JOURNAL\0v1\0",
        "nmlt-journal-v1:sha256:",
        RUNTIME_SPEC["contradictory_journal"],
    )
    temporal_output = value.pop("provider_temporal", None)
    if not isinstance(temporal_output, dict):
        raise ValueError("Phase 4 executable omitted provider temporal output")
    value["provider_blind_replay"] = provider_blind_replay(temporal_output)
    value["independent_replay"] = independent_replay(value)
    toolchain = subprocess.run(
        ["rustc", "--version"], cwd=ROOT, text=True, capture_output=True, check=True
    ).stdout.strip()
    value["implementation"] = {
        "checker_version": "0.0.1",
        "source_set_id": source_set_id(),
        "toolchain": toolchain,
        "executable_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
        "replayer_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    }
    value["evidence_id"] = evidence_id(value)
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--update", action="store_true")
    args = parser.parse_args()
    try:
        expected = run()
        schema = load_json(SCHEMA)
        errors = validate_schema_definition(schema, "temporal-evidence")
        errors.extend(validate_instance(expected, schema, schema))
        if errors:
            raise ValueError("; ".join(errors))
        if args.update:
            OUTPUT.parent.mkdir(parents=True, exist_ok=True)
            OUTPUT.write_text(json.dumps(expected, indent=2) + "\n", encoding="utf-8")
        actual = load_json(OUTPUT)
        if actual != expected:
            raise ValueError("persisted Phase 4 evidence differs from current execution")
    except (OSError, ValueError, json.JSONDecodeError, KeyError, TypeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print("ok: temporal lasso/fairness, hidden-state refinement, and runtime trace evidence")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
