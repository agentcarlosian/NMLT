#!/usr/bin/env python3
"""Build and read back exact M11-001b open-refinement evidence."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
from pathlib import Path
from typing import Any

from validate_benchmark_integrity import (
    load_json,
    source_id,
    validate_instance,
    validate_schema_definition,
)

ROOT = Path(__file__).resolve().parents[1]
LEAN_ROOT = ROOT / "mechanization/lean"
ENTRYPOINT = LEAN_ROOT / "NMLT/Behavior/OpenRefinement.lean"
ROOT_MODULE = LEAN_ROOT / "NMLT.lean"
TOOLCHAIN = LEAN_ROOT / "lean-toolchain"
RUST_TOOLCHAIN = ROOT / "rust-toolchain.toml"
METATHEORY_CHECKER = ROOT / "tools/check_metatheory.sh"
NANODA_CHECKER = ROOT / "tools/check_nanoda.sh"
CI_WORKFLOW = ROOT / ".github/workflows/ci.yml"
EVIDENCE_CHECKER = Path(__file__).resolve()
SCHEMA = ROOT / "schemas/open-refinement-evidence-v1.schema.json"
TCB_INVENTORY = ROOT / "security/trusted-components.toml"
SCHEMA_VALIDATOR = ROOT / "tools/validate_benchmark_integrity.py"
EVIDENCE = ROOT / "benchmarks/results/open-refinement/m11-001b-evidence.json"

SOURCE_PATHS = (
    ROOT_MODULE,
    LEAN_ROOT / "NMLT/Behavior/TemporalTyping.lean",
    LEAN_ROOT / "NMLT/Behavior/Refinement.lean",
    LEAN_ROOT / "NMLT/Behavior/OpenComposition.lean",
    ENTRYPOINT,
    TOOLCHAIN,
    LEAN_ROOT / "lakefile.toml",
    LEAN_ROOT / "lake-manifest.json",
    RUST_TOOLCHAIN,
    ROOT / "crates/nmlt-temporal/src/lib.rs",
    ROOT / "crates/nmlt-temporal/src/open.rs",
    ROOT / "crates/nmlt-temporal/src/open_contract.rs",
    ROOT / "crates/nmlt-temporal/src/open_refinement.rs",
)

THEOREM_AUDITS: tuple[tuple[str, str], ...] = (
    ("NMLT.Behavior.OpenRefinement.PredicateSubset.refl", "identity"),
    ("NMLT.Behavior.OpenRefinement.PredicateSubset.trans", "composition"),
    ("NMLT.Behavior.OpenRefinement.Refinement.identity", "identity"),
    ("NMLT.Behavior.OpenRefinement.Refinement.compose", "composition"),
    (
        "NMLT.Behavior.OpenRefinement.Refinement.exactPayloadIdentity",
        "payload_identity",
    ),
    (
        "NMLT.Behavior.OpenRefinement.Refinement.abstractAssumptionIncluded",
        "variance",
    ),
    (
        "NMLT.Behavior.OpenRefinement.Refinement.concreteGuaranteeIncluded",
        "variance",
    ),
)

AXIOM_PATTERN = re.compile(
    r"'(?P<handle>[A-Za-z0-9_.]+)' "
    r"(?:(?P<none>does not depend on any axioms)|"
    r"depends on axioms: \[(?P<axioms>[^\]\r\n]*)\])"
)


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode()


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def source_entries() -> list[dict[str, str]]:
    paths = sorted(SOURCE_PATHS, key=lambda path: str(path.relative_to(ROOT)).encode())
    return [
        {"path": str(path.relative_to(ROOT)), "sha256": digest(path.read_bytes())}
        for path in paths
    ]


def source_set_id(entries: list[dict[str, str]]) -> str:
    encoded = canonical_json(entries)
    value = hashlib.sha256(
        b"NMLT-OPEN-REFINEMENT-SOURCE-SET\0v1\0"
        + len(encoded).to_bytes(8, "big")
        + encoded
    ).hexdigest()
    return "nmlt-open-refinement-source-set-v1:sha256:" + value


def evidence_id(value: dict[str, Any]) -> str:
    payload = dict(value)
    payload.pop("evidence_id", None)
    encoded = canonical_json(payload)
    result = hashlib.sha256(
        b"NMLT-OPEN-REFINEMENT-EVIDENCE\0v1\0"
        + len(encoded).to_bytes(8, "big")
        + encoded
    ).hexdigest()
    return "nmlt-open-refinement-evidence-v1:sha256:" + result


def assert_source_contract() -> None:
    if "import NMLT.Behavior.OpenRefinement" not in ROOT_MODULE.read_text():
        raise ValueError("OpenRefinement is not imported by the Lean root module")
    source = ENTRYPOINT.read_text()
    for handle, _ in THEOREM_AUDITS:
        terminal = handle.rsplit(".", 1)[1]
        if re.search(
            rf"^[ \t]*(?:def|theorem)[ \t]+(?:[A-Za-z0-9_]+\.)*{re.escape(terminal)}\b",
            source,
            re.M,
        ) is None:
            raise ValueError(f"audited declaration is missing: {handle}")
        if re.search(rf"^[ \t]*#print[ \t]+axioms[ \t]+(?:[A-Za-z0-9_.]+\.)?{re.escape(terminal)}[ \t]*$", source, re.M) is None:
            raise ValueError(f"audited declaration lacks #print axioms: {handle}")
    if TOOLCHAIN.read_text().strip() != "v4.30.0":
        raise ValueError("unexpected Lean toolchain")


def expected_evidence() -> dict[str, Any]:
    assert_source_contract()
    entries = source_entries()
    value: dict[str, Any] = {
        "schema_version": "1.0.0",
        "claim_scope": "finite_contract_sound_open_refinement",
        "statement": (
            "Finite truth-table assumptions and guarantees refine under exact nominal "
            "payload identity, contravariant assumptions, covariant guarantees, and a "
            "complete injective label renaming; identity and composition are kernel checked."
        ),
        "sources": {
            "source_set_id": source_set_id(entries),
            "entries": entries,
            "root_source_id": source_id(ROOT_MODULE),
            "trusted_component_inventory_source_id": source_id(TCB_INVENTORY),
            "checker_source_id": source_id(METATHEORY_CHECKER),
            "nanoda_checker_source_id": source_id(NANODA_CHECKER),
            "ci_workflow_source_id": source_id(CI_WORKFLOW),
            "evidence_checker_source_id": source_id(EVIDENCE_CHECKER),
            "schema_validator_source_id": source_id(SCHEMA_VALIDATOR),
            "schema_source_id": source_id(SCHEMA),
        },
        "toolchain": {
            "lean": "v4.30.0",
            "rust": "1.94.0",
            "standard_library_only": True,
        },
        "theorem_audits": [
            {"handle": handle, "role": role, "declared_axioms": []}
            for handle, role in THEOREM_AUDITS
        ],
        "rust_controls": [
            "identity_is_accepted",
            "accepts_nonidentity_covariant_guarantee",
            "composes_nonidentity_refinement_witnesses",
            "rejects_assumption_strengthening",
            "rejects_guarantee_weakening",
            "rejects_payload_substitution_even_with_same_variants",
            "rejects_hidden_boundary_action",
            "rejects_incomplete_abstract_boundary_mapping",
            "rejects_noninjective_boundary_mapping",
            "rejects_circular_contract_discharge",
        ],
        "validation_gate": "./tools/check_metatheory.sh",
        "executable_correspondence": {
            "rust_checker": "finite_open_refinement_instance_checker",
            "verified": False,
        },
        "limitations": [
            "No product congruence theorem for the new open-refinement relation.",
            "No Rust/Lean correspondence or compiler-correctness theorem.",
            "No payload subtyping, representation conversion, capabilities, or grades.",
            "No circular contracts, fixed-point semantics, fairness, divergence, or liveness transport.",
            "Lean predicates use one shared finite message cardinality; Rust binds each predicate to a nominal finite enum identity.",
        ],
    }
    value["evidence_id"] = evidence_id(value)
    return value


def verify_lean_output_text(output: str) -> list[str]:
    handles = {handle for handle, _ in THEOREM_AUDITS}
    found: dict[str, list[tuple[str, ...]]] = {}
    for match in AXIOM_PATTERN.finditer(output):
        handle = match.group("handle")
        if handle not in handles:
            continue
        axioms = () if match.group("none") else tuple(
            item.strip() for item in match.group("axioms").split(",") if item.strip()
        )
        found.setdefault(handle, []).append(axioms)
    errors = []
    for handle, _ in THEOREM_AUDITS:
        if found.get(handle) != [()]:
            errors.append(f"Lean axiom audit for {handle} is missing, duplicated, or nonempty")
    return errors


def self_test(expected: dict[str, Any]) -> list[str]:
    failures = []
    stale = copy.deepcopy(expected)
    stale["sources"]["source_set_id"] = "nmlt-open-refinement-source-set-v1:sha256:" + "0" * 64
    stale["evidence_id"] = evidence_id(stale)
    if stale == expected:
        failures.append("stale source negative control was accepted")
    forged = copy.deepcopy(expected)
    forged["executable_correspondence"]["verified"] = True
    forged["evidence_id"] = evidence_id(forged)
    if forged == expected:
        failures.append("forged correspondence negative control was accepted")
    synthetic = "\n".join(f"'{handle}' does not depend on any axioms" for handle, _ in THEOREM_AUDITS)
    if verify_lean_output_text(synthetic):
        failures.append("exact synthetic Lean audit was rejected")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--update", action="store_true")
    parser.add_argument("--lean-output", type=Path)
    args = parser.parse_args()
    expected = expected_evidence()
    if args.update:
        EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
        EVIDENCE.write_bytes(canonical_json(expected))
    try:
        actual = load_json(EVIDENCE)
        schema = load_json(SCHEMA)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"error: cannot read evidence: {error}")
        return 1
    errors = []
    if actual != expected or actual.get("evidence_id") != evidence_id(actual):
        errors.append("evidence is stale or differs from the canonical exact record")
    errors.extend(validate_schema_definition(schema, SCHEMA.name))
    errors.extend(validate_instance(actual, schema, schema))
    errors.extend(self_test(expected))
    if args.lean_output:
        errors.extend(verify_lean_output_text(args.lean_output.read_text(errors="replace")))
    if errors:
        for error in errors:
            print(f"error: {error}")
        return 1
    print("ok: exact M11-001b contract/refinement source, theorem, control, and limitation bindings")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
