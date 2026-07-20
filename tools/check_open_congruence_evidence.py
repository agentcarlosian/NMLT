#!/usr/bin/env python3
"""Build and read back exact M11-001c open-congruence evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any

from validate_benchmark_integrity import (
    load_json,
    validate_instance,
    validate_schema_definition,
)

ROOT = Path(__file__).resolve().parents[1]
LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenComposition.lean"
MAPPED_LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenMappedCongruence.lean"
RESOURCE_LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenResourceCongruence.lean"
ENCODING_LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenEncodingCorrespondence.lean"
KERNEL_EXECUTION_LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenKernelExecution.lean"
KERNEL_READBACK_LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenKernelReadback.lean"
SOURCE_READBACK_LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenSourceReadback.lean"
VECTOR = ROOT / "mechanization/vectors/m11-open-congruence-v1.json"
SCHEMA = ROOT / "schemas/open-congruence-evidence-v1.schema.json"
EVIDENCE = ROOT / "benchmarks/results/open-congruence/m11-001c-evidence.json"

SOURCE_PATHS = (
    ROOT / "crates/nmlt-temporal/src/lib.rs",
    ROOT / "crates/nmlt-temporal/src/open.rs",
    ROOT / "crates/nmlt-temporal/src/open_contract.rs",
    ROOT / "crates/nmlt-temporal/src/open_refinement.rs",
    ROOT / "crates/nmlt-temporal/src/open_congruence.rs",
    ROOT / "crates/nmlt-temporal/src/open_encoding.rs",
    ROOT / "crates/nmlt-temporal/src/open_resources.rs",
    ROOT / "crates/nmlt-open-kernel/Cargo.toml",
    ROOT / "crates/nmlt-open-kernel/src/lib.rs",
    ROOT / "mechanization/lean/NMLT.lean",
    LEAN,
    MAPPED_LEAN,
    RESOURCE_LEAN,
    ENCODING_LEAN,
    KERNEL_EXECUTION_LEAN,
    KERNEL_READBACK_LEAN,
    SOURCE_READBACK_LEAN,
    ROOT / "mechanization/lean/NMLT/Behavior/OpenKernelGenerated/Types.lean",
    ROOT / "mechanization/lean/NMLT/Behavior/OpenKernelGenerated/Funs.lean",
    ROOT / "mechanization/lean/NMLT/Behavior/OpenRefinement.lean",
    VECTOR,
    ROOT / "mechanization/lean/lean-toolchain",
    ROOT / "mechanization/lean/lakefile.toml",
    ROOT / "rust-toolchain.toml",
    ROOT / "tools/check_m11_congruence_correspondence.py",
    Path(__file__).resolve(),
    SCHEMA,
)

THEOREMS = (
    (
        "NMLT.Behavior.OpenComposition.StrongRefinement.parallelStepCongruenceRight",
        "two_sided_lifting", (),
    ),
    (
        "NMLT.Behavior.OpenComposition.StrongRefinement.twoSidedCompositionCongruence",
        "two_sided_lifting", (),
    ),
    (
        "NMLT.Behavior.OpenComposition.StrongRefinement.preservesComposableRightUnderWiring",
        "contract_soundness", (),
    ),
    (
        "NMLT.Behavior.OpenComposition.StrongRefinement.transportInvariant",
        "invariant_transport", (),
    ),
    (
        "NMLT.Behavior.OpenMappedCongruence.MappedWiringEquivalent.leftConnected_iff",
        "contract_soundness", (),
    ),
    (
        "NMLT.Behavior.OpenMappedCongruence.MappedWiringEquivalent.rightConnected_iff",
        "contract_soundness", (),
    ),
    (
        "NMLT.Behavior.OpenMappedCongruence.MappedRefinement.parallelStepCongruence",
        "two_sided_lifting", (),
    ),
    (
        "NMLT.Behavior.OpenMappedCongruence.MappedRefinement.liftParallel",
        "two_sided_lifting", (),
    ),
    (
        "NMLT.Behavior.OpenMappedCongruence.MappedRefinement.transportInvariant",
        "invariant_transport", (),
    ),
    (
        "NMLT.Behavior.OpenMappedCongruence.MappedRefinement.abstractPortCovered",
        "contract_soundness", (),
    ),
    (
        "NMLT.Behavior.OpenMappedCongruence.MappedRefinement.abstractAssumptionIncluded",
        "contract_soundness", (),
    ),
    (
        "NMLT.Behavior.OpenMappedCongruence.MappedRefinement.concreteGuaranteeIncluded",
        "contract_soundness", (),
    ),
    (
        "NMLT.Behavior.OpenMappedCongruence.Examples.positiveMappedProduct",
        "two_sided_lifting", (),
    ),
    ("NMLT.Behavior.OpenResourceCongruence.partition_preserved", "resource_soundness", ()),
    ("NMLT.Behavior.OpenResourceCongruence.liftOpenProductResources", "resource_soundness", ("propext", "Quot.sound")),
    ("NMLT.Behavior.OpenResourceCongruence.liftResourceAwareParallel", "resource_soundness", ("propext", "Quot.sound")),
    ("NMLT.Behavior.OpenResourceCongruence.synchronized_transfer_exact", "resource_soundness", ()),
    ("NMLT.Behavior.OpenResourceCongruence.concreteGradeIncluded", "resource_soundness", ()),
    ("NMLT.Behavior.OpenResourceCongruence.concreteRelyIncluded", "resource_soundness", ()),
    ("NMLT.Behavior.OpenEncodingCorrespondence.check_sound", "encoding_correspondence", ("propext", "Quot.sound")),
    ("NMLT.Behavior.OpenEncodingCorrespondence.accepted_has_typed_action_maps", "encoding_correspondence", ("propext", "Quot.sound")),
    ("NMLT.Behavior.OpenEncodingCorrespondence.accepted_implementation_contract", "encoding_correspondence", ("propext", "Quot.sound")),
    ("NMLT.Behavior.OpenKernelExecution.check_accepts_implies_contract", "execution_correspondence", ("propext", "Classical.choice", "Quot.sound")),
    ("NMLT.Behavior.OpenKernelReadback.decode_functional", "execution_correspondence", ("propext",)),
    ("NMLT.Behavior.OpenKernelReadback.decode_injective", "execution_correspondence", ("propext", "Quot.sound")),
    ("NMLT.Behavior.OpenKernelReadback.check_accepts_with_readback", "execution_correspondence", ("propext", "Classical.choice", "Quot.sound")),
    ("NMLT.Behavior.OpenKernelReadback.referenced_id_in_dictionary", "execution_correspondence", ("propext", "Classical.choice", "Quot.sound")),
    ("NMLT.Behavior.OpenSourceReadback.accepted_exact_readback_contract", "encoding_correspondence", ("propext", "Quot.sound")),
    ("NMLT.Behavior.OpenSourceReadback.exact_readback_functional", "encoding_correspondence", ()),
)

AXIOMS = re.compile(
    r"'(?P<handle>[A-Za-z0-9_.]+)' "
    r"(?:(?P<none>does not depend on any axioms)|depends on axioms: \[(?P<axioms>[^\]]*)\])"
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


def domain_id(domain: bytes, prefix: str, value: object) -> str:
    encoded = canonical_json(value)
    result = digest(domain + len(encoded).to_bytes(8, "big") + encoded)
    return f"{prefix}:sha256:{result}"


def expected_evidence() -> dict[str, Any]:
    entries = source_entries()
    value: dict[str, Any] = {
        "schema_version": "1.0.0",
        "claim_scope": "finite_two_sided_open_refinement_congruence",
        "statement": (
            "Two independently checked finite open refinements lift through exact "
            "bijective wiring to a contract-checked product refinement, with finite "
            "reachable-state invariant transport, affine capability partitioning, "
            "monotone grades and rely discharge across every structural product-action "
            "case, plus a dictionary-readback-bound executable kernel contract."
        ),
        "sources": {
            "source_set_id": domain_id(
                b"NMLT-OPEN-CONGRUENCE-SOURCE-SET\0v1\0",
                "nmlt-open-congruence-source-set-v1",
                entries,
            ),
            "entries": entries,
            "vector_sha256": digest(VECTOR.read_bytes()),
        },
        "toolchain": {
            "lean": "v4.30.0",
            "rust": "1.94.0",
            "aeneas_commit": "45061fa1a5b4bad876f17c03d3a5544d818622e6",
            "charon_commit": "40ee060a8df43f4e7e0842d3f05387b0a4426aaf",
            "standard_library_only": False,
        },
        "theorem_audits": [
            {"handle": handle, "role": role, "declared_axioms": list(axioms)}
            for handle, role, axioms in THEOREMS
        ],
        "rust_controls": [
            "accepts_two_sided_contract_sound_congruence_and_invariant_transport",
            "rejects_connection_not_preserved_on_both_mapped_endpoints",
            "rejects_abstract_connection_outside_concrete_image",
            "rejects_stale_invariant_domain",
            "rejects_incomplete_component_boundary_mapping",
            "rejects_nonmonotone_composite_grade",
            "rejects_undischarged_rely_condition",
            "rejects_shared_affine_capability_between_components",
            "rejects_nonuniform_payload_universe_at_correspondence_boundary",
            "canonical_certificate_validator_fails_closed_after_mutation",
            "kernel_readback_rejects_dictionary_substitution",
            "kernel_readback_rejects_numeric_atom_substitution",
            "kernel_readback_rejects_active_action_omission",
            "canonical_validator_rejects_kernel_capacity_overflow",
            "source_readback_rejects_action_name_substitution",
            "source_readback_rejects_resource_substitution",
            "source_readback_rejects_wiring_substitution",
            "shared_m11_congruence_vectors_bind_the_rust_controls",
        ],
        "correspondence": {
            "kind": "proof_carrying_canonical_certificate",
            "vector_schema": "nmlt-m11-open-congruence-v1",
            "verified_implementation_theorem": False,
            "verified_execution_kernel_theorem": True,
            "dictionary_readback_enforced": True,
            "source_readback_enforced": True,
            "adapter_verified": False,
        },
        "validation_gate": "./tools/check_metatheory.sh",
        "limitations": [
            "Safety-only finite exact-action profile; no liveness or fairness transport.",
            "No liveness, fairness, hidden-divergence, payload-subtyping, or circular-discharge result.",
            "The bounded Rust kernel is translated by pinned Charon/Aeneas; its numeric envelope carries a canonical atom dictionary and Rust independently reads every active field back. Rust also independently reads the canonical representation back against every rich source field, while Lean specifies exact source-readback transport plus unique dictionary decoding and referenced-ID coverage. The rich encoder and both Rust readback implementations remain outside verified extraction.",
            "The correspondence profile deliberately requires one exact nominal payload universe across all four components and total visible action maps.",
            "The proved canonical projection retains the uncertainty upper-bound coordinate but not Rust uncertainty family/profile identity; the stronger family checks remain in nmlt-grades.",
        ],
    }
    value["evidence_id"] = domain_id(
        b"NMLT-OPEN-CONGRUENCE-EVIDENCE\0v1\0",
        "nmlt-open-congruence-evidence-v1",
        value,
    )
    return value


def verify_lean_output(output: str) -> list[str]:
    expected = {handle: axioms for handle, _, axioms in THEOREMS}
    found: dict[str, list[tuple[str, ...]]] = {}
    for match in AXIOMS.finditer(output):
        handle = match.group("handle")
        if handle in expected:
            axioms = () if match.group("none") else tuple(
                item.strip() for item in match.group("axioms").split(",") if item.strip()
            )
            found.setdefault(handle, []).append(axioms)
    return [
        f"Lean axiom audit for {handle} is missing, duplicated, or differs"
        for handle, axioms in expected.items()
        if found.get(handle) != [axioms]
    ]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--update", action="store_true")
    parser.add_argument("--lean-output", type=Path)
    args = parser.parse_args()
    expected = expected_evidence()
    if args.update:
        EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
        EVIDENCE.write_bytes(canonical_json(expected))
    actual = load_json(EVIDENCE)
    schema = load_json(SCHEMA)
    errors: list[str] = []
    if actual != expected:
        errors.append("evidence is stale or differs from the canonical exact record")
    errors.extend(validate_schema_definition(schema, SCHEMA.name))
    errors.extend(validate_instance(actual, schema, schema))
    if args.lean_output:
        errors.extend(verify_lean_output(args.lean_output.read_text(errors="replace")))
    if errors:
        for error in errors:
            print(f"error: {error}")
        return 1
    print("ok: exact M11-001c congruence source, theorem, vector, and limitation bindings")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
