#!/usr/bin/env bash
set -euo pipefail

# lean-action v1.5.0's built-in nanoda path combines an old nanoda `debug`
# branch with an unpinned lean4export HEAD. Pin a compatible NDJSON exporter
# and checker pair until the action provides equivalent immutable inputs.
readonly LEAN4EXPORT_COMMIT="a3e35a584f59b390667db7269cd37fca8575e4bf"
readonly NANODA_COMMIT="f58f2f6d535e189a40fcb02ede8eb95f97a92d37"
readonly NANODA_RUST_TOOLCHAIN="1.94.0"

readonly LAKE_PACKAGE_DIR="${1:?usage: check_nanoda.sh <lake-package-directory> [module]}"
readonly MODULE_NAME="${2:-NMLT}"

if [[ ! -f "${LAKE_PACKAGE_DIR}/lakefile.toml" && ! -f "${LAKE_PACKAGE_DIR}/lakefile.lean" ]]; then
  echo "error: no Lake package at ${LAKE_PACKAGE_DIR}" >&2
  exit 2
fi

if [[ ! "${MODULE_NAME}" =~ ^[A-Za-z_][A-Za-z0-9_.]*$ ]]; then
  echo "error: invalid Lean module name: ${MODULE_NAME}" >&2
  exit 2
fi

for command_name in cargo git lake rustup; do
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "error: required command is unavailable: ${command_name}" >&2
    exit 2
  fi
done

readonly TEMP_PARENT="${RUNNER_TEMP:-/tmp}"
NANODA_TEMP_DIR="$(mktemp -d "${TEMP_PARENT}/nmlt-nanoda.XXXXXX")"
readonly NANODA_TEMP_DIR

cleanup() {
  rm -rf -- "${NANODA_TEMP_DIR}"
}
trap cleanup EXIT

fetch_commit() {
  local repository_url="$1"
  local commit="$2"
  local destination="$3"

  git init --quiet "${destination}"
  git -C "${destination}" remote add origin "${repository_url}"
  git -C "${destination}" fetch --quiet --depth 1 origin "${commit}"
  git -C "${destination}" checkout --quiet --detach FETCH_HEAD

  local actual_commit
  actual_commit="$(git -C "${destination}" rev-parse HEAD)"
  if [[ "${actual_commit}" != "${commit}" ]]; then
    echo "error: fetched ${actual_commit}, expected ${commit}" >&2
    exit 1
  fi
}

readonly EXPORTER_DIR="${NANODA_TEMP_DIR}/lean4export"
readonly CHECKER_DIR="${NANODA_TEMP_DIR}/nanoda_lib"
readonly EXPORT_FILE="${NANODA_TEMP_DIR}/environment.ndjson"
readonly CONFIG_FILE="${NANODA_TEMP_DIR}/nanoda-config.json"

echo "Fetching lean4export ${LEAN4EXPORT_COMMIT}"
fetch_commit \
  "https://github.com/leanprover/lean4export.git" \
  "${LEAN4EXPORT_COMMIT}" \
  "${EXPORTER_DIR}"

# The exporter must use the same exact Lean toolchain as the checked package.
cp "${LAKE_PACKAGE_DIR}/lean-toolchain" "${EXPORTER_DIR}/lean-toolchain"
(
  cd "${EXPORTER_DIR}"
  lake build
)

echo "Fetching nanoda ${NANODA_COMMIT}"
fetch_commit \
  "https://github.com/ammkrn/nanoda_lib.git" \
  "${NANODA_COMMIT}" \
  "${CHECKER_DIR}"
rustup toolchain install "${NANODA_RUST_TOOLCHAIN}" --profile minimal
cargo "+${NANODA_RUST_TOOLCHAIN}" build \
  --locked \
  --release \
  --manifest-path "${CHECKER_DIR}/Cargo.toml"

echo "Exporting ${MODULE_NAME}"
(
  cd "${LAKE_PACKAGE_DIR}"
  lake env "${EXPORTER_DIR}/.lake/build/bin/lean4export" "${MODULE_NAME}"
) > "${EXPORT_FILE}"

echo "Export bytes: $(wc -c < "${EXPORT_FILE}")"
echo "Export lines: $(wc -l < "${EXPORT_FILE}")"

if [[ "${NANODA_ALLOW_SORRY:-false}" == "true" ]]; then
  readonly PERMITTED_AXIOMS='["propext", "Classical.choice", "Quot.sound", "Lean.trustCompiler", "sorryAx"]'
else
  readonly PERMITTED_AXIOMS='["propext", "Classical.choice", "Quot.sound", "Lean.trustCompiler"]'
fi

{
  echo '{'
  echo '  "export_file_path": "environment.ndjson",'
  echo '  "use_stdin": false,'
  echo "  \"permitted_axioms\": ${PERMITTED_AXIOMS},"
  echo '  "unpermitted_axiom_hard_error": false,'
  echo '  "unsafe_permit_all_axioms": false,'
  echo '  "nat_extension": true,'
  echo '  "string_extension": true,'
  echo '  "print_axioms": false,'
  echo '  "print_success_message": true'
  echo '}'
} > "${CONFIG_FILE}"

echo "Checking ${MODULE_NAME} with pinned nanoda"
(
  cd "${NANODA_TEMP_DIR}"
  "${CHECKER_DIR}/target/release/nanoda_bin" "${CONFIG_FILE}"
)
