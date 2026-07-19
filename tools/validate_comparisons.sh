#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

cargo run --quiet -p nmlt-cli -- check examples/technicus/provider_attempt.nmlt
npx --yes @informalsystems/quint@0.32.0 typecheck \
  comparisons/provider-attempt/quint/provider_attempt.qnt

if [[ -n "${TLA2TOOLS_JAR:-}" ]]; then
  java -cp "$TLA2TOOLS_JAR" tlc2.TLC -cleanup -deadlock \
    -config comparisons/provider-attempt/tla/ProviderAttempt.cfg \
    comparisons/provider-attempt/tla/ProviderAttempt.tla
else
  echo "skip: set TLA2TOOLS_JAR to run the pinned TLC comparison"
fi

if command -v p >/dev/null 2>&1; then
  comparison_tmp="$(mktemp -d)"
  trap 'rm -rf "$comparison_tmp"' EXIT
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
