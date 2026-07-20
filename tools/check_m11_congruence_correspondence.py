#!/usr/bin/env python3
"""Check the bounded M11-001c Rust/Lean translation-validation bindings."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VECTORS = ROOT / "mechanization/vectors/m11-open-congruence-v1.json"
RUST = ROOT / "crates/nmlt-temporal/src/open_congruence.rs"
LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenComposition.lean"
MAPPED_LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenMappedCongruence.lean"
RESOURCE_LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenResourceCongruence.lean"
ENCODING_LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenEncodingCorrespondence.lean"
EXECUTION_LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenKernelExecution.lean"
READBACK_LEAN = ROOT / "mechanization/lean/NMLT/Behavior/OpenKernelReadback.lean"


def main() -> None:
    vectors = json.loads(VECTORS.read_text(encoding="utf-8"))
    assert vectors["schema_version"] == "nmlt-m11-open-congruence-v1"
    assert vectors["claim_class"] == "shared-vector-drift-control"
    assert len(vectors["cases"]) == 15
    rust = RUST.read_text(encoding="utf-8")
    lean = "".join(
        path.read_text(encoding="utf-8")
        for path in (
            LEAN,
            MAPPED_LEAN,
            RESOURCE_LEAN,
            ENCODING_LEAN,
            EXECUTION_LEAN,
            READBACK_LEAN,
        )
    )
    seen: set[str] = set()
    for case in vectors["cases"]:
        assert case["id"] not in seen, f"duplicate case id: {case['id']}"
        seen.add(case["id"])
        assert case["expected"] in {"accepted", "rejected"}
        assert case["rust_control"] in rust, case["rust_control"]
        handle = case["lean_handle"]
        assert handle in lean or handle.rsplit(".", 1)[-1] in lean, handle
    assert "verified extraction" in " ".join(vectors["limitations"])
    print("M11-001c Rust/Lean correspondence vectors passed")


if __name__ == "__main__":
    main()
