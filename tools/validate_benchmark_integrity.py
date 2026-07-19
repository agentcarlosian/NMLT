#!/usr/bin/env python3
"""Validate the frozen provider-attempt benchmark without third-party packages.

This is deliberately a narrow validator, not a general JSON Schema library. It
implements every keyword used by the five benchmark-integrity schemas and
fails if those schemas introduce an unsupported keyword.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "benchmarks" / "manifest.json"
RESULT_SCHEMA_PATH = ROOT / "schemas" / "benchmark-result-v1.schema.json"
MODEL_REPORT_SCHEMA_PATH = ROOT / "schemas" / "model-check-report.schema.json"
RESULT_DOMAIN = b"NMLT-BENCHMARK-RESULT\0v1\0"
RESULT_PREFIX = "nmlt-benchmark-result-v1:sha256:"

SCHEMA_PATHS = {
    "suite": ROOT / "schemas" / "benchmark-suite-v2.schema.json",
    "intent": ROOT / "schemas" / "benchmark-intent-capsule-v1.schema.json",
    "counterexample": ROOT / "schemas" / "expected-counterexample-v1.schema.json",
    "property": ROOT / "schemas" / "benchmark-property-v1.schema.json",
    "provenance": ROOT / "schemas" / "benchmark-provenance-v1.schema.json",
}

EXPECTED_CASES = (
    "provider-attempt-reference",
    "dispatch-before-authorize",
    "blind-replay",
    "response-binding",
    "passing-selection",
)
EXPECTED_CONTROLS = (
    "malformed-unclosed-system",
    "vacuous-dispatch-property",
    "weakened-dispatch-invariant",
    "one-shot-replay-regression",
)
EXPECTED_PROPERTY_HANDLES = (
    "ProviderAttempt.DispatchRequiresArm",
    "ProviderAttempt.NoBlindReplay",
    "ProviderAttempt.EvaluationRequiresIntactResponse",
    "ProviderAttempt.SelectionRequiresPass",
)
CORRECTED_NO_BLIND_REPLAY = (
    "always(phase == indeterminate implies not enabled(dispatch))"
)
HISTORICAL_NO_BLIND_REPLAY = (
    "always(phase == indeterminate implies next(not enabled(dispatch)))"
)

FROZEN_SOURCE_SET_ID = (
    "nmlt-source-set-v1:sha256:"
    "cc43fcb4d0fe8308bd5abfceed809b938351479ec9926c4507c2248df9fa6334"
)
FROZEN_SUITE_ID = "provider-attempt-seeded-defects-v2"
FROZEN_PROVENANCE_ID = (
    "nmlt-benchmark-provenance-v1:sha256:"
    "957b1a6a5727b50ff19fa0b90cc3ec8413c7e31fad4d849d9498d685080a7105"
)
FROZEN_PROPERTY_IDS = {
    "ProviderAttempt.DispatchRequiresArm": (
        "nmlt-benchmark-property-v1:sha256:"
        "52ad13dda5c9023da2ac4142673e4d24db34a98e2d201b9b9795fcbfae4763bd"
    ),
    "ProviderAttempt.NoBlindReplay": (
        "nmlt-benchmark-property-v1:sha256:"
        "c4b9cebc0ed34632f94db260c3c221832ceb243095268b3a54e04e5ff93a3fbe"
    ),
    "ProviderAttempt.EvaluationRequiresIntactResponse": (
        "nmlt-benchmark-property-v1:sha256:"
        "15a3427faba0a4cf568c7bb01b98cf5ff750430d605d20c04dc8df80e7c64eb0"
    ),
    "ProviderAttempt.SelectionRequiresPass": (
        "nmlt-benchmark-property-v1:sha256:"
        "03a2a2ee37e3191d1f81ca722e60fb600efc60310c4855196904f0fd6ada7ba6"
    ),
}
FROZEN_CASE_BINDINGS = {
    "provider-attempt-reference": {
        "source_id": "nmlt-source-v1:sha256:bd79e2b9928732945b2daaab83021404677a9dbdb506aaff406300e6a9858a3c",
        "capsule_id": "nmlt-intent-capsule-v1:sha256:e63b55d76f75f0d79bac1665916f9898d5749ae1130be4048ac46144f3f171af",
        "properties": tuple(FROZEN_PROPERTY_IDS.values()),
        "witness_ids": (),
        "oracle": "holds_within_frozen_future_bounds",
        "intended": "model_checked",
        "current": "model_checked",
        "support": "current",
        "result_path": "benchmarks/results/provider-attempt/provider-attempt-reference.json",
        "result_id": "nmlt-benchmark-result-v1:sha256:1f9e4cd3b51811dea570bf6ee28bf3de0a196076a467e9540229206382be90eb",
    },
    "dispatch-before-authorize": {
        "source_id": "nmlt-source-v1:sha256:5755f61bb3fc209fb06161660cef7cb89b542b2d8e32aed412360146c935da66",
        "capsule_id": "nmlt-intent-capsule-v1:sha256:960da41f601d0953cfe978ee7d17636839a4e20153362c8ad943e39dddf59d48",
        "properties": (FROZEN_PROPERTY_IDS["ProviderAttempt.DispatchRequiresArm"],),
        "witness_ids": (
            "nmlt-expected-counterexample-v1:sha256:ef750d2addd7c3dacb4d0095890c6f96037eb416238c98d5ff60ae1814f03627",
        ),
        "oracle": "refuted",
        "intended": "refuted",
        "current": "refuted",
        "support": "current",
        "result_path": "benchmarks/results/provider-attempt/dispatch-before-authorize.json",
        "result_id": "nmlt-benchmark-result-v1:sha256:2c1cd4bd09712b0f5289c092a87c1e11cedefad00c3675975d6e5893de767cd9",
    },
    "blind-replay": {
        "source_id": "nmlt-source-v1:sha256:1d61387c8998f8e2bda759a2ba05d66201507e522bbedf5bc4483b3ed4afaaac",
        "capsule_id": "nmlt-intent-capsule-v1:sha256:a10630be4fb8632c0fb48781611d813f13ae10900a8670e97cb987316a529799",
        "properties": (FROZEN_PROPERTY_IDS["ProviderAttempt.NoBlindReplay"],),
        "witness_ids": (
            "nmlt-expected-counterexample-v1:sha256:da4bbd49ff308c67dc37ae24f5f7ed5e6ff11eb1e12312a313a6365a72cacf2e",
        ),
        "oracle": "refuted",
        "intended": "refuted",
        "current": "refuted",
        "support": "current",
        "result_path": "benchmarks/results/provider-attempt/blind-replay.json",
        "result_id": "nmlt-benchmark-result-v1:sha256:ff00c850c6a04cfcc1e6bca12ce06db5e50fad35f18a2eed28edd2becf3aec71",
    },
    "response-binding": {
        "source_id": "nmlt-source-v1:sha256:3f491ca3c289a1d5eff7d11519fb4509e60345aa8eb60819baa1ad442460462e",
        "capsule_id": "nmlt-intent-capsule-v1:sha256:6d3ad04c45c3a8faf555de46c14132de769216a1c8ef82cbe191e4c88b9260a6",
        "properties": (
            FROZEN_PROPERTY_IDS["ProviderAttempt.EvaluationRequiresIntactResponse"],
        ),
        "witness_ids": (
            "nmlt-expected-counterexample-v1:sha256:ebae96b362b15cff69eb2e8f8631b1e19d6ffc703494d609fc98b4b1384155b7",
        ),
        "oracle": "refuted",
        "intended": "refuted",
        "current": "refuted",
        "support": "current",
        "result_path": "benchmarks/results/provider-attempt/response-binding.json",
        "result_id": "nmlt-benchmark-result-v1:sha256:c1bd34a64dde9e4b39e1ed5c28e0857dd53c618ec888f82e2c11ed044321a73a",
    },
    "passing-selection": {
        "source_id": "nmlt-source-v1:sha256:ae395e8f3941be5647d30015e7689332e3bcf00948329e3d8b88960b65e71f37",
        "capsule_id": "nmlt-intent-capsule-v1:sha256:7072ec1def14b78717435da443be4ef639381117acf8603bf0d2e9348592b32c",
        "properties": (FROZEN_PROPERTY_IDS["ProviderAttempt.SelectionRequiresPass"],),
        "witness_ids": (
            "nmlt-expected-counterexample-v1:sha256:45bb21364362c8ab809b4366ac099ee2084fe89380f54df5f85ed1deab3e9c63",
        ),
        "oracle": "refuted",
        "intended": "refuted",
        "current": "refuted",
        "support": "current",
        "result_path": "benchmarks/results/provider-attempt/passing-selection.json",
        "result_id": "nmlt-benchmark-result-v1:sha256:a3b47575ffdf4c1e91da3ee12c38f640b16c681c5b29cc7188961229734e3189",
    },
}
FROZEN_CONTROL_BINDINGS = {
    "malformed-unclosed-system": {
        "source_id": "nmlt-source-v1:sha256:d495f69886c4e5e2626ef8d4034315aa2f6d0f17f1f8b9204dea730bf8c1516a",
        "class": "malformed",
        "outcome": "frontend_rejected",
        "current": "unknown",
        "support": "current",
        "property_id": None,
    },
    "vacuous-dispatch-property": {
        "source_id": "nmlt-source-v1:sha256:db4feefcb0b6748dfca2ef395a85635b1d7ac542f81842c558233697a6b9c352",
        "class": "vacuous_property",
        "outcome": "integrity_rejected_for_vacuity",
        "current": "unknown",
        "support": "fixture_only",
        "property_id": FROZEN_PROPERTY_IDS["ProviderAttempt.DispatchRequiresArm"],
    },
    "weakened-dispatch-invariant": {
        "source_id": "nmlt-source-v1:sha256:414de2c8b70a991f32999f67307e0375640ea952a305d5023a91df2cf2301c42",
        "class": "weakened_invariant",
        "outcome": "integrity_rejected_for_property_identity_mismatch",
        "current": "unknown",
        "support": "fixture_only",
        "property_id": FROZEN_PROPERTY_IDS["ProviderAttempt.DispatchRequiresArm"],
    },
    "one-shot-replay-regression": {
        "source_id": "nmlt-source-v1:sha256:d30e6cb68ddbcac3875ec8b72071b3f4c3d6b4f852403856c8624cd0a3314d4b",
        "class": "semantic_regression",
        "outcome": "model_checker_refuted",
        "current": "refuted",
        "support": "current",
        "property_id": FROZEN_PROPERTY_IDS["ProviderAttempt.NoBlindReplay"],
    },
}

SOURCE_PREFIX = "nmlt-source-v1:sha256:"
SOURCE_SET_PREFIX = "nmlt-source-set-v1:sha256:"
ARTIFACT_DOMAINS = {
    "property_id": (
        b"NMLT-BENCHMARK-PROPERTY\0v1\0",
        "nmlt-benchmark-property-v1:sha256:",
    ),
    "capsule_id": (
        b"NMLT-INTENT-CAPSULE\0v1\0",
        "nmlt-intent-capsule-v1:sha256:",
    ),
    "witness_id": (
        b"NMLT-EXPECTED-COUNTEREXAMPLE\0v1\0",
        "nmlt-expected-counterexample-v1:sha256:",
    ),
    "provenance_id": (
        b"NMLT-BENCHMARK-PROVENANCE\0v1\0",
        "nmlt-benchmark-provenance-v1:sha256:",
    ),
}

SUPPORTED_SCHEMA_KEYS = {
    "$schema",
    "$id",
    "$defs",
    "$ref",
    "title",
    "type",
    "additionalProperties",
    "required",
    "properties",
    "const",
    "enum",
    "pattern",
    "minLength",
    "minItems",
    "uniqueItems",
    "items",
    "minimum",
    "oneOf",
    "allOf",
    "if",
    "then",
    "$comment",
}


class DuplicateKey(ValueError):
    """Raised for JSON objects whose member names are not unique."""


def _object_without_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise DuplicateKey(f"duplicate JSON member {key!r}")
        value[key] = item
    return value


def _reject_constant(value: str) -> None:
    raise ValueError(f"non-finite JSON number {value!r} is forbidden")


def load_json(path: Path) -> Any:
    return json.loads(
        path.read_text(encoding="utf-8"),
        object_pairs_hook=_object_without_duplicates,
        parse_constant=_reject_constant,
    )


def canonical_json(value: Any) -> bytes:
    """Encode the benchmark's integer/string/bool/null canonical JSON subset."""

    def reject_float(item: Any, location: str = "$") -> None:
        if isinstance(item, float):
            raise ValueError(f"floating-point value forbidden at {location}")
        if isinstance(item, dict):
            for key, child in item.items():
                reject_float(child, f"{location}.{key}")
        elif isinstance(item, list):
            for index, child in enumerate(item):
                reject_float(child, f"{location}[{index}]")

    reject_float(value)
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def artifact_id(value: dict[str, Any], identity_member: str) -> str:
    domain, prefix = ARTIFACT_DOMAINS[identity_member]
    payload = dict(value)
    payload.pop(identity_member, None)
    encoded = canonical_json(payload)
    digest = hashlib.sha256(domain + len(encoded).to_bytes(8, "big") + encoded)
    return prefix + digest.hexdigest()


