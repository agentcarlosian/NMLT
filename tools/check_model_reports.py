#!/usr/bin/env python3
"""Generate and verify deterministic, source-bound provider model-check reports."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import subprocess
import sys
import os
from functools import cache
from pathlib import Path
from typing import Any

from validate_benchmark_integrity import (
    canonical_json,
    load_json,
    resolve_portable_path,
    source_id,
    validate_instance,
    validate_schema_definition,
)


ROOT = Path(__file__).resolve().parents[1]
SUITE = ROOT / "benchmarks/manifest.json"
RESULTS = ROOT / "benchmarks/results/provider-attempt"
RESULT_SCHEMA = ROOT / "schemas/benchmark-result-v1.schema.json"
REPORT_SCHEMA = ROOT / "schemas/model-check-report.schema.json"
RESULT_DOMAIN = b"NMLT-BENCHMARK-RESULT\0v1\0"
RESULT_PREFIX = "nmlt-benchmark-result-v1:sha256:"
ENGINE_SET_DOMAIN = b"NMLT-ENGINE-SOURCE-SET\0v1\0"
ENGINE_SET_PREFIX = "nmlt-engine-source-set-v1:sha256:"
ELABORATOR_SET_DOMAIN = b"NMLT-ELABORATOR-SOURCE-SET\0v1\0"
ELABORATOR_SET_PREFIX = "nmlt-elaborator-source-set-v1:sha256:"
KERNEL_SET_DOMAIN = b"NMLT-KERNEL-SOURCE-SET\0v1\0"
KERNEL_SET_PREFIX = "nmlt-kernel-source-set-v1:sha256:"
EXPECTED = {
    "provider-attempt-reference": "model_checked",
    "dispatch-before-authorize": "refuted",
    "blind-replay": "refuted",
    "response-binding": "refuted",
    "passing-selection": "refuted",
}


def result_id(value: dict[str, Any]) -> str:
    payload = dict(value)
    payload.pop("result_id", None)
    encoded = canonical_json(payload)
    digest = hashlib.sha256(RESULT_DOMAIN + len(encoded).to_bytes(8, "big") + encoded)
    return RESULT_PREFIX + digest.hexdigest()


def component_source_set_id(domain: bytes, prefix: str, crates: tuple[str, ...]) -> str:
    relative_paths = [
        Path("Cargo.toml"),
        Path("Cargo.lock"),
        Path("rust-toolchain.toml"),
    ]
    for crate in crates:
        crate_root = ROOT / "crates" / crate
        relative_paths.extend(path.relative_to(ROOT) for path in crate_root.rglob("*.rs"))
        relative_paths.append(Path("crates") / crate / "Cargo.toml")
    normalized = sorted(
        {str(path).encode("utf-8"): path for path in relative_paths}.items()
    )
    encoded = bytearray(domain)
    encoded.extend(len(normalized).to_bytes(8, "big"))
    for path_bytes, path in normalized:
        data = (ROOT / path).read_bytes()
        encoded.extend(len(path_bytes).to_bytes(8, "big"))
        encoded.extend(path_bytes)
        encoded.extend(len(data).to_bytes(8, "big"))
        encoded.extend(hashlib.sha256(data).digest())
    return prefix + hashlib.sha256(encoded).hexdigest()


def engine_source_set_id() -> str:
    return component_source_set_id(
        ENGINE_SET_DOMAIN,
        ENGINE_SET_PREFIX,
        (
            "nmlt-core",
            "nmlt-hir",
            "nmlt-ir",
            "nmlt-certificate",
            "nmlt-elaborate",
            "nmlt-kernel",
            "nmlt-compile",
            "nmlt-engine",
            "nmlt-cli",
        ),
    )


def elaborator_source_set_id() -> str:
    return component_source_set_id(
        ELABORATOR_SET_DOMAIN,
        ELABORATOR_SET_PREFIX,
        ("nmlt-core", "nmlt-hir", "nmlt-ir", "nmlt-certificate", "nmlt-elaborate", "nmlt-compile"),
    )


def kernel_source_set_id() -> str:
    return component_source_set_id(
        KERNEL_SET_DOMAIN,
        KERNEL_SET_PREFIX,
        ("nmlt-hir", "nmlt-ir", "nmlt-certificate", "nmlt-kernel"),
    )


def _u64(value: int) -> bytes:
    return value.to_bytes(8, "big")


def _bytes(value: bytes) -> bytes:
    return _u64(len(value)) + value


def expected_source_set_id(path_text: str, path: Path) -> str:
    source_digest = bytes.fromhex(source_id(path).rsplit(":", 1)[1])
    encoded = b"NMLT-SOURCE-SET\0v1\0" + _u64(1) + _bytes(path_text.encode()) + source_digest
    return "nmlt-source-set-v1:sha256:" + hashlib.sha256(encoded).hexdigest()


def expected_module_map_id(source_set: str, path_text: str) -> str:
    encoded = (
        b"NMLT-MODULE-MAP\0v1\0"
        + bytes.fromhex(source_set.rsplit(":", 1)[1])
        + _u64(1)
        + _bytes(b"Main")
        + _bytes(path_text.encode())
    )
    return "nmlt-module-map-v1:sha256:" + hashlib.sha256(encoded).hexdigest()


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


@cache
def model_checker_binary() -> Path:
    target = ROOT / "target/evidence"
    environment = os.environ.copy()
    environment.update(
        {
            "CARGO_INCREMENTAL": "0",
            "CARGO_TARGET_DIR": str(target),
            "SOURCE_DATE_EPOCH": "0",
            "RUSTFLAGS": f"--remap-path-prefix={ROOT}=/nmlt -Cdebuginfo=0",
        }
    )
    process = subprocess.run(
        ["cargo", "build", "--quiet", "--release", "-p", "nmlt-cli"],
        cwd=ROOT,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
    )
    if process.returncode != 0:
        raise ValueError(f"cannot build model checker: {process.stderr.strip()}")
    binary = target / "release/nmlt"
    if not binary.is_file():
        raise ValueError(f"model checker build did not create {binary}")
    return binary


def run_report(source: Path) -> dict[str, Any]:
    command = [
        str(model_checker_binary()),
        "model-check",
        "--json",
        str(source.relative_to(ROOT)),
    ]
    first = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
    if first.returncode != 0:
        raise ValueError(f"model checker failed for {source}: {first.stderr.strip()}")
    second = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
    if second.returncode != 0 or first.stdout != second.stdout:
        raise ValueError(f"nondeterministic model-check output for {source}")
    try:
        value = json.loads(first.stdout)
    except json.JSONDecodeError as error:
        raise ValueError(f"model checker emitted invalid JSON for {source}: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"model checker report for {source} is not an object")
    return value


@cache
def fresh_semantic_binding(source: Path) -> dict[str, Any]:
    value = run_report(source)
    binding = value.get("semantic_binding")
    if not isinstance(binding, dict):
        raise ValueError(f"model checker omitted semantic binding for {source}")
    return binding


def build_result(case: dict[str, Any]) -> dict[str, Any]:
    path_text = case["path"]
    path = ROOT / path_text
    calculated_source_id = source_id(path)
    if calculated_source_id != case["source_id"]:
        raise ValueError(f"suite has stale source ID for {case['id']}")
    report = run_report(path)
    expected = EXPECTED[case["id"]]
    if report.get("result") != expected:
        raise ValueError(
            f"{case['id']}: expected {expected}, observed {report.get('result')!r}"
        )
    validate_expected_witness(case, report)
    bounds = report["bounds"]
    value: dict[str, Any] = {
        "schema_version": "1.0.0",
        "result_id": "",
        "case_id": case["id"],
        "source": {"path": path_text, "source_id": calculated_source_id},
        "engine": {
            "name": "nmlt-explicit-state",
            "version": "0.0.1",
            "source_set_id": engine_source_set_id(),
            "elaborator_source_set_id": elaborator_source_set_id(),
            "kernel_source_set_id": kernel_source_set_id(),
            "executable_sha256": hashlib.sha256(model_checker_binary().read_bytes()).hexdigest(),
            "toolchain": rust_toolchain(),
            "target": rust_target(),
            "algorithm": "deterministic-bfs-v1",
            "report_schema": "model-check-report/1.1.0",
        },
        "configuration": {
            "max_states": bounds["max_states"],
            "max_depth": bounds["max_depth"],
            "action_order": "lexicographic-action-then-state",
            "terminal_behavior": "identity-stutter",
        },
        "assumptions": [
            "The kernel-checked M9-v1 core projection and i64 runtime arithmetic match the frozen source intent.",
            "A model_checked result requires exhaustion of the reachable frontier within both bounds.",
        ],
        "trusted_components": [
            "nmlt-core projection and nmlt-hir resolver",
            "nmlt-elaborate producer plus independently replaying nmlt-kernel",
            "nmlt-engine checked-core adapter, evaluator, and BFS explorer",
            "Rust standard library ordered collections and integer operations",
        ],
        "negative_controls": [
            "dispatch-before-authorize",
            "blind-replay",
            "response-binding",
            "passing-selection",
        ],
        "report": report,
    }
    value["result_id"] = result_id(value)
    return value


def validate_expected_witness(case: dict[str, Any], report: dict[str, Any]) -> None:
    references = case.get("expected_witnesses", [])
    if not references:
        if report["result"] == "refuted":
            raise ValueError(f"{case['id']}: refutation has no frozen witness oracle")
        return
    if len(references) != 1:
        raise ValueError(f"{case['id']}: v1 readback requires exactly one witness oracle")
    oracle = load_json(ROOT / references[0]["path"])
    refutations = [item for item in report["properties"] if item["result"] == "refuted"]
    if len(refutations) != 1 or refutations[0]["witness"] is None:
        raise ValueError(f"{case['id']}: expected exactly one observed refutation witness")
    actual_steps = refutations[0]["witness"]["steps"]
    expected_steps = oracle["steps"]
    if len(actual_steps) - 1 != oracle["minimal_transition_count"]:
        raise ValueError(f"{case['id']}: counterexample is not the frozen minimal length")
    if [step["action"] for step in actual_steps[1:]] != [
        step["action"] for step in expected_steps
    ]:
        raise ValueError(f"{case['id']}: observed action sequence differs from oracle")

    def check_observation(actual: dict[str, Any], expected: dict[str, Any], where: str) -> None:
        state = actual["state"]
        enabled_actions = set(actual["enabled_actions"])
        for name, value in expected.items():
            if name.endswith("_enabled"):
                action = name.removesuffix("_enabled")
                if (action in enabled_actions) != value:
                    raise ValueError(
                        f"{case['id']}: {where} derived field {name!r} expected "
                        f"{value!r}, observed {action in enabled_actions!r}"
                    )
                continue
            if state.get(name) != value:
                raise ValueError(
                    f"{case['id']}: {where} field {name!r} expected {value!r}, "
                    f"observed {state.get(name)!r}"
                )

    check_observation(actual_steps[0], oracle["initial_observation"], "initial")
    for index, (actual, frozen) in enumerate(zip(actual_steps[1:], expected_steps, strict=True)):
        check_observation(
            actual, frozen["resulting_observation"], f"step {index + 1}"
        )


def validate_result(
    value: dict[str, Any],
    expected: dict[str, Any] | None,
    result_schema: dict[str, Any],
    report_schema: dict[str, Any],
) -> list[str]:
    errors = validate_instance(value, result_schema, result_schema)
    errors.extend(validate_instance(value.get("report"), report_schema, report_schema, "$.report"))
    if value.get("result_id") != result_id(value):
        errors.append("$.result_id: stale canonical result identity")
    source = value.get("source", {})
    try:
        path = resolve_portable_path(source.get("path"))
    except (TypeError, ValueError) as error:
        errors.append(f"$.source.path: {error}")
    else:
        if not path.is_file():
            errors.append("$.source.path: source is not a regular file")
        elif source.get("source_id") != source_id(path):
            errors.append("$.source.source_id: stale exact-source identity")
        else:
            binding = value.get("report", {}).get("semantic_binding", {})
            calculated_source_set = expected_source_set_id(source.get("path"), path)
            if binding.get("source_set_id") != calculated_source_set:
                errors.append("$.report.semantic_binding.source_set_id: stale exact source-set identity")
            calculated_module_map = expected_module_map_id(calculated_source_set, source.get("path"))
            if binding.get("module_map_id") != calculated_module_map:
                errors.append("$.report.semantic_binding.module_map_id: stale logical-module binding")
            try:
                fresh_binding = fresh_semantic_binding(path)
            except ValueError as error:
                errors.append(f"$.report.semantic_binding: cannot replay: {error}")
            else:
                if binding != fresh_binding:
                    errors.append("$.report.semantic_binding: differs from fresh kernel replay")
    engine = value.get("engine", {})
    if engine.get("source_set_id") != engine_source_set_id():
        errors.append("$.engine.source_set_id: stale engine source-set identity")
    if engine.get("elaborator_source_set_id") != elaborator_source_set_id():
        errors.append("$.engine.elaborator_source_set_id: stale elaborator source-set identity")
    if engine.get("kernel_source_set_id") != kernel_source_set_id():
        errors.append("$.engine.kernel_source_set_id: stale kernel source-set identity")
    if engine.get("executable_sha256") != hashlib.sha256(
        model_checker_binary().read_bytes()
    ).hexdigest():
        errors.append("$.engine.executable_sha256: stale executable identity")
    if engine.get("toolchain") != rust_toolchain():
        errors.append("$.engine.toolchain: stale Rust toolchain identity")
    if engine.get("target") != rust_target():
        errors.append("$.engine.target: stale Rust target identity")
    report = value.get("report", {})
    if isinstance(report, dict):
        overall = report.get("result")
        properties = report.get("properties", [])
        if not properties:
            errors.append("$.report: model checking requires at least one property")
        if overall == "model_checked" and report.get("complete") is not True:
            errors.append("$.report: model_checked requires complete frontier exhaustion")
        if overall == "refuted" and not any(
            isinstance(item, dict)
            and item.get("result") == "refuted"
            and isinstance(item.get("witness"), dict)
            for item in properties
        ):
            errors.append("$.report: refuted requires a structured counterexample")
    if expected is not None and value != expected:
        errors.append("persisted bytes do not describe the current deterministic execution")
    return errors


def negative_self_tests(
    generated: dict[str, dict[str, Any]],
    result_schema: dict[str, Any],
    report_schema: dict[str, Any],
) -> list[str]:
    failures: list[str] = []
    reference = generated["provider-attempt-reference"]
    forged_proof = copy.deepcopy(reference)
    forged_proof["report"]["result"] = "proved"
    forged_proof["result_id"] = result_id(forged_proof)
    if not validate_result(forged_proof, None, result_schema, report_schema):
        failures.append("bounded result was promotable to proved")

    stale_source = copy.deepcopy(reference)
    stale_source["source"]["source_id"] = (
        "nmlt-source-v1:sha256:" + "0" * 64
    )
    stale_source["result_id"] = result_id(stale_source)
    if not validate_result(stale_source, None, result_schema, report_schema):
        failures.append("stale source binding was accepted")

    mutant = generated["dispatch-before-authorize"]
    missing_witness = copy.deepcopy(mutant)
    for property_result in missing_witness["report"]["properties"]:
        property_result["witness"] = None
    missing_witness["result_id"] = result_id(missing_witness)
    if not validate_result(missing_witness, None, result_schema, report_schema):
        failures.append("refutation without a witness was accepted")

    stale_engine = copy.deepcopy(reference)
    stale_engine["engine"]["source_set_id"] = (
        "nmlt-engine-source-set-v1:sha256:" + "0" * 64
    )
    stale_engine["result_id"] = result_id(stale_engine)
    if not validate_result(stale_engine, None, result_schema, report_schema):
        failures.append("stale engine binding was accepted")

    forged_certificate = copy.deepcopy(reference)
    forged_certificate["report"]["semantic_binding"]["certificate_id"] = (
        "nmlt-elaboration-certificate-v1:sha256:" + "0" * 64
    )
    forged_certificate["result_id"] = result_id(forged_certificate)
    if not validate_result(forged_certificate, None, result_schema, report_schema):
        failures.append("forged certificate identity was accepted by readback")

    stale_kernel = copy.deepcopy(reference)
    stale_kernel["engine"]["kernel_source_set_id"] = (
        "nmlt-kernel-source-set-v1:sha256:" + "0" * 64
    )
    stale_kernel["result_id"] = result_id(stale_kernel)
    if not validate_result(stale_kernel, None, result_schema, report_schema):
        failures.append("stale kernel identity was accepted")

    propertyless = copy.deepcopy(reference)
    propertyless["report"]["properties"] = []
    propertyless["result_id"] = result_id(propertyless)
    if not validate_result(propertyless, None, result_schema, report_schema):
        failures.append("propertyless model_checked report was accepted")

    incomplete = copy.deepcopy(reference)
    incomplete["report"]["complete"] = False
    incomplete["result_id"] = result_id(incomplete)
    if not validate_result(incomplete, None, result_schema, report_schema):
        failures.append("incomplete model_checked report was accepted")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--update",
        action="store_true",
        help="replace persisted generated reports after an intentional semantic change",
    )
    args = parser.parse_args()

    try:
        suite = load_json(SUITE)
        result_schema = load_json(RESULT_SCHEMA)
        report_schema = load_json(REPORT_SCHEMA)
        schema_errors = validate_schema_definition(result_schema, "benchmark-result")
        schema_errors.extend(validate_schema_definition(report_schema, "model-check-report"))
        if schema_errors:
            raise ValueError("; ".join(schema_errors))
        cases = {case["id"]: case for case in suite["cases"]}
        generated = {case_id: build_result(cases[case_id]) for case_id in EXPECTED}
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    if args.update:
        RESULTS.mkdir(parents=True, exist_ok=True)
        for case_id, value in generated.items():
            (RESULTS / f"{case_id}.json").write_text(
                json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
            )

    errors: list[str] = []
    for case_id, expected in generated.items():
        path = RESULTS / f"{case_id}.json"
        try:
            actual = load_json(path)
        except (OSError, ValueError, json.JSONDecodeError) as error:
            errors.append(f"{path.relative_to(ROOT)}: {error}")
            continue
        for error in validate_result(actual, expected, result_schema, report_schema):
            errors.append(f"{path.relative_to(ROOT)}: {error}")
    errors.extend(
        f"negative self-test: {error}"
        for error in negative_self_tests(generated, result_schema, report_schema)
    )

    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(
        "ok: 5 deterministic source-bound model-check reports "
        "(1 model_checked, 4 refuted with witnesses)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
