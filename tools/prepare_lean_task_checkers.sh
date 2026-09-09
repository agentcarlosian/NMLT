#!/usr/bin/env bash
set -euo pipefail

# The same immutable exporter/checker pair as the repository metatheory gate.
readonly EXPORTER_COMMIT=411dce7db58a3afc60ecab2d211acd1042b593dc
readonly NANODA_COMMIT=05055695879dfebb6628a67da88ceca6cd6b0421
readonly OUTPUT="${1:?usage: prepare_lean_task_checkers.sh <new-directory>}"
if [[ -e "$OUTPUT" || -L "$OUTPUT" ]]; then
  echo 'error: checker output must be a new directory' >&2
  exit 2
fi
mkdir -- "$OUTPUT"
readonly ROOT="$(cd "$OUTPUT" && pwd -P)"
fetch() {
  local url="$1" revision="$2" destination="$3"
  git init --quiet "$destination"
  git -C "$destination" remote add origin "$url"
  git -C "$destination" fetch --quiet --depth 1 origin "$revision"
  git -C "$destination" checkout --quiet --detach FETCH_HEAD
  [[ "$(git -C "$destination" rev-parse HEAD)" == "$revision" ]]
}
fetch https://github.com/leanprover/lean4export.git "$EXPORTER_COMMIT" "$ROOT/lean4export"
printf 'leanprover/lean4:v4.33.1\n' > "$ROOT/lean4export/lean-toolchain"
(cd "$ROOT/lean4export" && lake build)
fetch https://github.com/ammkrn/nanoda_lib.git "$NANODA_COMMIT" "$ROOT/nanoda"
cargo +1.94.0 build --locked --release --manifest-path "$ROOT/nanoda/Cargo.toml"
printf 'lean=v4.33.1\nlean4export=%s\nnanoda=%s\nrust=1.94.0\n' "$EXPORTER_COMMIT" "$NANODA_COMMIT" > "$ROOT/provenance.txt"
echo "Checker installations: $ROOT"
