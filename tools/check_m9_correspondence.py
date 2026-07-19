#!/usr/bin/env python3
"""Fail closed when the shared M9 Rust/Lean correspondence vectors drift."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "mechanization/vectors/m9-kernel-v1.json"
LEAN = ROOT / "mechanization/lean/NMLT/Correspondence/M9Kernel.lean"
RUST = ROOT / "crates/nmlt-kernel/tests/kernel.rs"


def main() -> int:
    value = json.loads(MANIFEST.read_text(encoding="utf-8"))
    assert value["schema_version"] == "nmlt-m9-correspondence-v1"
    assert value["rule"] == {"action_use_tag": 52, "judgment_tag": 8}
    vectors = {item["name"]: item for item in value["vectors"]}
    assert set(vectors) == {
        "accepted_action",
        "missing_frame",
        "bad_action_rule_tag",
    }
    assert vectors["accepted_action"]["expected"] == "accept"
    assert vectors["accepted_action"]["frames"] == [0]
    assert vectors["missing_frame"]["expected"] == "reject"
    assert vectors["missing_frame"]["frames"] == []
    assert vectors["bad_action_rule_tag"]["expected"] == "reject"
    assert vectors["bad_action_rule_tag"]["certificate_rule_tag"] == 51

    lean = LEAN.read_text(encoding="utf-8")
    rust = RUST.read_text(encoding="utf-8")
    for required in (
        "ruleTag := 52",
        "def acceptedAction",
        "def missingFrameAction",
        "def badRuleCertificate",
        "check acceptedCore acceptedCertificate",
        "check missingFrameCore acceptedCertificate",
        "check acceptedCore badRuleCertificate",
    ):
        assert required in lean, f"Lean correspondence binding missing: {required}"
    for required in (
        "shared_m9_correspondence_vectors_match_the_rust_kernel_boundary",
        "node.rule_tag == 52 && node.obligation.judgment_tag == 8",
        ".rule_tag = 51",
    ):
        assert required in rust, f"Rust correspondence binding missing: {required}"

    print("M9 Rust/Lean correspondence vectors passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