def benchmark_result_id(value: dict[str, Any]) -> str:
    payload = dict(value)
    payload.pop("result_id", None)
    encoded = canonical_json(payload)
    digest = hashlib.sha256(
        RESULT_DOMAIN + len(encoded).to_bytes(8, "big") + encoded
    )
    return RESULT_PREFIX + digest.hexdigest()


def source_digest(data: bytes) -> bytes:
    preimage = b"NMLT-SOURCE\0v1\0" + len(data).to_bytes(8, "big") + data
    return hashlib.sha256(preimage).digest()


def source_id(path: Path) -> str:
    return SOURCE_PREFIX + source_digest(path.read_bytes()).hex()


def source_set_id(paths: list[str]) -> str:
    encoded = bytearray(b"NMLT-SOURCE-SET\0v1\0")
    encoded.extend(len(paths).to_bytes(8, "big"))
    for path_text in sorted(paths, key=lambda item: item.encode("utf-8")):
        path_bytes = path_text.encode("utf-8")
        encoded.extend(len(path_bytes).to_bytes(8, "big"))
        encoded.extend(path_bytes)
        encoded.extend(source_digest(resolve_portable_path(path_text).read_bytes()))
    return SOURCE_SET_PREFIX + hashlib.sha256(encoded).hexdigest()


