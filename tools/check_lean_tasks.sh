#!/usr/bin/env bash
set -euo pipefail
readonly LEAN_BIN="${1:?usage: check_lean_tasks.sh <direct-lean-executable>}"
mkdir -p target/r3-checkers
readonly RUN="$(mktemp -d target/r3-checkers/run.XXXXXX)"
bash tools/prepare_lean_task_checkers.sh "$RUN/tools"
exporter="$RUN/tools/lean4export/.lake/build/bin/lean4export"
nanoda="$RUN/tools/nanoda/target/release/nanoda_bin"
if [[ -f "$exporter.exe" ]]; then exporter+=".exe"; fi
if [[ -f "$nanoda.exe" ]]; then nanoda+=".exe"; fi
"${PYTHON:-python3}" tools/check_lean_tasks.py --lean-bin "$LEAN_BIN" --exporter "$exporter" --nanoda "$nanoda"
"${PYTHON:-python3}" tools/check_lean_imports.py --lean-bin "$LEAN_BIN" --exporter "$exporter" --nanoda "$nanoda"
"${PYTHON:-python3}" tools/check_lean_exports.py --lean-bin "$LEAN_BIN" --exporter "$exporter" --nanoda "$nanoda"
"${PYTHON:-python3}" tools/check_lean_dependencies.py --lean-bin "$LEAN_BIN" --exporter "$exporter" --nanoda "$nanoda"
