#!/usr/bin/env python3
"""Generate and independently read back schema-valid provider evidence."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import sys
from functools import cache
from pathlib import Path
from typing import Any

from check_model_reports import engine_source_set_id, model_checker_binary
from validate_benchmark_integrity import (
    DuplicateKey,
    artifact_id,
    benchmark_result_id,
    canonical_json,
    load_json,
    resolve_portable_path,
    source_id,
    validate_instance,
    validate_schema_definition,
)


ROOT = Path(__file__).resolve().parents[1]
SUITE = ROOT / "benchmarks/manifest.json"
SCHEMA = ROOT / "schemas/evidence-manifest.schema.json"
EVIDENCE = ROOT / "benchmarks/evidence/provider-attempt"
TCB_INVENTORY = ROOT / "security/trusted-components.toml"
DOMAIN = b"NMLT-EVIDENCE\0v1\0"
PREFIX = "nmlt-evidence-v1:sha256:"
CONFIG_DOMAIN = b"NMLT-CONFIG\0v1\0"
CONFIG_PREFIX = "nmlt-config-v1:sha256:"
ASSURANCE_RESULTS = frozenset(
    {"proved", "model_checked", "tested", "monitored", "refuted"}
)


def raw_sha256_id(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def trusted_component(component_id: str, identity: str) -> dict[str, str]:
    return {"id": component_id, "identity": identity}


def evidence_id(value: dict[str, Any]) -> str:
    payload = dict(value)
    payload.pop("manifest_id", None)
    payload.pop("signatures", None)
    encoded = canonical_json(payload)
    return PREFIX + hashlib.sha256(
        DOMAIN + len(encoded).to_bytes(8, "big") + encoded
    ).hexdigest()


def configuration_id(value: dict[str, Any]) -> str:
    encoded = canonical_json(value)
    return CONFIG_PREFIX + hashlib.sha256(
        CONFIG_DOMAIN + len(encoded).to_bytes(8, "big") + encoded
    ).hexdigest()


def slug(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-")


def load_property_contracts(suite: dict[str, Any]) -> dict[str, dict[str, Any]]:
    contracts: dict[str, dict[str, Any]] = {}
    for reference in suite["property_contracts"]:
        contract = load_json(ROOT / reference["path"])
        calculated = artifact_id(contract, "property_id")
        if reference["property_id"] != calculated or contract.get("property_id") != calculated:
            raise ValueError(f"stale property identity: {reference['path']}")
        contracts[calculated] = contract
    return contracts


def build_all(suite: dict[str, Any]) -> dict[str, dict[str, Any]]:
    properties = load_property_contracts(suite)
    cases = {case["id"]: case for case in suite["cases"]}
    results = {
        case_id: load_json(ROOT / case["result"]["path"])
        for case_id, case in cases.items()
    }
    generated: dict[str, dict[str, Any]] = {}

    for case_id, case in cases.items():
        result = results[case_id]
        if result["source"] != {"path": case["path"], "source_id": case["source_id"]}:
            raise ValueError(f"{case_id}: result is not bound to the suite source")
        report_by_name = {
            item["property"]: item for item in result["report"]["properties"]
        }
        for property_id in case["property_ids"]:
            contract = properties[property_id]
            name = contract["handle"].split(".")[-1]
            property_result = report_by_name.get(name)
            if property_result is None:
                raise ValueError(f"{case_id}: report omits property {name}")
            result_class = property_result["result"]
            if result_class not in {"model_checked", "refuted", "unknown"}:
                raise ValueError(f"{case_id}/{name}: invalid bounded result {result_class}")

            relevant_mutants = [
                other["result"]["result_id"]
                for other in cases.values()
                if other["class"] == "semantic_mutant"
                and property_id in other["property_ids"]
            ]
            control_ids = [
                witness["witness_id"] for witness in case.get("expected_witnesses", [])
            ]
            negative_controls = sorted(set(relevant_mutants + control_ids))
            engine = result["engine"]
            evidence: dict[str, Any] = {
                "schema_version": "0.1.0",
                "manifest_id": "",
                "artifact": {
                    "path": case["path"],
                    "source_id": case["source_id"],
                },
                "claim": {
                    "id": property_id,
                    "kind": contract["claim_kind"],
                },
                "result": result_class,
                "method": {
                    "kind": "deterministic_explicit_state_bfs_v1",
                    "engine": engine["name"],
                    "engine_version": engine["version"],
                    "engine_source_set_id": engine["source_set_id"],
                    "engine_executable_sha256": engine["executable_sha256"],
                },
                "scope": {
                    "configuration_id": configuration_id(result["configuration"]),
                    "bounds": [
                        f"max_states={result['configuration']['max_states']}",
                        f"max_depth={result['configuration']['max_depth']}",
                        f"frontier_complete={str(result['report']['complete']).lower()}",
                    ],
                    "observations": contract["observables"],
                },
                "assumptions": result["assumptions"],
                "trusted_components": [
                    trusted_component(
                        "trusted-component-inventory",
                        raw_sha256_id(TCB_INVENTORY),
                    ),
                    trusted_component(
                        "nmlt-engine-source", engine["source_set_id"]
                    ),
                    trusted_component(
                        "nmlt-engine-executable",
                        f"sha256:{engine['executable_sha256']}",
                    ),
                    trusted_component("benchmark-result", result["result_id"]),
                    trusted_component(
                        "rust-toolchain-declaration",
                        raw_sha256_id(ROOT / "rust-toolchain.toml"),
                    ),
                    trusted_component("evidence-schema", raw_sha256_id(SCHEMA)),
                    trusted_component(
                        "evidence-checker", raw_sha256_id(Path(__file__).resolve())
                    ),
                    trusted_component(
                        "benchmark-integrity-checker",
                        raw_sha256_id(ROOT / "tools/validate_benchmark_integrity.py"),
                    ),
                ],
                "negative_controls": negative_controls,
                "residual_gaps": [
                    "Finite explicit-state evidence is not an unbounded proof.",
                    "The executable provider fragment uses checked i64 arithmetic and open nominal constructors.",
                    "No verified compiler theorem yet connects parsed Rust IR to the Lean core.",
                ],
            }
            if result_class == "refuted":
                if property_result["witness"] is None:
                    raise ValueError(f"{case_id}/{name}: refutation has no witness")
                evidence["witness"] = {
                    "kind": "model-check-report/1.1.0#counterexample",
                    "reference": f"{result['result_id']}#property={name}",
                }
            evidence["manifest_id"] = evidence_id(evidence)
            filename = f"{slug(case_id)}--{slug(name)}.json"
            generated[filename] = evidence
    return generated


@cache
def repository_resolution() -> dict[str, Any]:
    suite = load_json(SUITE)
    contracts = load_property_contracts(suite)
    claims = set(contracts)
    controls: set[str] = set()
    witness_references: set[str] = set()
    result_ids: set[str] = set()
    configuration_ids: set[str] = set()
    expected_engine_source = engine_source_set_id()
    expected_executable = hashlib.sha256(model_checker_binary().read_bytes()).hexdigest()

    for case in suite["cases"]:
        result_path = ROOT / case["result"]["path"]
        result = load_json(result_path)
        calculated_result_id = benchmark_result_id(result)
        if result.get("result_id") != calculated_result_id:
            raise ValueError(f"stale benchmark result identity: {result_path.relative_to(ROOT)}")
        if case["result"].get("result_id") != calculated_result_id:
            raise ValueError(f"suite has stale result identity for {case['id']}")
        if result.get("engine", {}).get("source_set_id") != expected_engine_source:
            raise ValueError(f"stale engine source-set binding for {case['id']}")
        if result.get("engine", {}).get("executable_sha256") != expected_executable:
            raise ValueError(f"stale engine executable binding for {case['id']}")
        result_ids.add(calculated_result_id)
        configuration_ids.add(configuration_id(result["configuration"]))
        for property_result in result.get("report", {}).get("properties", []):
            if property_result.get("witness") is not None:
                witness_references.add(
                    f"{calculated_result_id}#property={property_result['property']}"
                )

        for witness_reference in case.get("expected_witnesses", []):
            witness = load_json(ROOT / witness_reference["path"])
            calculated_witness_id = artifact_id(witness, "witness_id")
            if (
                witness.get("witness_id") != calculated_witness_id
                or witness_reference.get("witness_id") != calculated_witness_id
            ):
                raise ValueError(f"stale expected-witness identity for {case['id']}")
            controls.add(calculated_witness_id)

    controls.update(result_ids)
    component_identities: dict[str, set[str]] = {
        "trusted-component-inventory": {raw_sha256_id(TCB_INVENTORY)},
        "nmlt-engine-source": {expected_engine_source},
        "nmlt-engine-executable": {f"sha256:{expected_executable}"},
        "benchmark-result": result_ids,
        "rust-toolchain-declaration": {raw_sha256_id(ROOT / "rust-toolchain.toml")},
        "evidence-schema": {raw_sha256_id(SCHEMA)},
        "evidence-checker": {raw_sha256_id(Path(__file__).resolve())},
        "benchmark-integrity-checker": {
            raw_sha256_id(ROOT / "tools/validate_benchmark_integrity.py")
        },
    }
    return {
        "claims": claims,
        "configuration_ids": configuration_ids,
        "controls": controls,
        "witness_references": witness_references,
        "component_identities": component_identities,
        "engine_source_set_id": expected_engine_source,
        "engine_executable_sha256": expected_executable,
        # No generic source-set membership resolver is implemented yet.
        "artifact_source_sets": set(),
    }


def semantic_errors(
    value: dict[str, Any], resolution: dict[str, Any] | None = None
) -> list[str]:
    errors: list[str] = []
    if str(value.get("manifest_id", "")).startswith("structural:"):
        if value.get("result") != "unknown":
            errors.append("legacy structural manifests have an unknown result ceiling")
        return errors

    if value.get("manifest_id") != evidence_id(value):
        errors.append("stale canonical manifest_id")

    if value.get("result") in ASSURANCE_RESULTS:
        if resolution is None:
            resolution = repository_resolution()
        artifact = value.get("artifact")
        if not isinstance(artifact, dict):
            errors.append("assurance evidence requires a structured artifact binding")
        else:
            path_text = artifact.get("path")
            try:
                path = resolve_portable_path(path_text)
            except (TypeError, ValueError) as error:
                errors.append(f"artifact path is not a confined portable path: {error}")
            else:
                if not path.is_file():
                    errors.append(f"artifact does not exist as a regular file: {path_text}")
                elif "source_id" in artifact:
                    try:
                        actual_source_id = source_id(path)
                    except OSError as error:
                        errors.append(f"artifact source bytes cannot be read: {error}")
                    else:
                        if artifact.get("source_id") != actual_source_id:
                            errors.append("artifact source_id does not match exact source bytes")
                elif artifact.get("source_set_id") not in resolution["artifact_source_sets"]:
                    errors.append("artifact source_set_id is not resolvable by the local checker")

        claim_id = value.get("claim", {}).get("id")
        if claim_id not in resolution["claims"]:
            errors.append("claim identity is not a current resolvable property contract")
        configuration = value.get("scope", {}).get("configuration_id")
        if configuration not in resolution["configuration_ids"]:
            errors.append("configuration identity is not a current resolvable configuration")

        method = value.get("method")
        trusted = value.get("trusted_components")
        if isinstance(method, dict) and isinstance(trusted, list):
            inventory_components = [
                component
                for component in trusted
                if isinstance(component, dict)
                and component.get("id") == "trusted-component-inventory"
            ]
            expected_inventory_identity = raw_sha256_id(TCB_INVENTORY)
            if len(inventory_components) != 1:
                errors.append(
                    "trusted_components must bind exactly one trusted-component inventory"
                )
            elif inventory_components[0].get("identity") != expected_inventory_identity:
                errors.append("trusted-component inventory identity is stale")
            identities = {
                item.get("identity")
                for item in trusted
                if isinstance(item, dict)
            }
            engine_source = method.get("engine_source_set_id")
            executable = method.get("engine_executable_sha256")
            if engine_source not in identities:
                errors.append("trusted_components do not bind the engine source set")
            if (
                not isinstance(executable, str)
                or f"sha256:{executable}" not in identities
            ):
                errors.append("trusted_components do not bind the engine executable")
            if engine_source != resolution["engine_source_set_id"]:
                errors.append("method engine source set is not current or resolvable")
            if executable != resolution["engine_executable_sha256"]:
                errors.append("method engine executable is not current or resolvable")
            for component in trusted:
                if not isinstance(component, dict):
                    continue
                allowed = resolution["component_identities"].get(component.get("id"), set())
                if component.get("identity") not in allowed:
                    errors.append(
                        f"trusted component is not locally resolvable: {component.get('id')}"
                    )

        for control in value.get("negative_controls", []):
            if control not in resolution["controls"]:
                errors.append(f"negative-control identity is not locally resolvable: {control}")

    certificate = value.get("certificate")
    if isinstance(certificate, dict) and "path" in certificate:
        try:
            certificate_path = resolve_portable_path(certificate["path"])
        except (TypeError, ValueError) as error:
            errors.append(f"certificate path is not a confined portable path: {error}")
        else:
            if not certificate_path.is_file():
                errors.append("certificate path does not name a regular file")
            else:
                expected_reference = raw_sha256_id(certificate_path)
                if certificate.get("reference") != expected_reference:
                    errors.append("certificate reference does not match certificate bytes")

    if value.get("result") == "refuted" and not isinstance(value.get("witness"), dict):
        errors.append("refuted evidence requires a structured witness reference")
    elif value.get("result") == "refuted":
        reference = value["witness"].get("reference")
        active_resolution = resolution if resolution is not None else repository_resolution()
        if reference not in active_resolution["witness_references"]:
            errors.append("witness reference is not locally resolvable")
    if value.get("result") == "model_checked":
        bounds = value.get("scope", {}).get("bounds", [])
        if not any(item == "frontier_complete=true" for item in bounds):
            errors.append("model_checked requires exhausted reachable frontier")
    if value.get("result") == "proved":
        errors.append("this bounded producer is not authorized to emit proved")
    return errors


def negative_self_tests(
    generated: dict[str, dict[str, Any]], schema: dict[str, Any]
) -> list[str]:
    failures: list[str] = []

    def schema_errors(value: dict[str, Any]) -> list[str]:
        return validate_instance(value, schema, schema)

    def accepted(value: dict[str, Any]) -> bool:
        return not schema_errors(value) and not semantic_errors(value)

    def expect_schema_rejection(label: str, value: dict[str, Any]) -> None:
        if not schema_errors(value):
            failures.append(f"{label} was schema-valid")

    def expect_semantic_rejection(label: str, value: dict[str, Any]) -> None:
        if schema_errors(value):
            failures.append(f"{label} did not exercise semantic readback")
        elif not semantic_errors(value):
            failures.append(f"{label} passed semantic readback")

    reference = next(
        value for value in generated.values() if value["result"] == "model_checked"
    )
    forged = copy.deepcopy(reference)
    forged["result"] = "proved"
    forged["manifest_id"] = evidence_id(forged)
    if accepted(forged):
        failures.append("bounded model check was promotable to proved")

    no_bounds = copy.deepcopy(reference)
    no_bounds["scope"].pop("bounds")
    no_bounds["manifest_id"] = evidence_id(no_bounds)
    if accepted(no_bounds):
        failures.append("model_checked evidence without bounds was accepted")

    refutation = next(value for value in generated.values() if value["result"] == "refuted")
    no_witness = copy.deepcopy(refutation)
    no_witness.pop("witness")
    no_witness["manifest_id"] = evidence_id(no_witness)
    if accepted(no_witness):
        failures.append("refutation without witness was accepted")

    nonexistent = copy.deepcopy(reference)
    nonexistent["artifact"] = {
        "path": "benchmarks/does-not-exist.nmlt",
        "source_id": "nmlt-source-v1:sha256:" + "0" * 64,
    }
    nonexistent["manifest_id"] = evidence_id(nonexistent)
    expect_semantic_rejection("nonexistent artifact forgery", nonexistent)

    stale_source = copy.deepcopy(reference)
    stale_source["artifact"]["source_id"] = "nmlt-source-v1:sha256:" + "0" * 64
    stale_source["manifest_id"] = evidence_id(stale_source)
    expect_semantic_rejection("stale exact-source binding", stale_source)

    unresolved_source_set = copy.deepcopy(reference)
    unresolved_source_set["artifact"].pop("source_id")
    unresolved_source_set["artifact"]["source_set_id"] = (
        "nmlt-source-set-v1:sha256:" + "7" * 64
    )
    unresolved_source_set["manifest_id"] = evidence_id(unresolved_source_set)
    expect_semantic_rejection("unresolved artifact source set", unresolved_source_set)

    unresolved_claim = copy.deepcopy(reference)
    unresolved_claim["claim"]["id"] = "nmlt-claim-v1:sha256:" + "7" * 64
    unresolved_claim["manifest_id"] = evidence_id(unresolved_claim)
    expect_semantic_rejection("unresolved claim identity", unresolved_claim)

    stale_configuration = copy.deepcopy(reference)
    stale_configuration["scope"]["configuration_id"] = (
        "nmlt-config-v1:sha256:" + "7" * 64
    )
    stale_configuration["manifest_id"] = evidence_id(stale_configuration)
    expect_semantic_rejection("unresolved configuration identity", stale_configuration)

    for member in (
        "engine",
        "engine_version",
        "engine_source_set_id",
        "engine_executable_sha256",
    ):
        missing_engine = copy.deepcopy(reference)
        missing_engine["method"].pop(member)
        missing_engine["manifest_id"] = evidence_id(missing_engine)
        expect_schema_rejection(f"missing method.{member}", missing_engine)

    unbound_engine_source = copy.deepcopy(reference)
    unbound_engine_source["method"]["engine_source_set_id"] = (
        "nmlt-engine-source-set-v1:sha256:" + "5" * 64
    )
    unbound_engine_source["manifest_id"] = evidence_id(unbound_engine_source)
    expect_semantic_rejection("engine source absent from TCB", unbound_engine_source)

    unbound_engine_executable = copy.deepcopy(reference)
    unbound_engine_executable["method"]["engine_executable_sha256"] = "6" * 64
    unbound_engine_executable["manifest_id"] = evidence_id(
        unbound_engine_executable
    )
    expect_semantic_rejection(
        "engine executable absent from TCB", unbound_engine_executable
    )

    missing_tcb = copy.deepcopy(reference)
    missing_tcb.pop("trusted_components")
    missing_tcb["manifest_id"] = evidence_id(missing_tcb)
    expect_schema_rejection("missing trusted_components", missing_tcb)

    empty_tcb = copy.deepcopy(reference)
    empty_tcb["trusted_components"] = []
    empty_tcb["manifest_id"] = evidence_id(empty_tcb)
    expect_schema_rejection("empty trusted_components", empty_tcb)

    trust_me_tcb = copy.deepcopy(reference)
    trust_me_tcb["trusted_components"] = [
        {"id": "trust-me", "identity": "trust-me"}
    ]
    trust_me_tcb["manifest_id"] = evidence_id(trust_me_tcb)
    expect_schema_rejection("unbound trust-me TCB", trust_me_tcb)

    unresolved_tcb = copy.deepcopy(reference)
    unresolved_tcb["trusted_components"].append(
        {"id": "invented-checker", "identity": "sha256:" + "7" * 64}
    )
    unresolved_tcb["manifest_id"] = evidence_id(unresolved_tcb)
    expect_semantic_rejection("unresolved content-addressed TCB", unresolved_tcb)

    omitted_inventory = copy.deepcopy(reference)
    omitted_inventory["trusted_components"] = [
        component
        for component in omitted_inventory["trusted_components"]
        if component["id"] != "trusted-component-inventory"
    ]
    omitted_inventory["manifest_id"] = evidence_id(omitted_inventory)
    expect_semantic_rejection("omitted trusted-component inventory", omitted_inventory)

    stale_inventory = copy.deepcopy(reference)
    next(
        component
        for component in stale_inventory["trusted_components"]
        if component["id"] == "trusted-component-inventory"
    )["identity"] = "sha256:" + "7" * 64
    stale_inventory["manifest_id"] = evidence_id(stale_inventory)
    expect_semantic_rejection("stale trusted-component inventory", stale_inventory)

    missing_source = copy.deepcopy(reference)
    missing_source["artifact"].pop("source_id")
    missing_source["manifest_id"] = evidence_id(missing_source)
    expect_schema_rejection("missing artifact source/source-set identity", missing_source)

    non_content_certificate = copy.deepcopy(reference)
    non_content_certificate["result"] = "proved"
    non_content_certificate["certificate"] = {
        "format": "trust-me/1",
        "reference": "trust-me",
    }
    non_content_certificate["manifest_id"] = evidence_id(non_content_certificate)
    expect_schema_rejection(
        "non-content-addressed proof certificate", non_content_certificate
    )

    digest_only_certificate = copy.deepcopy(reference)
    digest_only_certificate["result"] = "proved"
    digest_only_certificate["certificate"] = {
        "format": "self-test/1",
        "reference": "sha256:" + "7" * 64,
    }
    digest_only_certificate["manifest_id"] = evidence_id(digest_only_certificate)
    expect_schema_rejection("unresolvable digest-only certificate", digest_only_certificate)

    unresolved_control = copy.deepcopy(reference)
    unresolved_control["negative_controls"] = [
        "nmlt-test-control-v1:sha256:" + "7" * 64
    ]
    unresolved_control["manifest_id"] = evidence_id(unresolved_control)
    expect_semantic_rejection("unresolved negative control", unresolved_control)

    unresolved_witness = copy.deepcopy(refutation)
    unresolved_witness["witness"]["reference"] = (
        "nmlt-test-witness-v1:sha256:" + "7" * 64
    )
    unresolved_witness["manifest_id"] = evidence_id(unresolved_witness)
    expect_semantic_rejection("unresolved witness", unresolved_witness)

    audit_forgery = copy.deepcopy(reference)
    audit_forgery["result"] = "proved"
    audit_forgery["artifact"] = {
        "path": "benchmarks/does-not-exist.nmlt",
        "source_id": "nmlt-source-v1:sha256:" + "0" * 64,
    }
    audit_forgery["method"] = {
        "kind": "trust-me",
        "engine": "trust-me",
        "engine_version": "trust-me",
    }
    audit_forgery["trusted_components"] = ["trust-me"]
    audit_forgery["certificate"] = {
        "format": "trust-me",
        "reference": "trust-me",
    }
    audit_forgery["manifest_id"] = evidence_id(audit_forgery)
    if accepted(audit_forgery):
        failures.append(
            "nonexistent artifact + trust-me engine/TCB/certificate forgery was accepted"
        )

    structural = {
        "schema_version": "0.1.0",
        "manifest_id": "structural:example.nmlt",
        "artifact": {"path": "example.nmlt"},
        "claim": {"id": "source-structure", "kind": "well_formedness"},
        "result": "unknown",
        "method": {
            "kind": "structural_check",
            "engine": "nmlt-cli",
            "engine_version": "0.0.1",
        },
        "assumptions": [],
        "negative_controls": [],
        "residual_gaps": ["No semantic verification ran."],
    }
    if schema_errors(structural) or semantic_errors(structural):
        failures.append("honest legacy structural unknown was rejected")

    reordered = dict(reversed(list(reference.items())))
    if evidence_id(reordered) != evidence_id(reference):
        failures.append("object member order changed RFC 8785-subset identity")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--update", action="store_true")
    parser.add_argument(
        "--self-test-only",
        action="store_true",
        help="run adversarial controls without reading checked-in evidence manifests",
    )
    args = parser.parse_args()
    try:
        schema = load_json(SCHEMA)
        schema_errors = validate_schema_definition(schema, "evidence")
        if schema_errors:
            raise ValueError("; ".join(schema_errors))
        if args.self_test_only:
            suite = load_json(SUITE)
            failures = negative_self_tests(build_all(suite), schema)
            if failures:
                for failure in failures:
                    print(f"error: negative self-test: {failure}", file=sys.stderr)
                return 1
            print(
                "ok: evidence schema and adversarial source/engine/TCB/certificate "
                "controls passed"
            )
            return 0
        suite = load_json(SUITE)
        generated = build_all(suite)
    except (OSError, ValueError, KeyError, TypeError, DuplicateKey, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    if args.update:
        EVIDENCE.mkdir(parents=True, exist_ok=True)
        for filename, value in generated.items():
            (EVIDENCE / filename).write_text(
                json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
            )

    errors: list[str] = []
    for filename, expected in generated.items():
        path = EVIDENCE / filename
        try:
            actual = load_json(path)
        except (OSError, ValueError, DuplicateKey, json.JSONDecodeError) as error:
            errors.append(f"{path.relative_to(ROOT)}: {error}")
            continue
        errors.extend(
            f"{path.relative_to(ROOT)}: {error}"
            for error in validate_instance(actual, schema, schema) + semantic_errors(actual)
        )
        if actual != expected:
            errors.append(
                f"{path.relative_to(ROOT)}: evidence differs from current source-bound result"
            )
    errors.extend(
        f"negative self-test: {error}"
        for error in negative_self_tests(generated, schema)
    )
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(
        f"ok: {len(generated)} schema-valid canonical evidence manifests "
        "(source, engine, TCB, certificate, and bounded-promotion controls passed)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