def resolve_portable_path(path_text: str) -> Path:
    path = Path(path_text)
    if path.is_absolute() or not path.parts or any(part in {"", ".", ".."} for part in path.parts):
        raise ValueError(f"not a portable repository-relative path: {path_text!r}")
    resolved = (ROOT / path).resolve()
    try:
        resolved.relative_to(ROOT.resolve())
    except ValueError as error:
        raise ValueError(f"path escapes repository: {path_text!r}") from error
    return resolved


def schema_type_matches(value: Any, expected: str) -> bool:
    checks = {
        "object": lambda item: isinstance(item, dict),
        "array": lambda item: isinstance(item, list),
        "string": lambda item: isinstance(item, str),
        "integer": lambda item: isinstance(item, int) and not isinstance(item, bool),
        "boolean": lambda item: isinstance(item, bool),
        "null": lambda item: item is None,
    }
    return expected in checks and checks[expected](value)


def resolve_ref(root_schema: dict[str, Any], ref: str) -> dict[str, Any]:
    if not ref.startswith("#/"):
        raise ValueError(f"only local JSON Pointer refs are supported: {ref}")
    node: Any = root_schema
    for raw_part in ref[2:].split("/"):
        part = raw_part.replace("~1", "/").replace("~0", "~")
        node = node[part]
    if not isinstance(node, dict):
        raise ValueError(f"schema ref does not resolve to an object: {ref}")
    return node


