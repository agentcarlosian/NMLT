#!/usr/bin/env python3
"""Reproduce and validate the Phase 5 two-engine provider evidence."""

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
    artifact_id,
    canonical_json,
    load_json,
    source_id,
    validate_instance,
    validate_schema_definition,
)


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "benchmarks/results/multi-engine/provider-dispatch.json"
SCHEMA = ROOT / "schemas/multi-engine-evidence-v1.schema.json"
SOURCE = ROOT / "benchmarks/seeded-defects/provider-attempt/reference.nmlt"
PROPERTY = ROOT / "benchmarks/provider-attempt/properties/dispatch-requires-arm.json"
DOMAIN = b"NMLT-MULTI-ENGINE-EVIDENCE\0v1\0"
PREFIX = "nmlt-multi-engine-evidence-v1:sha256:"
SOURCE_SET_DOMAIN = b"NMLT-VERIFY-SOURCE-SET\0v1\0"
SOURCE_SET_PREFIX = "nmlt-verify-source-set-v1:sha256:"


def content_sha256(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def evidence_id(value: dict[str, Any]) -> str:
    payload = dict(value)
    payload.pop("evidence_id", None)
    encoded = canonical_json(payload)
    return PREFIX + hashlib.sha256(
        DOMAIN + len(encoded).to_bytes(8, "big") + encoded
    ).hexdigest()


def source_set_id() -> str:
    crate = ROOT / "crates/nmlt-verify"
    paths = [
        ROOT / "Cargo.toml",
        ROOT / "Cargo.lock",
        ROOT / "rust-toolchain.toml",
        crate / "Cargo.toml",
        *crate.rglob("*.rs"),
    ]
    entries = []
    for path in sorted(set(paths)):
        entries.append(
            {
                "path": str(path.relative_to(ROOT)),
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            }
        )
    encoded = canonical_json(entries)
    return SOURCE_SET_PREFIX + hashlib.sha256(
        SOURCE_SET_DOMAIN + len(encoded).to_bytes(8, "big") + encoded
    ).hexdigest()


def embedded_library_digest() -> str:
    names = [
        "certificate.rs",
        "evidence.rs",
        "identity.rs",
        "inductive.rs",
        "ir.rs",
        "proof.rs",
        "reachability.rs",
        "smt.rs",
        "test_hook.rs",
    ]
    material = bytearray()
    for name in names:
        data = (ROOT / "crates/nmlt-verify/src" / name).read_bytes()
        encoded_name = name.encode("utf-8")
        material.extend(len(encoded_name).to_bytes(8, "big"))
        material.extend(encoded_name)
        material.extend(len(data).to_bytes(8, "big"))
        material.extend(data)
    return "sha256:" + hashlib.sha256(material).hexdigest()


def rust_toolchain() -> str:
    process = subprocess.run(
        ["rustc", "--version"], cwd=ROOT, text=True, capture_output=True, check=False
    )
    if process.returncode != 0 or not process.stdout.strip():
        raise ValueError(f"cannot identify Rust toolchain: {process.stderr.strip()}")
    return process.stdout.strip()


def rust_target() -> str:
    process = subprocess.run(
        ["rustc", "--version", "--verbose"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if process.returncode != 0:
        raise ValueError(f"cannot identify Rust target: {process.stderr.strip()}")
    for line in process.stdout.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ")
    raise ValueError("rustc --version --verbose did not report a host target")


def run() -> dict[str, Any]:
    target = ROOT / "target/evidence-multi-engine"
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
            "nmlt-verify",
            "--example",
            "provider_two_engine",
        ],
        cwd=ROOT,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
    )
    if build.returncode != 0:
        raise ValueError(f"cannot build Phase 5 fixture: {build.stderr.strip()}")
    executable = target / "release/examples/provider_two_engine"
    if not executable.is_file():
        raise ValueError(f"cargo build did not create {executable}")

    first = subprocess.run(
        [str(executable)], cwd=ROOT, text=True, capture_output=True, check=False
    )
    second = subprocess.run(
        [str(executable)], cwd=ROOT, text=True, capture_output=True, check=False
    )
    if first.returncode != 0:
        raise ValueError(f"Phase 5 fixture failed: {first.stderr.strip()}")
    if second.returncode != 0 or second.stdout != first.stdout:
        raise ValueError("Phase 5 fixture execution is nondeterministic")
    value = json.loads(first.stdout)
    if not isinstance(value, dict):
        raise ValueError("Phase 5 fixture must emit a JSON object")

    property_document = load_json(PROPERTY)
    if property_document.get("property_id") != artifact_id(
        property_document, "property_id"
    ):
        raise ValueError("frozen property document has a stale identity")
    value["source"]["source_id"] = source_id(SOURCE)
    value["claim"]["property_id"] = property_document["property_id"]
    value["implementation"] = {
        "source_set_id": source_set_id(),
        "toolchain": rust_toolchain(),
        "target": rust_target(),
        "executable_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
    }
    value["evidence_id"] = evidence_id(value)
    return value


def semantic_errors(value: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    source = value.get("source", {})
    claim = value.get("claim", {})
    if source.get("content_sha256") != content_sha256(SOURCE):
        errors.append("source content digest is stale")
    if source.get("source_id") != source_id(SOURCE):
        errors.append("canonical source identity is stale")
    property_document = load_json(PROPERTY)
    if claim.get("content_sha256") != content_sha256(PROPERTY):
        errors.append("property content digest is stale")
    if claim.get("property_id") != property_document.get("property_id"):
        errors.append("canonical property identity is stale")
    if value.get("abstraction", {}).get("variables") != ["armed", "dispatched"]:
        errors.append("provider projection variable order changed")

    engines = value.get("engines", [])
    if not isinstance(engines, list) or len(engines) != 2:
        errors.append("exactly two engine results are required")
    else:
        expected = [
            (
                "nmlt-explicit-reachability",
                "deterministic-bfs/1",
                "model_checked",
                True,
            ),
            (
                "nmlt-finite-inductiveness",
                "finite-inductiveness-enumeration/1",
                "proved",
                True,
            ),
        ]
        build_digest = embedded_library_digest()
        for index, (name, method, classification, certificate_accepted) in enumerate(
            expected
        ):
            engine = engines[index]
            if (
                engine.get("name"),
                engine.get("method"),
                engine.get("normalized"),
                engine.get("certificate_accepted"),
            ) != (name, method, classification, certificate_accepted):
                errors.append(f"engine {index} classification or identity changed")
            if engine.get("build_digest") != build_digest:
                errors.append(f"engine {index} build/source digest is stale")
            if not engine.get("raw_output"):
                errors.append(f"engine {index} raw result was not retained")
        for index, purpose in ((0, "reachability coverage"), (1, "inductiveness")):
            certificate = engines[index].get("certificate")
            if not isinstance(certificate, dict):
                errors.append(f"{purpose} certificate is absent")
            elif (
                certificate.get("vc_digest") != value.get("vc_digest")
                or certificate.get("invariant_states") != [0, 1, 3]
            ):
                errors.append(f"{purpose} certificate is stale or forged")

    composite = value.get("composite", {})
    if composite.get("classification") != "proved":
        errors.append("compatible two-engine result did not compose to proved")
    if composite.get("assurance_subject") != "finite_vc_only":
        errors.append("proof classification escaped the exact finite VC")
    if composite.get("raw_results_retained") != 2:
        errors.append("composite evidence did not retain both raw results")
    if composite.get("disagreements") != []:
        errors.append("success fixture unexpectedly contains a disagreement")
    if value.get("anti_vacuity") != {
        "classification": "refuted",
        "witness_states": [0, 1, 3],
    }:
        errors.append("dispatch reachability anti-vacuity witness changed")

    controls = value.get("negative_controls", {})
    disagreement = controls.get("disagreement", {})
    if disagreement.get("classification") != "unknown":
        errors.append("engine disagreement did not fail closed")
    if disagreement.get("raw_results_retained") != 2:
        errors.append("disagreement did not retain both raw results")
    if disagreement.get("reason_count", 0) < 1:
        errors.append("disagreement has no localized reason")
    if controls.get("bounded_proof_laundering", {}).get("classification") != "unknown":
        errors.append("bounded evidence was laundered into proof")

    expected_implementation = run_implementation_identity()
    if value.get("implementation") != expected_implementation:
        errors.append("implementation identity is stale")
    if value.get("evidence_id") != evidence_id(value):
        errors.append("canonical evidence identity is stale")
    return errors


def run_implementation_identity() -> dict[str, str]:
    """Return the current identity without rebuilding during negative self-tests."""

    target = ROOT / "target/evidence-multi-engine/release/examples/provider_two_engine"
    if not target.is_file():
        raise ValueError("Phase 5 executable is absent")
    return {
        "source_set_id": source_set_id(),
        "toolchain": rust_toolchain(),
        "target": rust_target(),
        "executable_sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
    }


def validate_value(value: dict[str, Any], schema: dict[str, Any]) -> list[str]:
    errors = validate_instance(value, schema, schema)
    errors.extend(semantic_errors(value))
    return errors


def negative_self_tests(value: dict[str, Any], schema: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    mutations = []

    stale_source = copy.deepcopy(value)
    stale_source["source"]["source_id"] = "nmlt-source-v1:sha256:" + "0" * 64
    stale_source["evidence_id"] = evidence_id(stale_source)
    mutations.append(("stale source identity", stale_source))

    missing_raw = copy.deepcopy(value)
    missing_raw["engines"][0]["raw_output"] = ""
    missing_raw["evidence_id"] = evidence_id(missing_raw)
    mutations.append(("missing raw result", missing_raw))

    forged_certificate = copy.deepcopy(value)
    forged_certificate["engines"][1]["certificate"]["invariant_states"] = [0, 1]
    forged_certificate["evidence_id"] = evidence_id(forged_certificate)
    mutations.append(("forged certificate", forged_certificate))

    missing_coverage = copy.deepcopy(value)
    missing_coverage["engines"][0]["certificate"] = None
    missing_coverage["evidence_id"] = evidence_id(missing_coverage)
    mutations.append(("missing model-check coverage certificate", missing_coverage))

    accepted_disagreement = copy.deepcopy(value)
    accepted_disagreement["negative_controls"]["disagreement"]["classification"] = (
        "proved"
    )
    accepted_disagreement["evidence_id"] = evidence_id(accepted_disagreement)
    mutations.append(("accepted disagreement", accepted_disagreement))

    laundered_bound = copy.deepcopy(value)
    laundered_bound["negative_controls"]["bounded_proof_laundering"][
        "classification"
    ] = "proved"
    laundered_bound["evidence_id"] = evidence_id(laundered_bound)
    mutations.append(("bounded proof laundering", laundered_bound))

    for label, mutation in mutations:
        if not validate_value(mutation, schema):
            failures.append(f"negative control was accepted: {label}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--update", action="store_true")
    args = parser.parse_args()
    try:
        expected = run()
        schema = load_json(SCHEMA)
        errors = validate_schema_definition(schema, "multi-engine-evidence")
        errors.extend(validate_value(expected, schema))
        errors.extend(negative_self_tests(expected, schema))
        if errors:
            raise ValueError("; ".join(errors))
        if args.update:
            OUTPUT.parent.mkdir(parents=True, exist_ok=True)
            OUTPUT.write_text(json.dumps(expected, indent=2) + "\n", encoding="utf-8")
        actual = load_json(OUTPUT)
        if actual != expected:
            raise ValueError("persisted Phase 5 evidence differs from current execution")
    except (OSError, ValueError, json.JSONDecodeError, KeyError, TypeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print("ok: two independent routes, checked certificate, fail-closed composition")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
