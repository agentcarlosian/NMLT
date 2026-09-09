#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lean_root="${1:-$repo_root/mechanization/lean}"
artifact="$(mktemp)"
rust_snapshot="$(mktemp)"
lean_snapshot="$(mktemp)"
trap 'rm -f "$artifact" "$rust_snapshot" "$lean_snapshot"' EXIT

cd "$repo_root"
cargo run --quiet -p nmlt-cli -- elaborate examples/pivot/finite_value_cycle.nmlt \
  --emit-core "$artifact"
cmp examples/pivot/finite_value_cycle.behavior-core-v1.json "$artifact"
cargo run --quiet -p nmlt-eval --example finite_parity -- "$artifact" ValueCycle \
  > "$rust_snapshot"
(
  cd "$lean_root"
  lake build NMLT
)
if ! (
  cd "$lean_root"
  lake env lean --run "$repo_root/mechanization/lean/tests/FiniteParity.lean" \
    "$artifact" ValueCycle
) > "$lean_snapshot"; then
  cat "$lean_snapshot" >&2
  exit 1
fi

"${PYTHON:-python3}" - "$rust_snapshot" "$lean_snapshot" <<'PY'
from pathlib import Path
import hashlib
import sys

snapshots = []
for name in sys.argv[1:]:
    lines = Path(name).read_text(encoding="utf-8").splitlines()
    if any(not line.startswith(("I|", "S|", "T|")) for line in lines):
        raise SystemExit(f"unexpected output in {name}: {lines}")
    if len(set(lines)) != len(lines):
        raise SystemExit(f"duplicate rows in {name}")
    counts = tuple(sum(line.startswith(kind + "|") for line in lines) for kind in "IST")
    if counts != (1, 4, 10):
        raise SystemExit(f"frozen corpus coverage changed in {name}: {counts}, expected (1, 4, 10)")
    snapshots.append(set(lines))
if snapshots[0] != snapshots[1]:
    raise SystemExit(f"Rust-only: {sorted(snapshots[0] - snapshots[1])}\n"
                     f"Lean-only: {sorted(snapshots[1] - snapshots[0])}")
identity = hashlib.sha256(("\n".join(sorted(snapshots[0])) + "\n").encode()).hexdigest()
print(f"ok: finite value parity; 1 initial, 4 reachable states, 10 transitions; sha256={identity}")
print("scope: closed resource-free Bool/Unit/enum fixture; finite comparison, not compiler correctness")
PY
