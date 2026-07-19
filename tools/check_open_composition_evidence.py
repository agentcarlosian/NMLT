#!/usr/bin/env python3
"""Build and read back claim-specific M11 open-composition metatheory evidence."""

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
ENTRYPOINT = LEAN_ROOT / "NMLT/Behavior/OpenComposition.lean"
COUNTEREXAMPLE = LEAN_ROOT / "NMLT/Counterexamples/CompositionCongruence.lean"
ROOT_MODULE = LEAN_ROOT / "NMLT.lean"
TOOLCHAIN = LEAN_ROOT / "lean-toolchain"
METATHEORY_CHECKER = ROOT / "tools/check_metatheory.sh"
NANODA_CHECKER = ROOT / "tools/check_nanoda.sh"
CI_WORKFLOW = ROOT / ".github/workflows/ci.yml"
EVIDENCE_CHECKER = Path(__file__).resolve()
SCHEMA = ROOT / "schemas/open-composition-evidence-v1.schema.json"
TCB_INVENTORY = ROOT / "security/trusted-components.toml"
SCHEMA_VALIDATOR = ROOT / "tools/validate_benchmark_integrity.py"
EVIDENCE = ROOT / "benchmarks/results/open-composition/m11-001a-evidence.json"

SOURCE_PATHS = (
    ROOT_MODULE,
    LEAN_ROOT / "NMLT/Behavior/TemporalTyping.lean",
    LEAN_ROOT / "NMLT/Behavior/Refinement.lean",
    ENTRYPOINT,
    LEAN_ROOT / "NMLT/Core/Transition.lean",
    COUNTEREXAMPLE,
    TOOLCHAIN,
    LEAN_ROOT / "lakefile.toml",
    LEAN_ROOT / "lake-manifest.json",
)

THEOREM_AUDITS: tuple[tuple[str, str, tuple[str, ...]], ...] = (
    (
        "NMLT.Behavior.OpenComposition.StrongRefinement.compositionCongruence",
        "congruence",
        (),
    ),
    (
        "NMLT.Behavior.OpenComposition.StrongRefinement.parallelStepCongruence",
        "congruence",
        (),
    ),
    (
        "NMLT.Behavior.OpenComposition.parallelInputReceptive",
        "receptiveness",
        (),
    ),
    (
        "NMLT.Behavior.OpenComposition.StrongRefinement.outputCanSynchronize",
        "synchronization",
        (),
    ),
    (
        "NMLT.Behavior.OpenComposition.StrongRefinement.rightOutputCanSynchronize",
        "synchronization",
        (),
    ),
    (
        "NMLT.Behavior.OpenComposition.Examples.positiveCompositionCongruence",
        "positive_control",
        (),
    ),
    (
        "NMLT.Behavior.OpenComposition.Examples.positiveConcreteSynchronization",
        "positive_control",
        (),
    ),
    (
        "NMLT.Behavior.OpenComposition.Examples.peerOnlyWithEmptyWiring",
        "positive_control",
        (),
    ),
    (
        "NMLT.Behavior.OpenComposition.Examples.brokenWiringBlocksPeerOnly",
        "negative_control",
        (),
    ),
    (
        "NMLT.Behavior.OpenComposition.Examples.brokenWiringNotEquivalent",
        "negative_control",
        (),
    ),
    (
        "NMLT.Counterexamples.CompositionCongruence.noCompositeRefinement",
        "negative_control",
        ("propext",),
    ),
)

AXIOM_AUDIT_PATTERN = re.compile(
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
        b"NMLT-OPEN-COMPOSITION-SOURCE-SET\0v1\0"
        + len(encoded).to_bytes(8, "big")
        + encoded
    ).hexdigest()
    return "nmlt-open-composition-source-set-v1:sha256:" + value


def evidence_id(value: dict[str, Any]) -> str:
    payload = dict(value)
    payload.pop("evidence_id", None)
    encoded = canonical_json(payload)
    result = hashlib.sha256(
        b"NMLT-OPEN-COMPOSITION-EVIDENCE\0v1\0"
        + len(encoded).to_bytes(8, "big")
        + encoded
    ).hexdigest()
    return "nmlt-open-composition-evidence-v1:sha256:" + result


def assert_source_contract() -> None:
    root = ROOT_MODULE.read_text(encoding="utf-8")
    if "import NMLT.Behavior.OpenComposition" not in root:
        raise ValueError("OpenComposition is not imported by the Lean root module")
    entrypoint = ENTRYPOINT.read_text(encoding="utf-8")
    counterexample = COUNTEREXAMPLE.read_text(encoding="utf-8")
    for handle, _, _ in THEOREM_AUDITS:
        terminal = handle.rsplit(".", 1)[1]
        source = counterexample if ".Counterexamples." in handle else entrypoint
        if re.search(
            rf"^[ \t]*theorem[ \t]+{re.escape(terminal)}\b", source, re.MULTILINE
        ) is None:
            raise ValueError(f"audited theorem is missing: {handle}")
        if re.search(
            rf"^[ \t]*#print[ \t]+axioms[ \t]+{re.escape(terminal)}[ \t]*(?:--.*)?$",
            source,
            re.MULTILINE,
        ) is None:
            raise ValueError(f"audited theorem lacks #print axioms: {handle}")
    toolchain = TOOLCHAIN.read_text(encoding="utf-8").strip()
    if toolchain != "v4.30.0":
        raise ValueError(f"unexpected Lean toolchain {toolchain!r}")


