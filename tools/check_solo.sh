#!/usr/bin/env bash
set -euo pipefail
readonly LEAN_BIN="${1:?usage: check_solo.sh <direct-lean-executable>}"
cargo build --quiet -p nmlt-cli
mkdir -p target/solo-checkers
readonly RUN="$(mktemp -d target/solo-checkers/run.XXXXXX)"
bash tools/prepare_lean_task_checkers.sh "$RUN/tools"
exporter="$RUN/tools/lean4export/.lake/build/bin/lean4export"
nanoda="$RUN/tools/nanoda/target/release/nanoda_bin"
if [[ -f "$exporter.exe" ]]; then exporter+=".exe"; fi
if [[ -f "$nanoda.exe" ]]; then nanoda+=".exe"; fi
"${PYTHON:-python3}" tools/experiments/run_solo.py \
  --lean-bin "$LEAN_BIN" --exporter "$exporter" --nanoda "$nanoda"
