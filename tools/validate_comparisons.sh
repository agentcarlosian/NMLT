#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

comparison_tmp="$(mktemp -d)"
trap 'rm -rf "$comparison_tmp"' EXIT

python3 - <<'PY'
import hashlib
import json
import re
from pathlib import Path

validation_path = Path("comparisons/provider-attempt/validation.json")
validation = json.loads(validation_path.read_text(encoding="utf-8"))
expected_paths = {
    "tla_plus": {
        "comparisons/provider-attempt/tla/ProviderAttempt.tla",
        "comparisons/provider-attempt/tla/ProviderAttempt.cfg",
    },
    "quint": {
        "comparisons/provider-attempt/quint/provider_attempt.qnt",
    },
    "p": {
        "comparisons/provider-attempt/p/PSrc/ProviderAttempt.p",
        "comparisons/provider-attempt/p/PSpec/ProviderAttemptSafety.p",
        "comparisons/provider-attempt/p/PTst/ProviderAttemptTest.p",
        "comparisons/provider-attempt/p/ProviderAttempt.pproj",
    },
}
errors = []
models = validation.get("models", {})
for model, required_paths in expected_paths.items():
    model_record = models.get(model)
    if not isinstance(model_record, dict):
        errors.append(f"missing comparison validation record: {model}")
        continue
    bindings = model_record.get("source_sha256")
    if not isinstance(bindings, dict):
        errors.append(f"missing source_sha256 object: {model}")
        continue
    observed_paths = set(bindings)
    if observed_paths != required_paths:
        missing = sorted(required_paths - observed_paths)
        unexpected = sorted(observed_paths - required_paths)
        errors.append(
            f"{model} source binding path mismatch: "
            f"missing={missing!r}, unexpected={unexpected!r}"
        )
    for path_text, expected_digest in sorted(bindings.items()):
        if not isinstance(expected_digest, str) or re.fullmatch(
            r"[0-9a-f]{64}", expected_digest
        ) is None:
            errors.append(f"invalid SHA-256 binding for {model}: {path_text}")
            continue
        path = Path(path_text)
        if not path.is_file():
            errors.append(f"bound comparison source is not a file: {path_text}")
            continue
        actual_digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual_digest != expected_digest:
            errors.append(
                f"stale {model} comparison source binding: {path_text}; "
                f"expected {expected_digest}, observed {actual_digest}"
            )
if errors:
    raise SystemExit("comparison source binding validation failed:\n- " + "\n- ".join(errors))
print(
    "Comparison source bindings current: "
    + ", ".join(
        f"{model}={len(paths)}" for model, paths in expected_paths.items()
    )
)
PY

nmlt_report="$comparison_tmp/nmlt-provider-attempt.json"
cargo run --quiet -p nmlt-cli -- model-check --json \
  examples/technicus/provider_attempt.nmlt >"$nmlt_report"
python3 - "$nmlt_report" <<'PY'
import json
import sys

report_path = sys.argv[1]
with open(report_path, encoding="utf-8") as handle:
    report = json.load(handle)

expected_properties = {
    "DispatchRequiresArm",
    "SelectionRequiresPassingEvidence",
    "NoBlindReplay",
}
observed_properties = {item.get("property") for item in report.get("properties", [])}
errors = []
if report.get("result") != "model_checked":
    errors.append(f"overall result is {report.get('result')!r}, not 'model_checked'")
if report.get("complete") is not True:
    errors.append("reachable frontier was not exhausted within the reported bounds")
if report.get("explored_states") != 5 or report.get("explored_transitions") != 6:
    errors.append(
        "frozen graph-size mismatch: expected 5 states/6 transitions, "
        f"observed {report.get('explored_states')!r} states/"
        f"{report.get('explored_transitions')!r} transitions"
    )
if report.get("bounds") != {"max_states": 10_000, "max_depth": 100}:
    errors.append(f"checker-bound mismatch: observed {report.get('bounds')!r}")
if observed_properties != expected_properties:
    errors.append(
        "property set mismatch: "
        f"expected {sorted(expected_properties)!r}, observed {sorted(observed_properties)!r}"
    )
for item in report.get("properties", []):
    if item.get("result") != "model_checked" or item.get("witness") is not None:
        errors.append(
            f"property {item.get('property')!r} was not model_checked without a witness"
        )
if errors:
    raise SystemExit("NMLT comparison failed:\n- " + "\n- ".join(errors))

bounds = report["bounds"]
print(
    "NMLT comparison passed: "
    f"{report['explored_states']} states, {report['explored_transitions']} transitions, "
    f"frontier exhausted within max_states={bounds['max_states']} and "
    f"max_depth={bounds['max_depth']}"
)
PY

npm ci --prefix "$repo_root/tools/quint" --ignore-scripts --no-audit --no-fund
"$repo_root/tools/quint/node_modules/.bin/quint" typecheck \
  comparisons/provider-attempt/quint/provider_attempt.qnt

if [[ -n "${TLA2TOOLS_JAR:-}" ]]; then
  java -cp "$TLA2TOOLS_JAR" tlc2.TLC -cleanup -deadlock \
    -metadir "$comparison_tmp/tlc-states" \
    -config comparisons/provider-attempt/tla/ProviderAttempt.cfg \
    comparisons/provider-attempt/tla/ProviderAttempt.tla
else
  echo "skip: set TLA2TOOLS_JAR to run the pinned TLC comparison"
fi

if command -v p >/dev/null 2>&1; then
  cp -R comparisons/provider-attempt/p "$comparison_tmp/p"
  (
    cd "$comparison_tmp"
    p compile --pproj p/ProviderAttempt.pproj
    p check PGenerated/PChecker/net8.0/ProviderAttempt.dll \
      --testcase tcProviderAttempt --schedules 100 --max-steps 100 \
      --fail-on-maxsteps --outdir "$comparison_tmp/PCheckerOutput"
  )
else
  echo "skip: install P 3.1.0 to compile and systematically test the P comparison"
fi
