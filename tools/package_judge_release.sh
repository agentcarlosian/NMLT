#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
BUNDLE_NAME=${1:-nmlt-build-week-judge-demo-2026}
OUT_DIR=${2:-"$ROOT/dist"}
NMLT_BIN=${NMLT_BIN:-"$ROOT/target/release/nmlt"}

if [[ ! -x "$NMLT_BIN" ]]; then
    printf 'missing release binary: %s\n' "$NMLT_BIN" >&2
    printf 'build it with: cargo build -p nmlt-cli --release\n' >&2
    exit 2
fi

mkdir -p "$OUT_DIR"
OUT_DIR=$(CDPATH= cd -- "$OUT_DIR" && pwd)
STAGING=$(mktemp -d)
trap 'rm -rf "$STAGING"' EXIT
BUNDLE="$STAGING/$BUNDLE_NAME"

mkdir -p "$BUNDLE/demos"
install -m 0755 "$NMLT_BIN" "$BUNDLE/nmlt"
install -m 0755 "$ROOT/judge-demo.sh" "$BUNDLE/judge-demo.sh"
cp -R "$ROOT/demos/judge" "$BUNDLE/demos/judge"
install -m 0644 "$ROOT/JUDGE_QUICKSTART.md" "$BUNDLE/JUDGE_QUICKSTART.md"
install -m 0644 "$ROOT/LICENSE" "$BUNDLE/LICENSE"

REVISION=$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || printf 'unknown')
cat > "$BUNDLE/BUILD_INFO.txt" <<INFO
release_tag=build-week-judge-demo-2026
source_revision=$REVISION
supported_platform=Ubuntu 24.04 x86_64, glibc 2.39
python_requirement=Python 3.11 or newer
network_required_after_download=no
INFO

(
    cd "$BUNDLE"
    find . -type f ! -name SHA256SUMS -print0 \
        | sort -z \
        | xargs -0 sha256sum > SHA256SUMS
)

ARCHIVE="$OUT_DIR/$BUNDLE_NAME-linux-x86_64.tar.gz"
tar \
    --sort=name \
    --mtime=@0 \
    --owner=0 \
    --group=0 \
    --numeric-owner \
    -C "$STAGING" \
    -cf - "$BUNDLE_NAME" \
    | gzip -n > "$ARCHIVE"

(
    cd "$OUT_DIR"
    sha256sum "$(basename "$ARCHIVE")" > "$(basename "$ARCHIVE").sha256"
)

printf 'archive: %s\n' "$ARCHIVE"
printf 'checksum: %s.sha256\n' "$ARCHIVE"
cat "$ARCHIVE.sha256"