def validate_instance(
    value: Any,
    schema: dict[str, Any],
    root_schema: dict[str, Any],
    location: str = "$",
) -> list[str]:
    if "$ref" in schema:
        return validate_instance(value, resolve_ref(root_schema, schema["$ref"]), root_schema, location)

    errors: list[str] = []
    if "oneOf" in schema:
        matches = [
            candidate
            for candidate in schema["oneOf"]
            if not validate_instance(value, candidate, root_schema, location)
        ]
        if len(matches) != 1:
            errors.append(f"{location}: expected exactly one oneOf branch, matched {len(matches)}")
    for branch in schema.get("allOf", []):
        errors.extend(validate_instance(value, branch, root_schema, location))
    if "if" in schema and not validate_instance(value, schema["if"], root_schema, location):
        errors.extend(validate_instance(value, schema.get("then", {}), root_schema, location))
    expected_type = schema.get("type")
    if expected_type is not None:
        accepted_types = expected_type if isinstance(expected_type, list) else [expected_type]
        if not any(schema_type_matches(value, item) for item in accepted_types):
            return [f"{location}: expected type {expected_type!r}, got {type(value).__name__}"]

    if "const" in schema and value != schema["const"]:
        errors.append(f"{location}: expected constant {schema['const']!r}, got {value!r}")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{location}: {value!r} is not in {schema['enum']!r}")

    if isinstance(value, str):
        if len(value) < schema.get("minLength", 0):
            errors.append(f"{location}: string is shorter than minLength")
        pattern = schema.get("pattern")
        if pattern is not None and re.search(pattern, value) is None:
            errors.append(f"{location}: {value!r} does not match {pattern!r}")

    if isinstance(value, int) and not isinstance(value, bool):
        if "minimum" in schema and value < schema["minimum"]:
            errors.append(f"{location}: {value} is below minimum {schema['minimum']}")

    if isinstance(value, list):
        if len(value) < schema.get("minItems", 0):
            errors.append(f"{location}: array is shorter than minItems")
        if schema.get("uniqueItems"):
            serialized = [canonical_json(item) for item in value]
            if len(serialized) != len(set(serialized)):
                errors.append(f"{location}: array items are not unique")
        item_schema = schema.get("items")
        if item_schema is not None:
            for index, item in enumerate(value):
                errors.extend(
                    validate_instance(item, item_schema, root_schema, f"{location}[{index}]")
                )

    if isinstance(value, dict):
        properties = schema.get("properties", {})
        for required in schema.get("required", []):
            if required not in value:
                errors.append(f"{location}: missing required member {required!r}")
        if schema.get("additionalProperties") is False:
            for key in value:
                if key not in properties:
                    errors.append(f"{location}: unexpected member {key!r}")
        for key, child_schema in properties.items():
            if key in value:
                errors.extend(
                    validate_instance(value[key], child_schema, root_schema, f"{location}.{key}")
                )
    return errors


def validate_schema_definition(schema: dict[str, Any], name: str) -> list[str]:
    errors: list[str] = []
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        errors.append(f"schema {name}: draft must be 2020-12")
    if not isinstance(schema.get("$id"), str) or not schema["$id"].startswith(
        "https://nmlt.dev/schemas/"
    ):
        errors.append(f"schema {name}: missing canonical NMLT $id")

    def visit(node: Any, location: str) -> None:
        if isinstance(node, dict):
            for key, child in node.items():
                if key not in SUPPORTED_SCHEMA_KEYS and location != "$.$defs" and location != "$.properties":
                    errors.append(f"schema {name} {location}: unsupported keyword {key!r}")
                child_location = f"{location}.{key}"
                if key in {"properties", "$defs"}:
                    for member, member_schema in child.items():
                        visit(member_schema, f"{child_location}.{member}")
                elif key not in {"required", "enum", "type"}:
                    visit(child, child_location)
        elif isinstance(node, list):
            for index, child in enumerate(node):
                visit(child, f"{location}[{index}]")

    # Schema member names live under properties/$defs and are not keywords.
    for key, child in schema.items():
        if key not in SUPPORTED_SCHEMA_KEYS:
            errors.append(f"schema {name} $: unsupported keyword {key!r}")
        if key in {"properties", "$defs"}:
            for member, member_schema in child.items():
                visit(member_schema, f"$.{key}.{member}")
        elif key not in {"required", "enum", "type"}:
            visit(child, f"$.{key}")
    return errors


def normalized_source(path: Path) -> str:
    return re.sub(r"\s+", " ", path.read_text(encoding="utf-8")).strip()


def delimiter_depth(source: str) -> int:
    """Return brace depth for these controls while ignoring strings/comments."""

    depth = 0
    in_string = False
    escaped = False
    for line in source.splitlines():
        code = line.split("//", 1)[0]
        for character in code:
            if in_string:
                if escaped:
                    escaped = False
                elif character == "\\":
                    escaped = True
                elif character == '"':
                    in_string = False
            elif character == '"':
                in_string = True
            elif character == "{":
                depth += 1
            elif character == "}":
                depth -= 1
                if depth < 0:
                    return depth
    return depth


def load_schemas() -> tuple[dict[str, dict[str, Any]], list[str]]:
    schemas: dict[str, dict[str, Any]] = {}
    errors: list[str] = []
    for name, path in SCHEMA_PATHS.items():
        try:
            schema = load_json(path)
        except (OSError, UnicodeError, ValueError, json.JSONDecodeError) as error:
            errors.append(f"cannot load schema {path.relative_to(ROOT)}: {error}")
            continue
        if not isinstance(schema, dict):
            errors.append(f"schema {path.relative_to(ROOT)} must be an object")
            continue
        schemas[name] = schema
        errors.extend(validate_schema_definition(schema, name))
    return schemas, errors


