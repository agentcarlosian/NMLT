#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
lean_root="$repo_root/mechanization/lean"

if ! command -v lake >/dev/null 2>&1; then
  echo "error: lake is required; install Elan and the pinned lean-toolchain" >&2
  exit 1
fi

if rg -n '(^|[^[:alnum:]_])(sorry|sorryAx|admit|native_decide)([^[:alnum:]_]|$)|^[[:space:]]*axiom[[:space:]]' \
  "$lean_root/NMLT" "$lean_root/NMLT.lean"; then
  echo "error: forbidden unchecked Lean construct found" >&2
  exit 1
fi

build_log="$(mktemp)"
trap 'rm -f "$build_log"' EXIT

(
  cd "$lean_root"
  lake build
) 2>&1 | tee "$build_log"

python3 "$repo_root/tools/check_open_composition_evidence.py" --lean-output "$build_log"
python3 "$repo_root/tools/check_open_refinement_evidence.py" --lean-output "$build_log"
python3 "$repo_root/tools/check_m11_congruence_correspondence.py"
python3 "$repo_root/tools/check_open_congruence_evidence.py" --lean-output "$build_log"
