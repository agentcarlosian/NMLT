#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lean_root="${1:-$repo_root/mechanization/lean}"
source_root="$repo_root/mechanization/lean"

if ! command -v lake >/dev/null 2>&1; then
  echo "error: lake is required; install Elan and the pinned lean-toolchain" >&2
  exit 1
fi

if grep -rEn '(^|[^[:alnum:]_])(sorry|sorryAx|admit|native_decide)([^[:alnum:]_]|$)|^[[:space:]]*axiom[[:space:]]' \
  "$source_root/NMLT" "$source_root/NMLT.lean"; then
  echo "error: forbidden unchecked Lean construct found" >&2
  exit 1
fi

artifact="$repo_root/examples/pivot/visible_resource_sync.behavior-core-v1.json"
stale_artifact="$(mktemp)"
malformed_artifact="$(mktemp)"
axiom_probe="$(mktemp --suffix=.lean)"
axiom_log="$(mktemp)"
cleanup() {
  rm -f "$stale_artifact" "$malformed_artifact" "$axiom_probe" "$axiom_log"
}
trap cleanup EXIT

(
  cd "$lean_root"
  lake build
)

(
  cd "$lean_root"
  lake exe nmlt-artifact-check "$artifact" \
    "$repo_root/examples/pivot/visible_resource_sync.nmlt"
)

python3 - "$artifact" "$stale_artifact" "$malformed_artifact" <<'PY'
import json
import sys

source, stale_path, malformed_path = sys.argv[1:]
with open(source, encoding="utf-8") as handle:
    value = json.load(handle)

stale = dict(value)
stale["source_sha256"] = "0" * 64
with open(stale_path, "w", encoding="utf-8") as handle:
    json.dump(stale, handle)

malformed = json.loads(json.dumps(value))
malformed["systems"]["Receiver"]["actions"]["receive"]["resources"]["receives"] = []
with open(malformed_path, "w", encoding="utf-8") as handle:
    json.dump(malformed, handle)
PY

if (cd "$lean_root" && lake exe nmlt-artifact-check "$stale_artifact" \
    "$repo_root/examples/pivot/visible_resource_sync.nmlt"); then
  echo "error: Lean accepted a stale source digest" >&2
  exit 1
fi

if (cd "$lean_root" && lake exe nmlt-artifact-check "$malformed_artifact" \
    "$repo_root/examples/pivot/visible_resource_sync.nmlt"); then
  echo "error: Lean accepted a malformed transfer profile" >&2
  exit 1
fi

cat > "$axiom_probe" <<'EOF'
import NMLT
#print axioms NMLT.Behavior.ResourceBehavior.liftParallel
#print axioms NMLT.Examples.VisibleResourceSync.visibleResourceSync_lifts
EOF
(
  cd "$lean_root"
  lake env lean "$axiom_probe"
) | tee "$axiom_log"

if grep -Eq 'sorryAx|Classical.choice|Lean.trustCompiler' "$axiom_log"; then
  echo "error: behavior theorem uses an unapproved axiom" >&2
  exit 1
fi

if grep 'depends on axioms:' "$axiom_log" |
    grep -Ev 'axioms: \[(propext|Quot.sound|propext, Quot.sound)\]$'; then
  echo "error: focused behavior theorem exceeds the approved axiom allowlist" >&2
  exit 1
fi

echo "ok: Lean behavior core, decoder controls, no-sorry policy, and axiom audit"
