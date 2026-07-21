#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
PYTHON_BIN=${PYTHON_BIN:-python3}

if ! command -v "$PYTHON_BIN" >/dev/null 2>&1; then
    printf 'judge-demo: Python 3.11+ is required; set PYTHON_BIN if it is not named python3.\n' >&2
    exit 2
fi

exec "$PYTHON_BIN" "$ROOT/demos/judge/judge_demo.py" "$@"
