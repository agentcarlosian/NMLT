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
semantic_mismatch_artifact="$(mktemp)"
world_requirement_artifact="$(mktemp)"
axiom_probe="$(mktemp --suffix=.lean)"
axiom_log="$(mktemp)"
cleanup() {
  rm -f "$stale_artifact" "$malformed_artifact" "$semantic_mismatch_artifact" \
    "$world_requirement_artifact" "$axiom_probe" "$axiom_log"
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

python3 - "$artifact" "$stale_artifact" "$malformed_artifact" \
  "$semantic_mismatch_artifact" "$world_requirement_artifact" <<'PY'
import json
import sys

source, stale_path, malformed_path, semantic_mismatch_path, world_requirement_path = sys.argv[1:]
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

semantic_mismatch = json.loads(json.dumps(value))
abstract_send = semantic_mismatch["systems"]["AbstractSender"]["actions"]["send"]
abstract_send["guards"] = ["false"]
abstract_send["guard_ast"] = [{"kind": "bool", "type": "Bool", "value": False}]
with open(semantic_mismatch_path, "w", encoding="utf-8") as handle:
    json.dump(semantic_mismatch, handle)

world_requirement = json.loads(json.dumps(value))
world_requirement["systems"]["AbstractSender"]["actions"]["send"]["resources"]["requires"] = ["permit"]
with open(world_requirement_path, "w", encoding="utf-8") as handle:
    json.dump(world_requirement, handle)
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

if (cd "$lean_root" && lake exe nmlt-artifact-check "$semantic_mismatch_artifact" \
    "$repo_root/examples/pivot/visible_resource_sync.nmlt"); then
  echo "error: Lean accepted a failed visible-step simulation" >&2
  exit 1
fi

if (cd "$lean_root" && lake exe nmlt-artifact-check "$world_requirement_artifact" \
    "$repo_root/examples/pivot/visible_resource_sync.nmlt"); then
  echo "error: Lean accepted a refinement that does not preserve world-step enabledness" >&2
  exit 1
fi

cat > "$axiom_probe" <<'EOF'
import NMLT
#print axioms NMLT.Behavior.ResourceBehavior.liftParallel
#print axioms NMLT.Artifact.SemanticClosure.Certificate.lifted
#print axioms NMLT.Artifact.SemanticClosure.Certificate.liftedSynchronized
#print axioms NMLT.Artifact.SemanticClosure.Certificate.liftedStep
#print axioms NMLT.Behavior.ResourceWorld.ProductStep.synchronized_left_transfer_moves_once
#print axioms NMLT.Behavior.ResourceWorld.SyncStep.owner_after_is_explained
#print axioms NMLT.Behavior.ResourceWorld.LocalStep.world_preserved_of_refines_empty
#print axioms NMLT.Examples.ResourceWorldTransfer.permit_moves_exactly_once
#print axioms NMLT.Behavior.ResourceWorld.liftSynchronized
#print axioms NMLT.Behavior.ResourceWorld.liftProductSteps
#print axioms NMLT.Examples.ResourceWorldTransfer.dynamicallyMatchedProductTransfer
#print axioms NMLT.Examples.ResourceWorldTransfer.permit_transfer_lifts_dynamically
#print axioms NMLT.Counterexamples.ResourceWorldControls.hiddenConsumption_changesWorld
#print axioms NMLT.Counterexamples.ResourceWorldControls.hiddenConsumption_cannotRefineStutter
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

echo "ok: Lean behavior core, artifact-derived theorem closure, decoder controls, no-sorry policy, and axiom audit"
