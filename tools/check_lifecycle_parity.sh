#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lean_root="${1:-$repo_root/mechanization/lean}"
rust_trace="$(mktemp)"
lean_trace="$(mktemp)"
trap 'rm -f "$rust_trace" "$lean_trace"' EXIT

cd "$repo_root"
cargo run --quiet -p nmlt-runtime --example lifecycle_parity > "$rust_trace"
(
  cd "$lean_root"
  lake build NMLT
)
if ! (
  cd "$lean_root"
  lake env lean --run "$repo_root/mechanization/lean/tests/LifecycleParity.lean"
) > "$lean_trace"; then
  cat "$lean_trace" >&2
  exit 1
fi

"${PYTHON:-python3}" - "$rust_trace" "$lean_trace" <<'PY'
from pathlib import Path
import hashlib
import sys

rust_lines = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
lean_lines = Path(sys.argv[2]).read_text(encoding="utf-8").splitlines()
if len(rust_lines) != 19:
    raise SystemExit(f"expected 19 lifecycle parity steps from Rust, got {len(rust_lines)}")
if rust_lines != lean_lines:
    for idx, (r, l) in enumerate(zip(rust_lines, lean_lines), start=1):
        if r != l:
            raise SystemExit(f"mismatch at line {idx}:\n  Rust: {r}\n  Lean: {l}")
    raise SystemExit(f"length mismatch: Rust={len(rust_lines)}, Lean={len(lean_lines)}")
digest = hashlib.sha256(("\n".join(rust_lines) + "\n").encode()).hexdigest()
print(f"ok: multi-slot lifecycle parity; 19/19 steps matched Rust<->Lean; sha256={digest}")
print("scope: 19-command frozen multi-slot scenario (reserve/transfer/dispatch/stale-deliver/cancel/timeout/uncertain/reconcile/collect/recover/slot-reuse); differential test, not compiler verification")
PY
