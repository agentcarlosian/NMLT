#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lean_root="${1:-$repo_root/mechanization/lean}"
work="$(mktemp -d)"
trap 'rm -f "$work"/*.json "$work"/*.txt; rmdir "$work"' EXIT
cd "$repo_root"

for fixture in visible_resource_sync affine_continuation; do
  cargo run --quiet -p nmlt-cli -- elaborate "examples/pivot/$fixture.nmlt" \
    --core-version v2 --emit-core "$work/$fixture.json"
  cmp "examples/pivot/$fixture.behavior-core-v2.json" "$work/$fixture.json"
done

generate_path() {
  local fixture="$1" behavior="$2" name="$3" actions="$4"
  cargo run --quiet -p nmlt-cli -- trace --behavior "$behavior" --actions "$actions" \
    --emit-path "$work/$name.json" --max-states 32 "examples/pivot/$fixture.behavior-core-v2.json"
  cmp "examples/pivot/$name.behavior-execution-v1.json" "$work/$name.json"
}
generate_path visible_resource_sync ConcreteNetwork visible_resource_sync 'ConcreteSender.send|Receiver.receive'
generate_path affine_continuation Network receive_consume 'Receiver.receive|Sender.send,Receiver.monitor,Receiver.use'
generate_path affine_continuation Network receive_retransfer 'Receiver.receive|Sender.send,Receiver.giveback|Sender.returned,Sender.finish'

(cd "$lean_root" && lake build NMLT nmlt-artifact-check)
checker="$lean_root/.lake/build/bin/nmlt-artifact-check"
"$checker" "$repo_root/examples/pivot/visible_resource_sync.behavior-core-v2.json" \
  "$repo_root/examples/pivot/visible_resource_sync.nmlt" "$work/visible_resource_sync.json" > "$work/primary.txt"
cat "$work/primary.txt"
grep -F 'initial synchronized refinement applications: 1' "$work/primary.txt"
for path in receive_consume receive_retransfer; do
  "$checker" "$repo_root/examples/pivot/affine_continuation.behavior-core-v2.json" \
    "$repo_root/examples/pivot/affine_continuation.nmlt" "$work/$path.json"
done
"${PYTHON:-python3}" tests/execution/check_controls.py "$checker"
cargo run --quiet -p nmlt-eval --example execution_parity -- \
  examples/pivot/affine_continuation.behavior-core-v2.json Network > "$work/rust.txt"
if ! (cd "$lean_root" && lake env lean --run "$repo_root/mechanization/lean/tests/ExecutionParity.lean" \
    "$repo_root/examples/pivot/affine_continuation.behavior-core-v2.json" Network) > "$work/lean.txt"; then
  cat "$work/lean.txt" >&2
  exit 1
fi
"${PYTHON:-python3}" - "$work/rust.txt" "$work/lean.txt" <<'PY'
import hashlib
from pathlib import Path
import sys
snapshots = []
for path in sys.argv[1:]:
    lines = Path(path).read_text().splitlines()
    counts = tuple(sum(line.startswith(k + "|") for line in lines) for k in "IST")
    if counts != (1, 8, 12) or len(lines) != 21 or len(set(lines)) != 21:
        raise SystemExit(f"frozen execution coverage changed: {counts}; {path}")
    snapshots.append(set(lines))
if snapshots[0] != snapshots[1]:
    raise SystemExit(f"Rust-only: {snapshots[0] - snapshots[1]}\nLean-only: {snapshots[1] - snapshots[0]}")
digest = hashlib.sha256(("\n".join(sorted(snapshots[0])) + "\n").encode()).hexdigest()
print(f"ok: execution parity; 1 initial, 8 reachable states, 12 transitions; sha256={digest}")
PY
