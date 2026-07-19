#!/usr/bin/env python3
"""Reproduce the Phase 6 authority-repair and runtime-drift artifact graph."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

from validate_benchmark_integrity import (
    canonical_json,
    load_json,
    validate_instance,
    validate_schema_definition,
)


ROOT = Path(__file__).resolve().parents[1]
EVALUATION = ROOT / "benchmarks/agentic/evaluation.json"
SUITE = ROOT / "benchmarks/agentic/suite.json"
TEMPORAL = ROOT / "benchmarks/results/temporal/phase4-evidence.json"
OUTPUT = ROOT / "benchmarks/results/agentic/phase6-evidence-graph.json"
SCHEMA = ROOT / "schemas/phase6-evidence-graph-v1.schema.json"
DOMAIN = b"NMLT-PHASE6-ARTIFACT-GRAPH\0v1\0"
PREFIX = "nmlt-phase6-graph-v1:sha256:"


def sha256_id(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def graph_id(value: dict[str, Any]) -> str:
    payload = dict(value)
    payload.pop("graph_id", None)
    encoded = canonical_json(payload)
    return PREFIX + hashlib.sha256(
        DOMAIN + len(encoded).to_bytes(8, "big") + encoded
    ).hexdigest()


def source_set_id() -> str:
    paths = [
        ROOT / "Cargo.toml",
        ROOT / "Cargo.lock",
        ROOT / "rust-toolchain.toml",
        ROOT / "crates/nmlt-agent/Cargo.toml",
        *ROOT.joinpath("crates/nmlt-agent").rglob("*.rs"),
        *ROOT.joinpath("benchmarks/agentic").rglob("*"),
        *ROOT.glob("schemas/agentic-*.schema.json"),
    ]
    entries = []
    for path in sorted({path for path in paths if path.is_file()}):
        entries.append(
            {
                "path": str(path.relative_to(ROOT)),
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            }
        )
    encoded = canonical_json(entries)
    return "nmlt-agent-source-set-v1:sha256:" + hashlib.sha256(
        b"NMLT-AGENT-SOURCE-SET\0v1\0"
        + len(encoded).to_bytes(8, "big")
        + encoded
    ).hexdigest()


def rust_target() -> str:
    process = subprocess.run(
        ["rustc", "--version", "--verbose"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if process.returncode != 0:
        raise ValueError(process.stderr.strip())
    for line in process.stdout.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ")
    raise ValueError("rustc did not report its host target")


def build_and_run_evaluation() -> tuple[dict[str, Any], Path]:
    target = ROOT / "target/evidence-agentic"
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
            "nmlt-agent",
            "--bin",
            "nmlt-agent-evaluate",
        ],
        cwd=ROOT,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
    )
    if build.returncode != 0:
        raise ValueError(f"cannot build agentic evaluation: {build.stderr.strip()}")
    executable = target / "release/nmlt-agent-evaluate"
    if not executable.is_file():
        raise ValueError(f"cargo did not create {executable}")
    first = subprocess.run(
        [str(executable)], cwd=ROOT, text=True, capture_output=True, check=False
    )
    second = subprocess.run(
        [str(executable)], cwd=ROOT, text=True, capture_output=True, check=False
    )
    if first.returncode != 0:
        raise ValueError(first.stderr.strip())
    if second.returncode != 0 or first.stdout != second.stdout:
        raise ValueError("agentic evaluation execution is nondeterministic")
    value = json.loads(first.stdout)
    if not isinstance(value, dict):
        raise ValueError("agentic evaluator must emit an object")
    if value != load_json(EVALUATION):
        raise ValueError("checked-in agentic evaluation differs from current execution")
    return value, executable


def enrich_agent_node(node: dict[str, Any]) -> dict[str, Any]:
    enriched = dict(node)
    role = node["role"]
    task = node["id"].split(":", 1)[1] if ":" in node["id"] else ""
    if role in {"intent", "property", "oracle"}:
        enriched["path"] = f"benchmarks/agentic/trusted/{task}.{role}.txt"
    elif role == "candidate":
        candidate_paths = {
            "held-out-syntax-terminator": "benchmarks/agentic/candidates/held-out-syntax.nmlt",
            "held-out-type-boolean": "benchmarks/agentic/candidates/held-out-type.nmlt",
            "held-out-semantic-authority": "benchmarks/agentic/candidates/held-out-semantic.nmlt",
        }
        if task in candidate_paths:
            enriched["path"] = candidate_paths[task]
    return enriched


def produce() -> dict[str, Any]:
    evaluation, executable = build_and_run_evaluation()
    temporal_check = subprocess.run(
        [sys.executable, str(ROOT / "tools/check_temporal_evidence.py")],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if temporal_check.returncode != 0:
        raise ValueError(
            f"runtime evidence is not current: {temporal_check.stderr.strip()}"
        )
    temporal = load_json(TEMPORAL)
    runtime = temporal["runtime"]
    metrics = evaluation["metrics"]

    nodes = [enrich_agent_node(node) for node in evaluation["artifact_graph"]["nodes"]]
    edges = [dict(edge) for edge in evaluation["artifact_graph"]["edges"]]
    suite_node = "suite:nmlt-agentic-held-out-v1"
    evidence_node = "evidence:nmlt-agentic-held-out-v1"
    model_node = f"runtime-model:{temporal['graph_id']}"
    implementation_node = (
        f"runtime-implementation:{temporal['implementation']['source_set_id']}"
    )
    witness_node = f"runtime-witness:{runtime['contradictory_journal_id']}"
    drift_payload = {
        "case_id": runtime["case_id"],
        "journal_id": runtime["contradictory_journal_id"],
        "model_id": temporal["graph_id"],
        "implementation_id": temporal["implementation"]["source_set_id"],
        "classification": runtime["contradictory_trace"],
        "record_index": runtime["rejected_record_index"],
        "reason": runtime["rejected_reason"],
    }
    drift_digest = sha256_id(canonical_json(drift_payload))
    drift_node = f"runtime-drift:{drift_digest}"
    gaps = [
        "The deterministic assistant is a protocol baseline, not an LLM evaluation.",
        "Three hand-authored held-out tasks do not establish statistical generalization.",
        "The agentic checker is a narrow fixture checker, not the complete NMLT compiler.",
        "The runtime journal is synthetic and has no authenticity or completeness attestation.",
        "This graph links exact artifacts but does not prove a deployed implementation refines the model.",
    ]
    gaps_digest = sha256_id(canonical_json(gaps))
    gaps_node = f"residual-gaps:{gaps_digest}"

    nodes.extend(
        [
            {
                "id": suite_node,
                "role": "suite",
                "digest": sha256_id(SUITE.read_bytes()),
                "summary": "frozen held-out authority and repair task suite",
                "result_class": None,
                "path": str(SUITE.relative_to(ROOT)),
            },
            {
                "id": evidence_node,
                "role": "evidence",
                "digest": sha256_id(EVALUATION.read_bytes()),
                "summary": "deterministic authority-bounded repair evaluation",
                "result_class": "tested",
                "path": str(EVALUATION.relative_to(ROOT)),
            },
            {
                "id": model_node,
                "role": "model",
                "digest": "sha256:" + temporal["graph_id"].rsplit(":", 1)[1],
                "summary": "finite temporal model used to classify the runtime journal",
                "result_class": "model_checked",
                "path": str(TEMPORAL.relative_to(ROOT)),
                "locator": "$.graph",
                "binding_id": temporal["graph_id"],
            },
            {
                "id": implementation_node,
                "role": "implementation",
                "digest": "sha256:"
                + temporal["implementation"]["source_set_id"].rsplit(":", 1)[1],
                "summary": "exact temporal checker/runtime-adapter implementation source set",
                "result_class": None,
                "path": str(TEMPORAL.relative_to(ROOT)),
                "locator": "$.implementation",
                "binding_id": temporal["implementation"]["source_set_id"],
            },
            {
                "id": witness_node,
                "role": "witness",
                "digest": sha256_id(canonical_json(runtime["contradictory_journal"])),
                "summary": "contradictory journal localized at its first impossible transition",
                "result_class": "refuted",
                "path": str(TEMPORAL.relative_to(ROOT)),
                "locator": "$.runtime.contradictory_journal",
                "binding_id": runtime["contradictory_journal_id"],
            },
            {
                "id": drift_node,
                "role": "drift_event",
                "digest": drift_digest,
                "summary": runtime["rejected_reason"],
                "result_class": "rejected",
                "path": str(TEMPORAL.relative_to(ROOT)),
                "locator": "$.runtime",
                "binding_id": runtime["case_id"],
            },
            {
                "id": gaps_node,
                "role": "residual_gaps",
                "digest": gaps_digest,
                "summary": "explicit limitations of the Phase 6 combined claim",
                "result_class": None,
                "locator": "$.residual_gaps",
            },
        ]
    )
    for task in evaluation["tasks"]:
        edges.append(
            {
                "from": evidence_node,
                "relation": "contains",
                "to": f"evaluation:{task['task_id']}",
            }
        )
    edges.extend(
        [
            {"from": suite_node, "relation": "evaluated_as", "to": evidence_node},
            {"from": model_node, "relation": "classifies", "to": drift_node},
            {
                "from": implementation_node,
                "relation": "produced",
                "to": witness_node,
            },
            {"from": witness_node, "relation": "supports", "to": drift_node},
            {"from": drift_node, "relation": "qualified_by", "to": gaps_node},
            {"from": evidence_node, "relation": "qualified_by", "to": gaps_node},
        ]
    )
    nodes.sort(key=lambda node: node["id"].encode("utf-8"))
    edges.sort(key=lambda edge: (edge["from"], edge["relation"], edge["to"]))

    value: dict[str, Any] = {
        "schema_version": "1.0.0",
        "graph_id": "",
        "fixture": "authority-repair-and-runtime-drift-v1",
        "evaluation_binding": {
            "path": str(EVALUATION.relative_to(ROOT)),
            "content_sha256": sha256_id(EVALUATION.read_bytes()),
            "suite_path": str(SUITE.relative_to(ROOT)),
            "suite_sha256": sha256_id(SUITE.read_bytes()),
            "assistant_kind": evaluation["assistant"]["kind"],
            "checker_id": evaluation["checker_id"],
        },
        "runtime_binding": {
            "path": str(TEMPORAL.relative_to(ROOT)),
            "content_sha256": sha256_id(TEMPORAL.read_bytes()),
            "model_id": temporal["graph_id"],
            "implementation_id": temporal["implementation"]["source_set_id"],
            "runtime_case_id": runtime["case_id"],
            "journal_id": runtime["contradictory_journal_id"],
            "classification": runtime["contradictory_trace"],
            "record_index": runtime["rejected_record_index"],
            "reason": runtime["rejected_reason"],
        },
        "metrics": {
            "tasks": metrics["task_count"],
            "baseline_completed": metrics["baseline_completed"],
            "assisted_completed": metrics["assisted_completed"],
            "integrity_attempts": metrics["trusted_modification_attempts"],
            "integrity_rejections": metrics["trusted_modification_rejections"],
            "controls_retained": metrics["negative_controls_retained"],
            "controls_killed": metrics["negative_controls_killed"],
            "unknown_promotions": metrics["unknown_results_promoted"],
            "conflict_promotions": metrics["conflict_results_promoted"],
        },
        "nodes": nodes,
        "edges": edges,
        "implementation": {
            "source_set_id": source_set_id(),
            "toolchain": subprocess.run(
                ["rustc", "--version"],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=True,
            ).stdout.strip(),
            "target": rust_target(),
            "executable_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
            "reproducer_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        },
        "residual_gaps": gaps,
    }
    value["graph_id"] = graph_id(value)
    return value


def semantic_errors(value: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if value.get("graph_id") != graph_id(value):
        errors.append("canonical Phase 6 graph identity is stale")
    nodes = value.get("nodes", [])
    node_ids = [node.get("id") for node in nodes if isinstance(node, dict)]
    if len(node_ids) != len(set(node_ids)):
        errors.append("artifact graph has duplicate node IDs")
    node_set = set(node_ids)
    for edge in value.get("edges", []):
        if edge.get("from") not in node_set or edge.get("to") not in node_set:
            errors.append("artifact graph edge references an absent node")
    runtime = value.get("runtime_binding", {})
    current_temporal = load_json(TEMPORAL)
    current_runtime = current_temporal["runtime"]
    expected_runtime = {
        "path": str(TEMPORAL.relative_to(ROOT)),
        "content_sha256": sha256_id(TEMPORAL.read_bytes()),
        "model_id": current_temporal["graph_id"],
        "implementation_id": current_temporal["implementation"]["source_set_id"],
        "runtime_case_id": current_runtime["case_id"],
        "journal_id": current_runtime["contradictory_journal_id"],
        "classification": current_runtime["contradictory_trace"],
        "record_index": current_runtime["rejected_record_index"],
        "reason": current_runtime["rejected_reason"],
    }
    if runtime != expected_runtime:
        errors.append("runtime drift binding differs from the current exact temporal evidence")
    required_roles = {"intent", "evidence", "witness", "model", "implementation", "drift_event", "residual_gaps"}
    roles = {node.get("role") for node in nodes if isinstance(node, dict)}
    if not required_roles.issubset(roles):
        errors.append("one graph does not expose intent, evidence, witness, model, implementation, drift, and gaps")
    drift_nodes = [node for node in nodes if node.get("role") == "drift_event"]
    if len(drift_nodes) != 1 or drift_nodes[0].get("result_class") != "rejected":
        errors.append("runtime drift is not classified as rejected")
    witness_nodes = [node for node in nodes if node.get("role") == "witness"]
    if len(witness_nodes) != 1 or witness_nodes[0].get("binding_id") != runtime.get("journal_id"):
        errors.append("runtime witness is not bound to the exact journal")
    model_nodes = [node for node in nodes if node.get("role") == "model"]
    if len(model_nodes) != 1 or model_nodes[0].get("binding_id") != runtime.get("model_id"):
        errors.append("runtime drift is not bound to the exact model node")
    implementation_nodes = [
        node for node in nodes if node.get("role") == "implementation"
    ]
    if (
        len(implementation_nodes) != 1
        or implementation_nodes[0].get("binding_id")
        != runtime.get("implementation_id")
    ):
        errors.append("runtime drift is not bound to the exact implementation node")
    if value.get("evaluation_binding", {}).get("assistant_kind") != (
        "deterministic_protocol_conformance_baseline_not_llm_evidence"
    ):
        errors.append("protocol baseline was mislabeled as agent generalization evidence")
    if value.get("implementation", {}).get("source_set_id") != source_set_id():
        errors.append("agentic implementation source set is stale")
    return errors


def negative_self_tests(value: dict[str, Any], schema: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    mutations = []
    mislabeled = copy.deepcopy(value)
    mislabeled["evaluation_binding"]["assistant_kind"] = "llm-generalization-evidence"
    mislabeled["graph_id"] = graph_id(mislabeled)
    mutations.append(("assistant relabeling", mislabeled))
    accepted_drift = copy.deepcopy(value)
    accepted_drift["runtime_binding"]["classification"] = "accepted"
    accepted_drift["graph_id"] = graph_id(accepted_drift)
    mutations.append(("accepted contradictory drift", accepted_drift))
    stale_model = copy.deepcopy(value)
    stale_model["runtime_binding"]["model_id"] = (
        "nmlt-temporal-graph-v1:sha256:" + "0" * 64
    )
    stale_model["graph_id"] = graph_id(stale_model)
    mutations.append(("stale model binding", stale_model))
    missing_witness = copy.deepcopy(value)
    missing_witness["nodes"] = [
        node for node in missing_witness["nodes"] if node["role"] != "witness"
    ]
    missing_witness["edges"] = [
        edge
        for edge in missing_witness["edges"]
        if not edge["from"].startswith("runtime-witness:")
        and not edge["to"].startswith("runtime-witness:")
    ]
    missing_witness["graph_id"] = graph_id(missing_witness)
    mutations.append(("missing runtime witness", missing_witness))
    for label, mutation in mutations:
        errors = validate_instance(mutation, schema, schema)
        errors.extend(semantic_errors(mutation))
        if not errors:
            failures.append(f"negative control was accepted: {label}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--update", action="store_true")
    args = parser.parse_args()
    try:
        expected = produce()
        schema = load_json(SCHEMA)
        errors = validate_schema_definition(schema, "phase6-evidence-graph")
        errors.extend(validate_instance(expected, schema, schema))
        errors.extend(semantic_errors(expected))
        errors.extend(negative_self_tests(expected, schema))
        if errors:
            raise ValueError("; ".join(errors))
        if args.update:
            OUTPUT.parent.mkdir(parents=True, exist_ok=True)
            OUTPUT.write_text(json.dumps(expected, indent=2) + "\n", encoding="utf-8")
        actual = load_json(OUTPUT)
        if actual != expected:
            raise ValueError("persisted Phase 6 graph differs from current exact artifacts")
    except (OSError, ValueError, json.JSONDecodeError, KeyError, TypeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print("ok: authority-bounded repair and runtime drift share one exact artifact graph")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
