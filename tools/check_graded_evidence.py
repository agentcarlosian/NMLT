#!/usr/bin/env python3
"""Build, execute, bind, and schema-check the Phase 7 graded evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

from validate_benchmark_integrity import (
    canonical_json,
    load_json,
    validate_instance,
    validate_schema_definition,
)


ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates/nmlt-grades"
REFERENCE = ROOT / "benchmarks/grades/provider_pipeline.nmltg"
CONTROLS = {
    "privacy_budget_violation": ROOT
    / "benchmarks/grades/privacy_budget_violation.nmltg",
    "unknown_iteration": ROOT / "benchmarks/grades/unknown_iteration.nmltg",
    "invalid_uncertainty": ROOT / "benchmarks/grades/invalid_uncertainty.nmltg",
}
OUTPUT = ROOT / "benchmarks/grades/evidence.json"
SCHEMA = ROOT / "schemas/graded-evidence-v1.schema.json"
CLAIM_SPEC = ROOT / "rfcs/0012-graded-resource-modalities.md"
HARNESS = ROOT / "tools/check_graded_evidence.py"
VALIDATOR = ROOT / "tools/validate_benchmark_integrity.py"
METATHEORY_ROOT = ROOT / "mechanization/lean/NMLT.lean"
METATHEORY_ENTRYPOINT = ROOT / "mechanization/lean/NMLT/Grades/Algebra.lean"
METATHEORY_TOOLCHAIN = ROOT / "mechanization/lean/lean-toolchain"
METATHEORY_CHECKER = ROOT / "tools/check_metatheory.sh"
TCB_INVENTORY = ROOT / "security/trusted-components.toml"
METATHEORY_PATHS = [
    ROOT / "mechanization/lean/lakefile.toml",
    METATHEORY_ROOT,
    METATHEORY_ENTRYPOINT,
    METATHEORY_TOOLCHAIN,
]
METATHEORY_THEOREMS = [
    "NMLT.Grades.zero_sequential",
    "NMLT.Grades.sequential_zero",
    "NMLT.Grades.sequential_comm",
    "NMLT.Grades.sequential_assoc",
    "NMLT.Grades.zero_parallel",
    "NMLT.Grades.parallel_zero",
    "NMLT.Grades.parallel_comm",
    "NMLT.Grades.parallel_assoc",
    "NMLT.Grades.zero_choice",
    "NMLT.Grades.choice_zero",
    "NMLT.Grades.choice_comm",
    "NMLT.Grades.choice_assoc",
    "NMLT.Grades.choice_self",
    "NMLT.Grades.zero_le",
    "NMLT.Grades.le_iff_choice_eq_right",
    "NMLT.Grades.sequential_mono",
    "NMLT.Grades.sequential_choice_distrib",
    "NMLT.Grades.sequential_nonempty_choices_distrib",
    "NMLT.Grades.budgetAccept_iff",
    "NMLT.Grades.budgetAccept_sound",
]
CLAIM_HANDLE = "Grades.ProviderPipeline.WithinBudget"
CLAIM_BUDGET = {
    "cost_ticks": 100,
    "privacy_micro_epsilon": 550000,
    "energy_microjoules": 180,
    "uncertainty_ppm": 60000,
}


class DuplicateKey(ValueError):
    """Raised when executable evidence contains duplicate JSON names."""


def reject_duplicate_names(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise DuplicateKey(f"duplicate JSON member {key!r}")
        value[key] = item
    return value


def source_digest(data: bytes) -> bytes:
    preimage = b"NMLT-SOURCE\0v1\0" + len(data).to_bytes(8, "big") + data
    return hashlib.sha256(preimage).digest()


def source_id(path: Path) -> str:
    return "nmlt-source-v1:sha256:" + source_digest(path.read_bytes()).hex()


def implementation_paths() -> list[Path]:
    return sorted(
        [
            CRATE / "Cargo.toml",
            *CRATE.joinpath("src").rglob("*.rs"),
            *CRATE.joinpath("examples").rglob("*.rs"),
        ],
        key=lambda path: str(path.relative_to(ROOT)).encode("utf-8"),
    )


def source_set_id(paths: list[Path]) -> str:
    encoded = bytearray(b"NMLT-SOURCE-SET\0v1\0")
    encoded.extend(len(paths).to_bytes(8, "big"))
    seen: set[str] = set()
    for path in paths:
        relative = str(path.relative_to(ROOT))
        if relative in seen:
            raise ValueError(f"duplicate source-set path {relative!r}")
        seen.add(relative)
        path_bytes = relative.encode("utf-8")
        encoded.extend(len(path_bytes).to_bytes(8, "big"))
        encoded.extend(path_bytes)
        encoded.extend(source_digest(path.read_bytes()))
    return "nmlt-source-set-v1:sha256:" + hashlib.sha256(encoded).hexdigest()


def local_lean_statement_closure() -> list[Path]:
    """Resolve the repository-local import closure rooted at NMLT.lean."""
    lean_root = ROOT / "mechanization/lean"
    pending = [METATHEORY_ROOT]
    closure: set[Path] = set()
    while pending:
        path = pending.pop()
        if path in closure:
            continue
        if not path.is_file():
            raise ValueError(
                f"local Lean import does not resolve to a file: {path.relative_to(ROOT)}"
            )
        closure.add(path)
        for line in path.read_text(encoding="utf-8").splitlines():
            declaration = line.split("--", 1)[0].strip()
            if not declaration.startswith("import "):
                continue
            for module in declaration.removeprefix("import ").split():
                if module != "NMLT" and not module.startswith("NMLT."):
                    continue
                candidate = lean_root / (module.replace(".", "/") + ".lean")
                if candidate not in closure:
                    pending.append(candidate)
    return sorted(
        closure,
        key=lambda path: str(path.relative_to(ROOT)).encode("utf-8"),
    )


def load_tcb_inventory() -> dict[str, Any]:
    value = tomllib.loads(TCB_INVENTORY.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("trusted-component inventory must be a TOML table")
    return value


def lean_tcb_paths(inventory: dict[str, Any] | None = None) -> list[Path]:
    """Read and verify the exact nmlt_lean.statements component membership."""
    inventory = load_tcb_inventory() if inventory is None else inventory
    components = inventory.get("components")
    if not isinstance(components, list):
        raise ValueError("trusted-component inventory omits [[components]]")
    matches = [
        component
        for component in components
        if isinstance(component, dict)
        and component.get("id") == "nmlt_lean.statements"
    ]
    if len(matches) != 1:
        raise ValueError(
            "trusted-component inventory must define nmlt_lean.statements exactly once"
        )
    declared = matches[0].get("paths")
    if (
        not isinstance(declared, list)
        or not declared
        or not all(isinstance(path, str) and path for path in declared)
    ):
        raise ValueError("nmlt_lean.statements paths must be nonempty strings")
    if len(set(declared)) != len(declared):
        raise ValueError("nmlt_lean.statements contains duplicate paths")

    resolved: list[Path] = []
    for relative in declared:
        relative_path = Path(relative)
        if relative_path.is_absolute() or ".." in relative_path.parts:
            raise ValueError(
                f"nmlt_lean.statements path escapes the repository: {relative}"
            )
        candidate = ROOT / relative_path
        try:
            canonical_candidate = candidate.resolve(strict=True)
            canonical_candidate.relative_to(ROOT.resolve(strict=True))
        except (OSError, ValueError) as error:
            raise ValueError(
                f"nmlt_lean.statements path escapes the repository: {relative}"
            ) from error
        if canonical_candidate != candidate.absolute() or not candidate.is_file():
            raise ValueError(
                f"nmlt_lean.statements path is not a regular in-tree file: {relative}"
            )
        resolved.append(candidate)

    expected = local_lean_statement_closure()
    declared_names = {str(path.relative_to(ROOT)) for path in resolved}
    expected_names = {str(path.relative_to(ROOT)) for path in expected}
    if declared_names != expected_names:
        missing = sorted(expected_names - declared_names)
        extra = sorted(declared_names - expected_names)
        raise ValueError(
            "nmlt_lean.statements does not equal the local NMLT import closure; "
            f"missing={missing}, extra={extra}"
        )
    return sorted(
        resolved,
        key=lambda path: str(path.relative_to(ROOT)).encode("utf-8"),
    )


def claim_id(reference_id: str, claim_spec_id: str) -> str:
    payload = {
        "budget": CLAIM_BUDGET,
        "claim_handle": CLAIM_HANDLE,
        "claim_spec_source_id": claim_spec_id,
        "composition_profile": "nmlt-graded-product-v1",
        "reference_source_id": reference_id,
    }
    encoded = canonical_json(payload)
    digest = hashlib.sha256(
        b"NMLT-GRADED-CLAIM\0v1\0"
        + len(encoded).to_bytes(8, "big")
        + encoded
    ).hexdigest()
    return "nmlt-graded-claim-v1:sha256:" + digest


def metatheory_binding() -> dict[str, Any]:
    entrypoint = METATHEORY_ENTRYPOINT.read_text(encoding="utf-8")
    root_source = METATHEORY_ROOT.read_text(encoding="utf-8")
    if "import NMLT.Grades.Algebra" not in root_source:
        raise ValueError("graded algebra is not wired into the Lean root module")
    for handle in METATHEORY_THEOREMS:
        declaration = f"theorem {handle.rsplit('.', 1)[1]}"
        if declaration not in entrypoint:
            raise ValueError(f"graded metatheory theorem is missing: {handle}")
    toolchain = METATHEORY_TOOLCHAIN.read_text(encoding="utf-8").strip()
    if toolchain != "v4.30.0":
        raise ValueError(f"unexpected Lean toolchain {toolchain!r}")
    return {
        "scope": "mathematical_grade_algebra_only",
        "source_set_id": source_set_id(
            sorted(
                METATHEORY_PATHS,
                key=lambda path: str(path.relative_to(ROOT)).encode("utf-8"),
            )
        ),
        "lean_tcb_source_set_id": source_set_id(lean_tcb_paths()),
        "root_source_id": source_id(METATHEORY_ROOT),
        "entrypoint_source_id": source_id(METATHEORY_ENTRYPOINT),
        "checker_source_id": source_id(METATHEORY_CHECKER),
        "toolchain": toolchain,
        "theorem_handles": METATHEORY_THEOREMS,
        "declared_kernel_dependencies": ["propext", "Quot.sound"],
        "validation_gate": "./tools/check_metatheory.sh (separate pinned Lean gate)",
        "rust_correspondence": "manual_operation_level_alignment",
        "verified_rust_extraction": False,
    }


def manifest_id(value: dict[str, Any]) -> str:
    payload = dict(value)
    payload.pop("manifest_id", None)
    encoded = canonical_json(payload)
    digest = hashlib.sha256(
        b"NMLT-EVIDENCE\0v1\0" + len(encoded).to_bytes(8, "big") + encoded
    ).hexdigest()
    return "nmlt-evidence-v1:sha256:" + digest


def build(directory: Path) -> Path:
    directory.mkdir(parents=True, exist_ok=True)
    library = directory / "libnmlt_grades.rlib"
    executable = directory / "graded_evidence"
    common = [
        "--edition=2024",
        "-Copt-level=3",
        "-Cdebuginfo=0",
        "-Ccodegen-units=1",
        "-Cstrip=symbols",
        f"--remap-path-prefix={ROOT}=/nmlt",
    ]
    environment = os.environ.copy()
    environment.update({"SOURCE_DATE_EPOCH": "0"})
    commands = [
        [
            "rustc",
            *common,
            "--crate-name=nmlt_grades",
            "--crate-type=rlib",
            "-Cmetadata=nmlt_grades_phase7_v1",
            str(CRATE / "src/lib.rs"),
            "-o",
            str(library),
        ],
        [
            "rustc",
            *common,
            "--crate-name=graded_evidence",
            str(CRATE / "examples/graded_evidence.rs"),
            "--extern",
            f"nmlt_grades={library}",
            "-o",
            str(executable),
        ],
    ]
    for command in commands:
        completed = subprocess.run(
            command,
            cwd=ROOT,
            env=environment,
            text=True,
            capture_output=True,
            check=False,
        )
        if completed.returncode != 0:
            raise ValueError(completed.stderr.strip())
    return executable


def execute(
    executable: Path,
    reference_id: str,
    claim_spec_id: str,
    graded_claim_id: str,
) -> dict[str, Any]:
    environment = os.environ.copy()
    environment.update(
        {
            "NMLT_GRADED_REFERENCE_SOURCE_ID": reference_id,
            "NMLT_GRADED_CLAIM_SPEC_SOURCE_ID": claim_spec_id,
            "NMLT_GRADED_CLAIM_ID": graded_claim_id,
        }
    )
    command = [
        str(executable),
        str(REFERENCE),
        str(CONTROLS["privacy_budget_violation"]),
        str(CONTROLS["unknown_iteration"]),
        str(CONTROLS["invalid_uncertainty"]),
    ]
    first = subprocess.run(
        command, cwd=ROOT, env=environment, text=True, capture_output=True, check=False
    )
    second = subprocess.run(
        command, cwd=ROOT, env=environment, text=True, capture_output=True, check=False
    )
    if first.returncode != 0:
        raise ValueError(first.stderr.strip())
    if second.returncode != 0 or second.stdout != first.stdout:
        raise ValueError("graded evidence execution is nondeterministic")
    value = json.loads(first.stdout, object_pairs_hook=reject_duplicate_names)
    if not isinstance(value, dict):
        raise ValueError("graded evidence must be a JSON object")
    return value


def produce() -> tuple[dict[str, Any], Path]:
    first_executable = build(ROOT / "target/evidence-grades-a")
    second_executable = build(ROOT / "target/evidence-grades-b")
    first_digest = hashlib.sha256(first_executable.read_bytes()).hexdigest()
    second_digest = hashlib.sha256(second_executable.read_bytes()).hexdigest()
    if first_digest != second_digest:
        raise ValueError("independent graded evidence builds produced different executables")

    reference_id = source_id(REFERENCE)
    claim_spec_id = source_id(CLAIM_SPEC)
    value = execute(
        first_executable,
        reference_id,
        claim_spec_id,
        claim_id(reference_id, claim_spec_id),
    )
    verbose_version = subprocess.run(
        ["rustc", "--version", "--verbose"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    ).stdout
    host_lines = [
        line.removeprefix("host: ")
        for line in verbose_version.splitlines()
        if line.startswith("host: ")
    ]
    if len(host_lines) != 1:
        raise ValueError("rustc verbose version did not contain exactly one host")
    sysroot = subprocess.run(
        ["rustc", "--print", "sysroot"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    ).stdout.strip()
    rustc_executable = Path(sysroot) / "bin/rustc"
    if not rustc_executable.is_file():
        raise ValueError(f"rustc executable not found at {rustc_executable}")
    value["implementation"] = {
        "source_set_id": source_set_id(implementation_paths()),
        "toolchain": {
            "version": verbose_version.splitlines()[0],
            "host": host_lines[0],
            "rustc_sha256": hashlib.sha256(rustc_executable.read_bytes()).hexdigest(),
        },
        "executable_sha256": first_digest,
        "control_source_ids": {
            name: source_id(path) for name, path in CONTROLS.items()
        },
        "trusted_component_source_ids": {
            "evidence_harness": source_id(HARNESS),
            "evidence_schema": source_id(SCHEMA),
            "schema_validator": source_id(VALIDATOR),
            "trusted_component_inventory": source_id(TCB_INVENTORY),
        },
    }
    value["metatheory"] = metatheory_binding()
    value["manifest_id"] = manifest_id(value)
    return value, first_executable


def binding_errors(value: dict[str, Any], executable: Path) -> list[str]:
    errors: list[str] = []
    reference_id = source_id(REFERENCE)
    claim_spec_id = source_id(CLAIM_SPEC)
    reference = value.get("reference", {})
    implementation = value.get("implementation", {})
    if reference.get("source_id") != reference_id:
        errors.append("reference source identity is stale")
    if reference.get("claim_spec_source_id") != claim_spec_id:
        errors.append("graded claim specification identity is stale")
    if reference.get("claim_id") != claim_id(reference_id, claim_spec_id):
        errors.append("graded claim identity is stale")
    if implementation.get("source_set_id") != source_set_id(implementation_paths()):
        errors.append("implementation source-set identity is stale")
    if implementation.get("executable_sha256") != hashlib.sha256(
        executable.read_bytes()
    ).hexdigest():
        errors.append("evidence executable identity is stale")
    expected_controls = {name: source_id(path) for name, path in CONTROLS.items()}
    if implementation.get("control_source_ids") != expected_controls:
        errors.append("negative-control source identities are stale")
    expected_trusted = {
        "evidence_harness": source_id(HARNESS),
        "evidence_schema": source_id(SCHEMA),
        "schema_validator": source_id(VALIDATOR),
        "trusted_component_inventory": source_id(TCB_INVENTORY),
    }
    if implementation.get("trusted_component_source_ids") != expected_trusted:
        errors.append("trusted evidence-component identities are stale")
    if value.get("metatheory") != metatheory_binding():
        errors.append("graded metatheory binding is stale")
    if value.get("manifest_id") != manifest_id(value):
        errors.append("evidence manifest identity is stale")
    return errors


def negative_self_tests(
    expected: dict[str, Any], schema: dict[str, Any], executable: Path
) -> list[str]:
    failures: list[str] = []

    stale = json.loads(json.dumps(expected))
    stale["reference"]["source_id"] = (
        "nmlt-source-v1:sha256:" + "0" * 64
    )
    stale["manifest_id"] = manifest_id(stale)
    if not binding_errors(stale, executable):
        failures.append("stale source-id control was accepted")

    stale_metatheory = json.loads(json.dumps(expected))
    stale_metatheory["metatheory"]["entrypoint_source_id"] = (
        "nmlt-source-v1:sha256:" + "0" * 64
    )
    stale_metatheory["manifest_id"] = manifest_id(stale_metatheory)
    if not binding_errors(stale_metatheory, executable):
        failures.append("stale metatheory source-id control was accepted")

    stale_lean_tcb = json.loads(json.dumps(expected))
    stale_lean_tcb["metatheory"]["lean_tcb_source_set_id"] = (
        "nmlt-source-set-v1:sha256:" + "0" * 64
    )
    stale_lean_tcb["manifest_id"] = manifest_id(stale_lean_tcb)
    if not binding_errors(stale_lean_tcb, executable):
        failures.append("stale Lean TCB source-set control was accepted")

    missing_lean_tcb = json.loads(json.dumps(expected))
    missing_lean_tcb["metatheory"].pop("lean_tcb_source_set_id")
    missing_lean_tcb["manifest_id"] = manifest_id(missing_lean_tcb)
    if not validate_instance(missing_lean_tcb, schema, schema):
        failures.append("missing Lean TCB source-set binding was schema accepted")

    stale_inventory = json.loads(json.dumps(expected))
    stale_inventory["implementation"]["trusted_component_source_ids"][
        "trusted_component_inventory"
    ] = "nmlt-source-v1:sha256:" + "0" * 64
    stale_inventory["manifest_id"] = manifest_id(stale_inventory)
    if not binding_errors(stale_inventory, executable):
        failures.append("stale trusted-component inventory control was accepted")

    missing_inventory = json.loads(json.dumps(expected))
    missing_inventory["implementation"]["trusted_component_source_ids"].pop(
        "trusted_component_inventory"
    )
    missing_inventory["manifest_id"] = manifest_id(missing_inventory)
    if not validate_instance(missing_inventory, schema, schema):
        failures.append("missing trusted-component inventory was schema accepted")

    omitted_path_inventory = load_tcb_inventory()
    lean_component = next(
        component
        for component in omitted_path_inventory["components"]
        if component.get("id") == "nmlt_lean.statements"
    )
    lean_component["paths"].remove(
        "mechanization/lean/NMLT/Counterexamples/CompositionCongruence.lean"
    )
    try:
        lean_tcb_paths(omitted_path_inventory)
    except ValueError:
        pass
    else:
        failures.append("omitted imported Lean statement path was accepted")

    forged_extraction = json.loads(json.dumps(expected))
    forged_extraction["metatheory"]["verified_rust_extraction"] = True
    forged_extraction["manifest_id"] = manifest_id(forged_extraction)
    if not validate_instance(forged_extraction, schema, schema):
        failures.append("forged verified-Rust-extraction control was schema accepted")

    missing_theorem = json.loads(json.dumps(expected))
    missing_theorem["metatheory"]["theorem_handles"] = missing_theorem[
        "metatheory"
    ]["theorem_handles"][:-1]
    missing_theorem["manifest_id"] = manifest_id(missing_theorem)
    if not validate_instance(missing_theorem, schema, schema):
        failures.append("missing metatheory theorem binding was schema accepted")

    forged = json.loads(json.dumps(expected))
    forged["reference"]["decision"] = "proved"
    forged["manifest_id"] = manifest_id(forged)
    if not validate_instance(forged, schema, schema):
        failures.append("forged proved-result control was schema accepted")

    missing = json.loads(json.dumps(expected))
    missing["negative_controls"] = missing["negative_controls"][:-1]
    missing["manifest_id"] = manifest_id(missing)
    if not validate_instance(missing, schema, schema):
        failures.append("missing negative-control evidence was schema accepted")

    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--update", action="store_true")
    arguments = parser.parse_args()
    try:
        expected, executable = produce()
        schema = load_json(SCHEMA)
        errors = validate_schema_definition(schema, "graded-evidence")
        errors.extend(validate_instance(expected, schema, schema))
        errors.extend(binding_errors(expected, executable))
        errors.extend(negative_self_tests(expected, schema, executable))
        if errors:
            raise ValueError("; ".join(errors))
        if arguments.update:
            OUTPUT.write_text(
                json.dumps(expected, indent=2, ensure_ascii=False) + "\n",
                encoding="utf-8",
            )
        actual = load_json(OUTPUT)
        if actual != expected:
            raise ValueError("persisted Phase 7 graded evidence differs from execution")
        errors = binding_errors(actual, executable)
        if errors:
            raise ValueError("; ".join(errors))
    except (
        OSError,
        ValueError,
        json.JSONDecodeError,
        DuplicateKey,
        KeyError,
        TypeError,
        subprocess.SubprocessError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(
        "ok: graded algebra, bounded resource claim, Lean bindings, "
        "identities, and controls"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
