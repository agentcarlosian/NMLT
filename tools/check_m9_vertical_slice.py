#!/usr/bin/env python3
"""Reproduce the promoted M9 source-to-checked-engine boundary."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
EXPECTED = {
    "examples/agents/trust_chain.nmlt": (1, "NMLT-M9-PORT"),
    "examples/basics/boolean_toggle.nmlt": (0, "type_checked:"),
    "examples/concurrency/two_process_mutex.nmlt": (1, "NMLT-M9-SELECTED-UPDATE"),
    "examples/distributed/two_phase_commit.nmlt": (1, "NMLT-M9-SYSTEM-CONSTANT"),
    "examples/hyperbook/one_bit_clock.nmlt": (1, "NMLT_COMPILE_RESOLUTION"),
    "examples/math/euclid.nmlt": (1, "NMLT-M9-SYSTEM-INPUT"),
    "examples/refinement/bounded_channel.nmlt": (1, "NMLT-M9-HIDING"),
    "examples/resources/token_bucket.nmlt": (1, "NMLT-M9-ACTION-GRADE"),
    "examples/runtime/durable_controller.nmlt": (0, "type_checked:"),
    "examples/technicus/provider_attempt.nmlt": (0, "type_checked:"),
}


def run(command: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)


def main() -> int:
    errors: list[str] = []
    build = run(["cargo", "build", "--quiet", "-p", "nmlt-cli"])
    if build.returncode:
        print(build.stderr)
        return build.returncode
    binary = ROOT / "target/debug/nmlt"

    registry = json.loads((ROOT / "examples/canonical-v1.json").read_text())
    paths = {item["path"] for item in registry["examples"]}
    if paths != set(EXPECTED):
        errors.append("canonical registry membership differs from the M9 outcome table")

    for path, (expected_code, marker) in EXPECTED.items():
        result = run([str(binary), "typecheck", path])
        combined = result.stdout + result.stderr
        if result.returncode != expected_code or marker not in combined:
            errors.append(
                f"{path}: expected exit {expected_code} and {marker!r}; "
                f"got exit {result.returncode}: {combined.strip()}"
            )

    engine = ROOT / "crates/nmlt-engine/src"
    if (engine / "parser.rs").exists():
        errors.append("the removed engine parser has reappeared")
    public_api = (engine / "lib.rs").read_text()
    if "from_checked" not in public_api or "parse" in public_api:
        errors.append("engine public API is not exclusively checked-program based")
    checked_adapter = (engine / "checked.rs").read_text()
    if "CheckedProgram" not in checked_adapter:
        errors.append("engine adapter is not bound to CheckedProgram")

    schema = json.loads((ROOT / "schemas/model-check-report.schema.json").read_text())
    required = schema["properties"]["semantic_binding"]["required"]
    for field in (
        "source_set_id",
        "module_map_id",
        "surface_program_id",
        "resolved_hir_id",
        "core_program_id",
        "ruleset_bundle_id",
        "resource_policy_id",
        "certificate_id",
        "kernel_profile_id",
    ):
        if field not in required:
            errors.append(f"model report does not require semantic binding {field}")

    for required_test in (
        "canonical_certificate_round_trip_and_preallocation_controls",
        "unknown_duplicate_and_missing_coverage_inputs_are_rejected",
        "cycles_unreachable_nodes_and_noncanonical_order_are_rejected",
        "canonically_resealed_semantic_forgery_is_rejected",
    ):
        if not any(
            required_test in path.read_text()
            for path in (ROOT / "crates/nmlt-kernel").rglob("*.rs")
        ):
            errors.append(f"kernel negative control missing: {required_test}")

    if errors:
        for error in errors:
            print(f"error: {error}")
        return 1
    accepted = sum(code == 0 for code, _ in EXPECTED.values())
    print(
        f"M9 vertical slice passed: 10 canonical outcomes "
        f"({accepted} accepted, {10 - accepted} explicit boundaries), "
        "checked-only engine, semantic identity schema, and kernel controls"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