def validate_repository(manifest_override: dict[str, Any] | None = None) -> list[str]:
    schemas, errors = load_schemas()
    if errors:
        return errors
    try:
        result_schema = load_json(RESULT_SCHEMA_PATH)
        model_report_schema = load_json(MODEL_REPORT_SCHEMA_PATH)
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot load model-result schemas: {error}"]
    errors.extend(validate_schema_definition(result_schema, "benchmark-result"))
    errors.extend(validate_schema_definition(model_report_schema, "model-check-report"))
    if errors:
        return errors
    try:
        manifest = manifest_override or load_json(MANIFEST_PATH)
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot load benchmarks/manifest.json: {error}"]

    errors.extend(validate_instance(manifest, schemas["suite"], schemas["suite"]))
    if errors:
        return errors

    if manifest["suite_id"] != FROZEN_SUITE_ID:
        errors.append("manifest: suite identity differs from the frozen v2 correction")
    if tuple(case["id"] for case in manifest["cases"]) != EXPECTED_CASES:
        errors.append("manifest: the five frozen case IDs or order changed")
    if tuple(control["id"] for control in manifest["controls"]) != EXPECTED_CONTROLS:
        errors.append("manifest: the four frozen control IDs or order changed")
    if manifest["source_set_id"] != FROZEN_SOURCE_SET_ID:
        errors.append("manifest: source-set identity differs from the frozen v2 oracle")

    provenance_ref = manifest["provenance"]
    try:
        provenance = load_json(resolve_portable_path(provenance_ref["path"]))
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError) as error:
        return errors + [f"cannot load provenance: {error}"]
    errors.extend(validate_instance(provenance, schemas["provenance"], schemas["provenance"], "$.provenance_artifact"))
    expected_provenance_id = artifact_id(provenance, "provenance_id")
    if provenance.get("provenance_id") != expected_provenance_id:
        errors.append(f"provenance: stale identity; expected {expected_provenance_id}")
    if provenance_ref["provenance_id"] != provenance.get("provenance_id"):
        errors.append("manifest: provenance reference does not match the artifact")
    if provenance.get("provenance_id") != FROZEN_PROVENANCE_ID:
        errors.append("provenance: artifact differs from the frozen source-corpus record")

    properties: dict[str, dict[str, Any]] = {}
    property_paths: set[str] = set()
    for property_ref in manifest["property_contracts"]:
        path_text = property_ref["path"]
        property_paths.add(path_text)
        try:
            value = load_json(resolve_portable_path(path_text))
        except (OSError, UnicodeError, ValueError, json.JSONDecodeError) as error:
            errors.append(f"cannot load property {path_text}: {error}")
            continue
        errors.extend(validate_instance(value, schemas["property"], schemas["property"], f"$.property[{path_text!r}]"))
        expected = artifact_id(value, "property_id")
        if value.get("property_id") != expected:
            errors.append(f"property {path_text}: stale identity; expected {expected}")
        if property_ref["property_id"] != value.get("property_id"):
            errors.append(f"manifest: property reference mismatch for {path_text}")
        if value.get("property_id") in properties:
            errors.append(f"manifest: duplicate property identity {value.get('property_id')}")
        properties[value.get("property_id", path_text)] = value
        frozen_property_id = FROZEN_PROPERTY_IDS.get(value.get("handle"))
        if frozen_property_id != value.get("property_id"):
            errors.append(f"property {path_text}: differs from the frozen v2 property oracle")
    if tuple(value.get("handle") for value in properties.values()) != EXPECTED_PROPERTY_HANDLES:
        errors.append("manifest: property handles or order differ from the v2 freeze")
    no_blind_replay = properties.get(
        FROZEN_PROPERTY_IDS["ProviderAttempt.NoBlindReplay"], {}
    )
    if no_blind_replay.get("nmlt_formula") != CORRECTED_NO_BLIND_REPLAY:
        errors.append(
            "NoBlindReplay: v2 must check enabledness in the current indeterminate state"
        )

    case_paths: list[str] = []
    witness_paths: set[str] = set()
    intent_paths: set[str] = set()
    result_paths: set[str] = set()
    for case in manifest["cases"]:
        case_paths.append(case["path"])
        frozen_case = FROZEN_CASE_BINDINGS[case["id"]]
        actual_frozen_fields = {
            "source_id": case["source_id"],
            "capsule_id": case["intent_capsule"]["capsule_id"],
            "properties": tuple(case["property_ids"]),
            "witness_ids": tuple(
                witness["witness_id"] for witness in case["expected_witnesses"]
            ),
            "oracle": case["oracle_outcome"],
            "intended": case["intended_evidence_class"],
            "current": case["current_evidence_class"],
            "support": case["support"],
            "result_path": case["result"]["path"],
            "result_id": case["result"]["result_id"],
        }
        if actual_frozen_fields != frozen_case:
            errors.append(f"case {case['id']}: differs from the frozen v2 oracle")
        try:
            case_source = resolve_portable_path(case["path"])
            calculated_source_id = source_id(case_source)
        except (OSError, ValueError) as error:
            errors.append(f"case {case['id']}: cannot read source: {error}")
            continue
        if case["source_id"] != calculated_source_id:
            errors.append(f"case {case['id']}: stale source identity; expected {calculated_source_id}")
        if delimiter_depth(case_source.read_text(encoding="utf-8")) != 0:
            errors.append(f"case {case['id']}: source delimiters are not balanced")

        result_ref = case["result"]
        result_paths.add(result_ref["path"])
        try:
            result = load_json(resolve_portable_path(result_ref["path"]))
        except (OSError, UnicodeError, ValueError, json.JSONDecodeError) as error:
            errors.append(f"case {case['id']}: cannot load persisted result: {error}")
            continue
        errors.extend(
            validate_instance(
                result,
                result_schema,
                result_schema,
                f"$.result[{case['id']!r}]",
            )
        )
        report = result.get("report", {})
        errors.extend(
            validate_instance(
                report,
                model_report_schema,
                model_report_schema,
                f"$.result[{case['id']!r}].report",
            )
        )
        calculated_result_id = benchmark_result_id(result)
        if result.get("result_id") != calculated_result_id:
            errors.append(
                f"case {case['id']}: stale result identity; expected {calculated_result_id}"
            )
        if result_ref["result_id"] != result.get("result_id"):
            errors.append(f"case {case['id']}: manifest result reference mismatch")
        if result.get("case_id") != case["id"]:
            errors.append(f"case {case['id']}: persisted result case binding mismatch")
        if result.get("source") != {
            "path": case["path"],
            "source_id": case["source_id"],
        }:
            errors.append(f"case {case['id']}: persisted result source binding mismatch")
        if report.get("result") != case["current_evidence_class"]:
            errors.append(f"case {case['id']}: persisted/current result-class mismatch")
        if result.get("configuration", {}).get("max_states") != report.get(
            "bounds", {}
        ).get("max_states") or result.get("configuration", {}).get(
            "max_depth"
        ) != report.get(
            "bounds", {}
        ).get(
            "max_depth"
        ):
            errors.append(f"case {case['id']}: persisted configuration/bounds mismatch")

        report_properties = report.get("properties", [])
        expected_property_names = {
            properties[property_id]["handle"].split(".")[-1]
            for property_id in case["property_ids"]
            if property_id in properties
        }
        observed_property_names = {
            item.get("property") for item in report_properties if isinstance(item, dict)
        }
        if observed_property_names != expected_property_names:
            errors.append(f"case {case['id']}: persisted report property set mismatch")
        if case["current_evidence_class"] == "model_checked":
            if report.get("complete") is not True:
                errors.append(
                    f"case {case['id']}: model_checked requires an exhausted bounded frontier"
                )
            if any(
                item.get("result") != "model_checked" or item.get("witness") is not None
                for item in report_properties
                if isinstance(item, dict)
            ):
                errors.append(
                    f"case {case['id']}: model_checked properties must have no witnesses"
                )
        elif case["current_evidence_class"] == "refuted":
            refutations = [
                item
                for item in report_properties
                if isinstance(item, dict) and item.get("result") == "refuted"
            ]
            if not refutations or any(not item.get("witness") for item in refutations):
                errors.append(f"case {case['id']}: refuted result requires a witness")
        else:
            errors.append(
                f"case {case['id']}: unsupported current evidence class "
                f"{case['current_evidence_class']!r}"
            )

        intent_ref = case["intent_capsule"]
        intent_paths.add(intent_ref["path"])
        try:
            intent = load_json(resolve_portable_path(intent_ref["path"]))
        except (OSError, UnicodeError, ValueError, json.JSONDecodeError) as error:
            errors.append(f"case {case['id']}: cannot load intent capsule: {error}")
            continue
        errors.extend(validate_instance(intent, schemas["intent"], schemas["intent"], f"$.intent[{case['id']!r}]"))
        calculated_capsule_id = artifact_id(intent, "capsule_id")
        if intent.get("capsule_id") != calculated_capsule_id:
            errors.append(f"case {case['id']}: stale capsule identity; expected {calculated_capsule_id}")
        if intent_ref["capsule_id"] != intent.get("capsule_id"):
            errors.append(f"case {case['id']}: manifest capsule reference mismatch")
        if intent.get("case_id") != case["id"] or intent.get("case_class") != case["class"]:
            errors.append(f"case {case['id']}: intent case binding mismatch")
        if intent.get("source") != {"path": case["path"], "source_id": case["source_id"]}:
            errors.append(f"case {case['id']}: intent source binding mismatch")
        if intent.get("provenance_id") != provenance.get("provenance_id"):
            errors.append(f"case {case['id']}: intent provenance binding mismatch")
        if intent.get("property_ids") != case["property_ids"]:
            errors.append(f"case {case['id']}: intent property binding mismatch")

        expected = intent.get("expected", {})
        witness_ids = [item["witness_id"] for item in case["expected_witnesses"]]
        if expected.get("witness_ids") != witness_ids:
            errors.append(f"case {case['id']}: intent witness binding mismatch")
        for field in ("current_evidence_class", "intended_evidence_class", "oracle_outcome", "support"):
            if expected.get(field) != case[field]:
                errors.append(f"case {case['id']}: intent/manifest {field} mismatch")
        if expected.get("result") != case["result"]:
            errors.append(f"case {case['id']}: intent/manifest result binding mismatch")

        normalized = normalized_source(case_source)
        for property_id in case["property_ids"]:
            contract = properties.get(property_id)
            if contract is None:
                errors.append(f"case {case['id']}: references unknown property {property_id}")
            elif contract["nmlt_formula"] not in normalized:
                errors.append(f"case {case['id']}: frozen property formula is absent from source")

        if case["class"] == "reference":
            if (
                case["intended_evidence_class"] != "model_checked"
                or case["current_evidence_class"] != "model_checked"
                or case["expected_witnesses"]
            ):
                errors.append("reference: oracle must expect model_checked with no counterexample")
        else:
            if (
                case["intended_evidence_class"] != "refuted"
                or case["current_evidence_class"] != "refuted"
                or len(case["expected_witnesses"]) != 1
            ):
                errors.append(f"case {case['id']}: mutant must expect one refutation witness")

        for witness_ref in case["expected_witnesses"]:
            witness_paths.add(witness_ref["path"])
            try:
                witness = load_json(resolve_portable_path(witness_ref["path"]))
            except (OSError, UnicodeError, ValueError, json.JSONDecodeError) as error:
                errors.append(f"case {case['id']}: cannot load expected witness: {error}")
                continue
            errors.extend(validate_instance(witness, schemas["counterexample"], schemas["counterexample"], f"$.witness[{case['id']!r}]"))
            calculated_witness_id = artifact_id(witness, "witness_id")
            if witness.get("witness_id") != calculated_witness_id:
                errors.append(f"case {case['id']}: stale witness identity; expected {calculated_witness_id}")
            if witness_ref["witness_id"] != witness.get("witness_id"):
                errors.append(f"case {case['id']}: manifest witness reference mismatch")
            if witness.get("case_id") != case["id"]:
                errors.append(f"case {case['id']}: expected witness case binding mismatch")
            if witness.get("property_id") not in case["property_ids"]:
                errors.append(f"case {case['id']}: expected witness property binding mismatch")
            if witness.get("minimal_transition_count") != len(witness.get("steps", [])):
                errors.append(f"case {case['id']}: witness transition count is inconsistent")
            if witness.get("violation", {}).get("state_index", 0) > len(witness.get("steps", [])):
                errors.append(f"case {case['id']}: witness violation index exceeds trace")
            if case["id"] == "blind-replay" and {
                "minimal_transition_count": witness.get("minimal_transition_count"),
                "initial_observation": witness.get("initial_observation"),
                "steps": witness.get("steps"),
                "state_index": witness.get("violation", {}).get("state_index"),
                "predicate": witness.get("violation", {}).get("predicate"),
            } != {
                "minimal_transition_count": 0,
                "initial_observation": {
                    "phase": "indeterminate",
                    "dispatch_count": 1,
                    "dispatch_enabled": True,
                },
                "steps": [],
                "state_index": 0,
                "predicate": "indeterminate implies not enabled(dispatch)",
            }:
                errors.append(
                    "blind-replay: v2 oracle must expose the zero-transition "
                    "current-state enabledness violation"
                )

            contract = properties.get(witness.get("property_id"), {})
            property_name = contract.get("handle", ".").split(".")[-1]
            matching_reports = [
                item
                for item in report_properties
                if isinstance(item, dict) and item.get("property") == property_name
            ]
            if len(matching_reports) != 1:
                errors.append(
                    f"case {case['id']}: no unique persisted report for expected witness"
                )
                continue
            actual_witness = matching_reports[0].get("witness")
            if not isinstance(actual_witness, dict):
                errors.append(
                    f"case {case['id']}: persisted refutation omitted its witness"
                )
                continue
            actual_steps = actual_witness.get("steps", [])
            expected_steps = witness.get("steps", [])
            if len(actual_steps) != len(expected_steps) + 1:
                errors.append(
                    f"case {case['id']}: persisted and expected witness lengths differ"
                )
                continue
            if actual_steps[0].get("action") is not None:
                errors.append(
                    f"case {case['id']}: persisted witness must begin with the initial state"
                )

            def state_is_expected(
                actual_state: dict[str, Any], expected_observation: dict[str, Any]
            ) -> bool:
                return all(
                    key.endswith("_enabled") or actual_state.get(key) == value
                    for key, value in expected_observation.items()
                )

            if not state_is_expected(
                actual_steps[0].get("state", {}), witness.get("initial_observation", {})
            ):
                errors.append(
                    f"case {case['id']}: persisted initial state contradicts expected witness"
                )
            for index, expected_step in enumerate(expected_steps, start=1):
                actual_step = actual_steps[index]
                if actual_step.get("index") != index:
                    errors.append(
                        f"case {case['id']}: persisted witness indices are not canonical"
                    )
                if actual_step.get("action") != expected_step.get("action"):
                    errors.append(
                        f"case {case['id']}: persisted witness action contradicts oracle"
                    )
                if not state_is_expected(
                    actual_step.get("state", {}),
                    expected_step.get("resulting_observation", {}),
                ):
                    errors.append(
                        f"case {case['id']}: persisted witness state contradicts oracle"
                    )

    reference_text = normalized_source(resolve_portable_path(manifest["cases"][0]["path"]))
    for required_fragment in (
        "action dispatch",
        "set dispatched = true",
        "action lose_response",
        "action evaluate_pass",
        "action evaluate_fail",
    ):
        if required_fragment not in reference_text:
            errors.append(f"reference: missing anti-vacuity fragment {required_fragment!r}")

    control_paths: list[str] = []
    controls = {control["class"]: control for control in manifest["controls"]}
    for control in manifest["controls"]:
        control_paths.append(control["path"])
        frozen_control = FROZEN_CONTROL_BINDINGS[control["id"]]
        if {
            "source_id": control["source_id"],
            "class": control["class"],
            "outcome": control["expected_control_outcome"],
            "current": control["current_evidence_class"],
            "support": control["support"],
            "property_id": control.get("property_id"),
        } != frozen_control:
            errors.append(f"control {control['id']}: differs from the frozen v2 oracle")
        try:
            path = resolve_portable_path(control["path"])
            calculated_source_id = source_id(path)
        except (OSError, ValueError) as error:
            errors.append(f"control {control['id']}: cannot read source: {error}")
            continue
        if control["source_id"] != calculated_source_id:
            errors.append(f"control {control['id']}: stale source identity; expected {calculated_source_id}")

    malformed_path = resolve_portable_path(controls["malformed"]["path"])
    if delimiter_depth(malformed_path.read_text(encoding="utf-8")) == 0:
        errors.append("malformed control: missing-delimiter defect is no longer observable")
    vacuous_text = normalized_source(resolve_portable_path(controls["vacuous_property"]["path"]))
    dispatch_formula = properties[manifest["property_contracts"][0]["property_id"]]["nmlt_formula"]
    dispatch_property_id = FROZEN_PROPERTY_IDS["ProviderAttempt.DispatchRequiresArm"]
    for control_class in ("vacuous_property", "weakened_invariant"):
        if controls[control_class].get("property_id") != dispatch_property_id:
            errors.append(f"{control_class} control: canonical property binding changed")
    if "property_id" in controls["malformed"]:
        errors.append("malformed control: must not claim a semantic property binding")
    if dispatch_formula not in vacuous_text or "set dispatched = true" in vacuous_text:
        errors.append("vacuous control: property is absent or dispatch became reachable")
    weakened_text = normalized_source(resolve_portable_path(controls["weakened_invariant"]["path"]))
    if dispatch_formula in weakened_text or "armed or not armed" not in weakened_text:
        errors.append("weakened control: tautological mutation is no longer distinct")
    for control_class in ("vacuous_property", "weakened_invariant"):
        path = resolve_portable_path(controls[control_class]["path"])
        if delimiter_depth(path.read_text(encoding="utf-8")) != 0:
            errors.append(f"{control_class} control: source delimiters are not balanced")

    semantic_control = controls["semantic_regression"]
    semantic_text = normalized_source(
        resolve_portable_path(semantic_control["path"])
    )
    semantic_fragments = (
        "state phase: Phase = indeterminate",
        "action dispatch { require phase == indeterminate set phase = reconciled }",
        f"temporal HistoricalNextNoBlindReplay = {HISTORICAL_NO_BLIND_REPLAY}",
        f"temporal CorrectedNoBlindReplay = {CORRECTED_NO_BLIND_REPLAY}",
    )
    if any(fragment not in semantic_text for fragment in semantic_fragments):
        errors.append(
            "one-shot replay control: exact old-versus-corrected behavior is absent"
        )
    if semantic_control.get("property_id") != FROZEN_PROPERTY_IDS[
        "ProviderAttempt.NoBlindReplay"
    ]:
        errors.append("one-shot replay control: corrected property binding changed")
    if delimiter_depth(
        resolve_portable_path(semantic_control["path"]).read_text(encoding="utf-8")
    ) != 0:
        errors.append("one-shot replay control: source delimiters are not balanced")

    tla_text = normalized_source(
        ROOT / "comparisons" / "provider-attempt" / "tla" / "ProviderAttempt.tla"
    )
    if 'NoBlindReplay == phase = "indeterminate" => ~ENABLED Dispatch' not in tla_text:
        errors.append(
            "TLA+ comparison: NoBlindReplay is not aligned with current-state enabledness"
        )

    all_source_paths = case_paths + control_paths
    if manifest["source_set_id"] != source_set_id(all_source_paths):
        errors.append(f"manifest: stale source_set_id; expected {source_set_id(all_source_paths)}")

    expected_case_files = {
        path.relative_to(ROOT).as_posix()
        for path in (ROOT / "benchmarks" / "seeded-defects" / "provider-attempt").glob("*.nmlt")
    }
    expected_control_files = {
        path.relative_to(ROOT).as_posix()
        for path in (ROOT / "benchmarks" / "controls" / "provider-attempt").glob("*.nmlt")
    }
    if set(case_paths) != expected_case_files:
        errors.append("manifest: seeded provider sources and registered cases differ")
    if set(control_paths) != expected_control_files:
        errors.append("manifest: provider control sources and registered controls differ")

    actual_property_paths = {
        path.relative_to(ROOT).as_posix()
        for path in (ROOT / "benchmarks" / "provider-attempt" / "properties").glob("*.json")
    }
    actual_intent_paths = {
        path.relative_to(ROOT).as_posix()
        for path in (ROOT / "benchmarks" / "provider-attempt" / "intent").glob("*.json")
    }
    actual_witness_paths = {
        path.relative_to(ROOT).as_posix()
        for path in (ROOT / "benchmarks" / "provider-attempt" / "expected-counterexamples").glob("*.json")
    }
    actual_result_paths = {
        path.relative_to(ROOT).as_posix()
        for path in (ROOT / "benchmarks" / "results" / "provider-attempt").glob(
            "*.json"
        )
    }
    if property_paths != actual_property_paths:
        errors.append("manifest: property artifacts and registered property contracts differ")
    if intent_paths != actual_intent_paths:
        errors.append("manifest: intent artifacts and registered cases differ")
    if witness_paths != actual_witness_paths:
        errors.append("manifest: expected witnesses and registered witness references differ")
    if result_paths != actual_result_paths:
        errors.append("manifest: persisted results and registered result references differ")
    return errors


