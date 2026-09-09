#!/usr/bin/env bash
set -euo pipefail

# lean-action v1.5.0's built-in nanoda path combines an old nanoda `debug`
# branch with an unpinned lean4export HEAD. Pin a compatible NDJSON exporter
# and checker pair until the action provides equivalent immutable inputs.
readonly LEAN4EXPORT_COMMIT="411dce7db58a3afc60ecab2d211acd1042b593dc"
readonly NANODA_COMMIT="05055695879dfebb6628a67da88ceca6cd6b0421"
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

for command_name in cargo git lake rustup sha256sum cmp; do
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "error: required command is unavailable: ${command_name}" >&2
    exit 2
  fi
done

readonly TEMP_PARENT="$(cd "${RUNNER_TEMP:-/tmp}" && pwd -P)"
NANODA_TEMP_DIR="$(mktemp -d "${TEMP_PARENT}/nmlt-nanoda.XXXXXX")"
NANODA_TEMP_DIR="$(cd "${NANODA_TEMP_DIR}" && pwd -P)"
readonly NANODA_TEMP_DIR

cleanup() {
  # Keep cleanup within the resolved scratch parent, including on Windows.
  if [[ "$(dirname -- "${NANODA_TEMP_DIR}")" != "${TEMP_PARENT}" ||
        "$(basename -- "${NANODA_TEMP_DIR}")" != nmlt-nanoda.* ||
        -L "${NANODA_TEMP_DIR}" ]]; then
    echo "error: refusing cleanup outside the checker scratch directory" >&2
    return 1
  fi
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
readonly FILE_EXPORT_SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)/ExportFromFile.lean"

echo "Lean toolchain: $(cat "${LAKE_PACKAGE_DIR}/lean-toolchain")"
(
  cd "${LAKE_PACKAGE_DIR}"
  lake env lean --version
)

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
EXPORTER_BIN="${EXPORTER_DIR}/.lake/build/bin/lean4export"
if [[ -f "${EXPORTER_BIN}.exe" ]]; then EXPORTER_BIN+=".exe"; fi
readonly EXPORTER_BIN

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
rustc "+${NANODA_RUST_TOOLCHAIN}" --version
CHECKER_BIN="${CHECKER_DIR}/target/release/nanoda_bin"
if [[ -f "${CHECKER_BIN}.exe" ]]; then CHECKER_BIN+=".exe"; fi
readonly CHECKER_BIN
sha256sum "${CHECKER_DIR}/Cargo.lock" "${CHECKER_BIN}"

