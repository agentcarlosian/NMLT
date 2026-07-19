#!/usr/bin/env python3
"""Verify the frozen v1 canonical corpus and its domain-separated identities."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "examples" / "canonical-v1.json"
EXPECTED = (
    ("C01", "examples/basics/boolean_toggle.nmlt"),
    ("C02", "examples/hyperbook/one_bit_clock.nmlt"),
    ("C03", "examples/math/euclid.nmlt"),
    ("C04", "examples/technicus/provider_attempt.nmlt"),
    ("C05", "examples/concurrency/two_process_mutex.nmlt"),
    ("C06", "examples/refinement/bounded_channel.nmlt"),
    ("C07", "examples/agents/trust_chain.nmlt"),
    ("C08", "examples/runtime/durable_controller.nmlt"),
    ("C09", "examples/distributed/two_phase_commit.nmlt"),
    ("C10", "examples/resources/token_bucket.nmlt"),
)
SOURCE_PREFIX = "nmlt-source-v1:sha256:"
SOURCE_SET_PREFIX = "nmlt-source-set-v1:sha256:"
EXAMPLE_PREFIX = "nmlt-canonical-example-v1:sha256:"
CORPUS_PREFIX = "nmlt-canonical-corpus-v1:sha256:"


class DuplicateKey(ValueError):
    pass


def reject_duplicate_pairs(pairs: list[tuple[str, object]]) -> dict[str, object]:
    value: dict[str, object] = {}
    for key, item in pairs:
        if key in value:
            raise DuplicateKey(f"duplicate JSON member {key!r}")
        value[key] = item
    return value


def reject_number(raw: str) -> object:
    raise ValueError(f"canonical registry subset does not permit number {raw!r}")


def parse_registry(text: str) -> dict[str, object]:
    value = json.loads(
        text,
        object_pairs_hook=reject_duplicate_pairs,
        parse_float=reject_number,
        parse_int=reject_number,
        parse_constant=reject_number,
    )
    if not isinstance(value, dict):
        raise ValueError("canonical registry must be a JSON object")
    return value


def load_registry() -> dict[str, object]:
    return parse_registry(MANIFEST.read_text(encoding="utf-8"))


def canonical_json(value: object) -> bytes:
    """Encode the manifest's string/array/object subset deterministically."""
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def content_id(prefix: str, domain: bytes, value: object) -> str:
    encoded = canonical_json(value)
    return prefix + hashlib.sha256(
        domain + len(encoded).to_bytes(8, "big") + encoded
    ).hexdigest()


def example_id(value: dict[str, object]) -> str:
    payload = dict(value)
    payload.pop("entry_id", None)
    return content_id(
        EXAMPLE_PREFIX, b"NMLT-CANONICAL-EXAMPLE\0v1\0", payload
    )


def corpus_id(value: dict[str, object]) -> str:
    payload = dict(value)
    payload.pop("corpus_id", None)
    return content_id(CORPUS_PREFIX, b"NMLT-CANONICAL-CORPUS\0v1\0", payload)


def source_digest(data: bytes) -> bytes:
    preimage = b"NMLT-SOURCE\0v1\0" + len(data).to_bytes(8, "big") + data
    return hashlib.sha256(preimage).digest()


def source_id(path: Path) -> str:
    return SOURCE_PREFIX + source_digest(path.read_bytes()).hex()


def source_set_id(entries: tuple[tuple[str, str], ...]) -> str:
    encoded = bytearray(b"NMLT-SOURCE-SET\0v1\0")
    encoded.extend(len(entries).to_bytes(8, "big"))
    for _, portable_path in sorted(entries, key=lambda item: item[1].encode("utf-8")):
        path_bytes = portable_path.encode("utf-8")
        encoded.extend(len(path_bytes).to_bytes(8, "big"))
        encoded.extend(path_bytes)
        encoded.extend(source_digest((ROOT / portable_path).read_bytes()))
    return SOURCE_SET_PREFIX + hashlib.sha256(encoded).hexdigest()


def verify() -> list[str]:
    errors: list[str] = []
    try:
        manifest = load_registry()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot read {MANIFEST.relative_to(ROOT)}: {error}"]

    try:
        parse_registry('{"id":"C01","id":"forged"}')
    except DuplicateKey:
        pass
    else:
        errors.append("duplicate-member parser self-test did not reject ambiguity")

    if manifest.get("corpus_version") != "nmlt-canonical-v1":
        errors.append("corpus_version must be nmlt-canonical-v1")
    actual_entries = manifest.get("examples")
    if not isinstance(actual_entries, list) or len(actual_entries) != len(EXPECTED):
        return errors + ["manifest must contain exactly ten examples"]

    actual_pairs = tuple((item.get("id"), item.get("path")) for item in actual_entries)
    if actual_pairs != EXPECTED:
        errors.append("example IDs, paths, or order differ from the v1 freeze")

    for item in actual_entries:
        path_text = item.get("path", "")
        path = ROOT / path_text
        if not path.is_file():
            errors.append(f"missing canonical source: {path_text}")
            continue
        expected_id = source_id(path)
        if item.get("source_id") != expected_id:
            errors.append(f"stale source_id for {path_text}: expected {expected_id}")
        expected_entry_id = example_id(item)
        if item.get("entry_id") != expected_entry_id:
            errors.append(
                f"stale entry_id for {path_text}: expected {expected_entry_id}"
            )
        if not item.get("intent"):
            errors.append(f"missing intent for {path_text}")
        if not item.get("claims"):
            errors.append(f"missing claims for {path_text}")
        control = item.get("negative_control")
        if not isinstance(control, dict) or not control.get("mutation"):
            errors.append(f"missing negative control for {path_text}")

    expected_set = source_set_id(EXPECTED)
    if manifest.get("source_set_id") != expected_set:
        errors.append(f"stale source_set_id: expected {expected_set}")
    expected_corpus_id = corpus_id(manifest)
    if manifest.get("corpus_id") != expected_corpus_id:
        errors.append(f"stale corpus_id: expected {expected_corpus_id}")

    claim_probe = json.loads(json.dumps(actual_entries[0]))
    claim_probe["claims"].append("forged-claim-with-unchanged-source")
    if example_id(claim_probe) == example_id(actual_entries[0]):
        errors.append("canonical entry identity does not bind intended claims")
    corpus_probe = json.loads(json.dumps(manifest))
    corpus_probe["examples"][0]["intent"] += " forged"
    if corpus_id(corpus_probe) == corpus_id(manifest):
        errors.append("canonical corpus identity does not bind example intent")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--print-ids", action="store_true", help="print calculated IDs without editing files"
    )
    args = parser.parse_args()
    if args.print_ids:
        manifest = load_registry()
        for example_handle, portable_path in EXPECTED:
            item = next(
                item for item in manifest["examples"] if item["id"] == example_handle
            )
            print(
                example_handle,
                portable_path,
                source_id(ROOT / portable_path),
                example_id(item),
            )
        print("SET", source_set_id(EXPECTED))
        print("CORPUS", corpus_id(manifest))
        return 0

    errors = verify()
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(
        "ok: nmlt-canonical-v1 "
        "(10 examples; source, intent, claim, control, and corpus identities current)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