def expected_evidence() -> dict[str, Any]:
    assert_source_contract()
    entries = source_entries()
    value: dict[str, Any] = {
        "schema_version": "1.0.0",
        "claim_scope": "finite_exact_action_open_composition",
        "statement": (
            "Kernel-checked structural exact-action safety congruence plus separate "
            "composability preservation for globally input-receptive open systems "
            "under whole-wiring equivalence."
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
        "toolchain": {"lean": "v4.30.0", "standard_library_only": True},
        "theorem_audits": [
            {
                "handle": handle,
                "role": role,
                "declared_axioms": list(axioms),
            }
            for handle, role, axioms in THEOREM_AUDITS
        ],
        "validation_gate": "./tools/check_metatheory.sh",
        "executable_correspondence": {
            "rust_checker": "related_nonidentical_finite_instance_checker",
            "verified": False,
        },
        "limitations": [
            "No weak hiding or label-map congruence theorem.",
            "No payload-type, capability, grade, fairness, divergence, or liveness preservation.",
            "No temporal or circular assume/guarantee contract semantics.",
            "No Rust/Lean correspondence or compiler-correctness theorem.",
            "Lean permits arbitrary whole wiring relations; Rust requires one-to-one connections.",
            "The positive theorem is one-sided and requires state-surjective exact-action refinement.",
        ],
    }
    value["evidence_id"] = evidence_id(value)
    return value


def readback_errors(value: object, expected: dict[str, Any]) -> list[str]:
    if not isinstance(value, dict):
        return ["evidence must be a JSON object"]
    errors: list[str] = []
    if value != expected:
        errors.append("evidence is stale or differs from the canonical claim-specific record")
    if value.get("evidence_id") != evidence_id(value):
        errors.append("evidence_id does not bind the exact manifest")
    return errors


def render_axiom_audit(handle: str, axioms: tuple[str, ...]) -> str:
    if axioms:
        return f"'{handle}' depends on axioms: [{', '.join(axioms)}]"
    return f"'{handle}' does not depend on any axioms"


def verify_lean_output_text(output: str) -> list[str]:
    records: dict[str, list[tuple[str, ...]]] = {}
    audited_handles = {handle for handle, _, _ in THEOREM_AUDITS}
    for match in AXIOM_AUDIT_PATTERN.finditer(output):
        handle = match.group("handle")
        if handle not in audited_handles:
            continue
        if match.group("none") is not None:
            axioms: tuple[str, ...] = ()
        else:
            text = match.group("axioms").strip()
            axioms = tuple(item.strip() for item in text.split(",") if item.strip())
        records.setdefault(handle, []).append(axioms)

    errors = []
    for handle, _, expected_axioms in THEOREM_AUDITS:
        found = records.get(handle, [])
        if len(found) != 1:
            errors.append(
                f"Lean output has {len(found)} axiom audit records for {handle}; expected exactly one"
            )
        elif found[0] != expected_axioms:
            errors.append(
                "Lean axiom audit mismatch for "
                f"{handle}: expected {list(expected_axioms)!r}, found {list(found[0])!r}"
            )
    return errors


def verify_lean_output(path: Path) -> list[str]:
    return verify_lean_output_text(path.read_text(encoding="utf-8", errors="replace"))


def self_test(expected: dict[str, Any]) -> list[str]:
    failures = []
    stale_source = copy.deepcopy(expected)
    stale_source["sources"]["source_set_id"] = (
        "nmlt-open-composition-source-set-v1:sha256:" + "0" * 64
    )
    stale_source["evidence_id"] = evidence_id(stale_source)
    if not readback_errors(stale_source, expected):
        failures.append("stale source-set negative control was accepted")

    missing_theorem = copy.deepcopy(expected)
    missing_theorem["theorem_audits"].pop()
    missing_theorem["evidence_id"] = evidence_id(missing_theorem)
    if not readback_errors(missing_theorem, expected):
        failures.append("missing theorem negative control was accepted")

    forged_correspondence = copy.deepcopy(expected)
    forged_correspondence["executable_correspondence"]["verified"] = True
    forged_correspondence["evidence_id"] = evidence_id(forged_correspondence)
    if not readback_errors(forged_correspondence, expected):
        failures.append("forged Rust/Lean correspondence was accepted")

    exact_audit = "\n".join(
        render_axiom_audit(handle, axioms) for handle, _, axioms in THEOREM_AUDITS
    )
    if verify_lean_output_text(exact_audit):
        failures.append("exact synthetic Lean axiom audit was rejected")
    forged_extra_audit = exact_audit + "\n" + render_axiom_audit(
        THEOREM_AUDITS[0][0], ("Classical.choice",)
    )
    if not verify_lean_output_text(forged_extra_audit):
        failures.append("forged extra Lean axiom audit was accepted")
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
    if not EVIDENCE.is_file():
        print(f"error: missing {EVIDENCE.relative_to(ROOT)}")
        return 1

    try:
        actual = load_json(EVIDENCE)
        schema = load_json(SCHEMA)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"error: cannot read evidence: {error}")
        return 1

    errors = readback_errors(actual, expected)
    errors.extend(validate_schema_definition(schema, SCHEMA.name))
    errors.extend(validate_instance(actual, schema, schema))
    errors.extend(self_test(expected))
    if args.lean_output is not None:
        errors.extend(verify_lean_output(args.lean_output))
    if errors:
        for error in errors:
            print(f"error: {error}")
        return 1
    print(
        "ok: claim-specific M11 open-composition source, theorem, axiom, "
        "negative-control, and limitation bindings"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