# Export only the checked module's own constants plus their transitive
# dependency closure (dumpConstant recurses into everything they use).
# A whole-environment export additionally drags in unrelated library
# declarations — including dependency-internal `sorry`/`native_decide`
# artifacts NMLT never relies on — which the checker would reject.
readonly CONSTANT_LIST="${NANODA_TEMP_DIR}/checked-constants.txt"
readonly LIST_SCRIPT="${NANODA_TEMP_DIR}/ListConstants.lean"
cat > "${LIST_SCRIPT}" <<EOF
import ${MODULE_NAME}
import Lean.Meta.Basic
open Lean
run_meta do
  let env ← getEnv
  for entry in env.constants.toList do
    let declName := entry.1
    if !declName.isInternal then
      if let some idx := env.getModuleIdxFor? declName then
        if env.header.moduleNames[idx.toNat]!.getRoot == \`${MODULE_NAME} then
          IO.println declName
EOF
echo "Enumerating ${MODULE_NAME} constants"
(
  cd "${LAKE_PACKAGE_DIR}"
  lake env lean "${LIST_SCRIPT}" | sed 's/\r$//'
) > "${CONSTANT_LIST}"
echo "Checked-module constants: $(wc -l < "${CONSTANT_LIST}")"

# The full root list exceeds Windows' process command-line limit. Import the
# unchanged pinned Main and invoke its entry point with roots read from a file.
# Lake augments LEAN_PATH with the checked package's own dependencies.
export_from_file() (
  local constants_file="$1"
  local output_file="$2"
  local exporter_library="${EXPORTER_DIR}/.lake/build/lib/lean"
  local path_separator=':'
  if [[ "${EXPORTER_BIN}" == *.exe ]]; then
    constants_file="$(cygpath -m "${constants_file}")"
    output_file="$(cygpath -m "${output_file}")"
    exporter_library="$(cygpath -m "${exporter_library}")"
    path_separator=';'
  fi
  export LEAN_PATH="${exporter_library}${LEAN_PATH:+${path_separator}${LEAN_PATH}}"
  export NMLT_EXPORT_MODULE="${MODULE_NAME}"
  export NMLT_EXPORT_CONSTANTS="${constants_file}"
  export NMLT_EXPORT_OUTPUT="${output_file}"
  cd "${LAKE_PACKAGE_DIR}"
  lake env lean "${FILE_EXPORT_SCRIPT}"
)

# Check the file adapter against the upstream executable before trusting its
# full export. Both receive exactly the same ordered sample of actual roots.
readonly PARITY_CONSTANT_LIST="${NANODA_TEMP_DIR}/parity-constants.txt"
head -n 8 "${CONSTANT_LIST}" > "${PARITY_CONSTANT_LIST}"
parity_constants=()
while IFS= read -r constant; do
  parity_constants+=("${constant}")
done < "${PARITY_CONSTANT_LIST}"
if [[ "${#parity_constants[@]}" -eq 0 ]]; then
  echo "error: no checked-module constants were enumerated" >&2
  exit 1
fi
(
  cd "${LAKE_PACKAGE_DIR}"
  lake env "${EXPORTER_BIN}" "${MODULE_NAME}" -- "${parity_constants[@]}"
) > "${NANODA_TEMP_DIR}/parity-cli.ndjson"
export_from_file "${PARITY_CONSTANT_LIST}" "${NANODA_TEMP_DIR}/parity-file.ndjson"
cmp "${NANODA_TEMP_DIR}/parity-cli.ndjson" "${NANODA_TEMP_DIR}/parity-file.ndjson"
echo "File export adapter matches pinned CLI output for ${#parity_constants[@]} roots"

echo "Exporting ${MODULE_NAME}"
export_from_file "${CONSTANT_LIST}" "${EXPORT_FILE}"

echo "Export bytes: $(wc -c < "${EXPORT_FILE}")"
echo "Export lines: $(wc -l < "${EXPORT_FILE}")"

if [[ "${NANODA_ALLOW_SORRY:-false}" == "true" ]]; then
  readonly PERMITTED_AXIOMS='["propext", "Quot.sound", "Classical.choice", "sorryAx"]'
else
  readonly PERMITTED_AXIOMS='["propext", "Quot.sound", "Classical.choice"]'
fi

{
  echo '{'
  echo '  "export_file_path": "environment.ndjson",'
  echo '  "use_stdin": false,'
  echo "  \"permitted_axioms\": ${PERMITTED_AXIOMS},"
  echo '  "unpermitted_axiom_hard_error": true,'
  echo '  "unsafe_permit_all_axioms": false,'
  echo '  "nat_extension": true,'
  echo '  "string_extension": true,'
  echo '  "print_axioms": false,'
  echo '  "print_success_message": true'
  echo '}'
} > "${CONFIG_FILE}"

echo "Checking ${MODULE_NAME} with pinned nanoda"
sha256sum "${EXPORT_FILE}" "${CONSTANT_LIST}" "${CONFIG_FILE}"
(
  cd "${NANODA_TEMP_DIR}"
  "${CHECKER_BIN}" "${CONFIG_FILE}"
)

# Optionally retain the exact successful check inputs for local or CI replay.
# Each invocation gets a new directory; existing records are never replaced.
if [[ -n "${NMLT_NANODA_ARTIFACT_DIR:-}" ]]; then
  mkdir -p -- "${NMLT_NANODA_ARTIFACT_DIR}"
  ARTIFACT_RUN_DIR="$(mktemp -d "${NMLT_NANODA_ARTIFACT_DIR%/}/run.XXXXXX")"
  cp -- "${EXPORT_FILE}" "${CONSTANT_LIST}" "${CONFIG_FILE}" "${ARTIFACT_RUN_DIR}/"
  cp -- "${FILE_EXPORT_SCRIPT}" "${PARITY_CONSTANT_LIST}" \
    "${NANODA_TEMP_DIR}/parity-cli.ndjson" "${NANODA_TEMP_DIR}/parity-file.ndjson" \
    "${ARTIFACT_RUN_DIR}/"
  {
    echo "lean_toolchain=$(cat "${LAKE_PACKAGE_DIR}/lean-toolchain")"
    echo "lean4export_commit=${LEAN4EXPORT_COMMIT}"
    echo "nanoda_commit=${NANODA_COMMIT}"
    echo "rust_toolchain=${NANODA_RUST_TOOLCHAIN}"
    echo "module=${MODULE_NAME}"
    (
      cd "${ARTIFACT_RUN_DIR}"
      sha256sum environment.ndjson checked-constants.txt nanoda-config.json \
        ExportFromFile.lean parity-constants.txt parity-cli.ndjson parity-file.ndjson
    )
  } > "${ARTIFACT_RUN_DIR}/provenance.txt"
  echo "Retained checker artifacts: ${ARTIFACT_RUN_DIR}"
fi