def verify_upstream(path: Path) -> list[str]:
    errors: list[str] = []
    provenance = load_json(ROOT / "benchmarks" / "provider-attempt" / "provenance.json")
    revision = provenance["source_repository"]["revision"]
    records = provenance["source_artifacts"] + [provenance["license"]]
    for record in records:
        command = ["git", "-C", str(path), "show", f"{revision}:{record['path']}"]
        result = subprocess.run(command, check=False, capture_output=True)
        if result.returncode != 0:
            errors.append(f"upstream: cannot read {revision}:{record['path']}")
            continue
        digest = hashlib.sha256(result.stdout).hexdigest()
        if digest != record["sha256"]:
            errors.append(
                f"upstream: digest mismatch for {record['path']}; expected {record['sha256']}, got {digest}"
            )
    return errors


def run_self_test() -> list[str]:
    manifest = load_json(MANIFEST_PATH)
    failures: list[str] = []

    promoted = copy.deepcopy(manifest)
    promoted["cases"][0]["current_evidence_class"] = "proved"
    if not validate_repository(promoted):
        failures.append("self-test: model_checked-to-proved promotion was not rejected")

    stale = copy.deepcopy(manifest)
    stale["cases"][0]["source_id"] = SOURCE_PREFIX + "0" * 64
    if not validate_repository(stale):
        failures.append("self-test: stale source identity was not rejected")

    missing_control = copy.deepcopy(manifest)
    missing_control["controls"].pop()
    if not validate_repository(missing_control):
        failures.append("self-test: missing negative control was not rejected")

    stale_result = copy.deepcopy(manifest)
    stale_result["cases"][0]["result"]["result_id"] = RESULT_PREFIX + "0" * 64
    if not validate_repository(stale_result):
        failures.append("self-test: stale persisted-result binding was not rejected")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--self-test",
        action="store_true",
        help=(
            "also prove that proof promotion, stale source/result bindings, "
            "and missing controls fail"
        ),
    )
    parser.add_argument(
        "--upstream",
        type=Path,
        help="optionally verify frozen blobs against a local Technicusverus Git repository",
    )
    args = parser.parse_args()

    errors = validate_repository()
    if args.self_test:
        errors.extend(run_self_test())
    if args.upstream is not None:
        errors.extend(verify_upstream(args.upstream.resolve()))

    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    suffix = "; negative self-tests passed" if args.self_test else ""
    if args.upstream is not None:
        suffix += "; upstream blobs matched"
    print(
        "ok: provider-attempt-seeded-defects-v2 "
        "(1 bounded model_checked result, 4 refutations matched to witness oracles, "
        "3 integrity controls, 1 executable semantic-regression control"
        f"{suffix})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
