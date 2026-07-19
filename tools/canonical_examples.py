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
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read {MANIFEST.relative_to(ROOT)}: {error}"]

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
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--print-ids", action="store_true", help="print calculated IDs without editing files"
    )
    args = parser.parse_args()
    if args.print_ids:
        for example_id, portable_path in EXPECTED:
            print(example_id, portable_path, source_id(ROOT / portable_path))
        print("SET", source_set_id(EXPECTED))
        return 0

    errors = verify()
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print("ok: nmlt-canonical-v1 (10 examples; source identities current)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
