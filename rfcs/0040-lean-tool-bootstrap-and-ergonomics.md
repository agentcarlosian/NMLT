# RFC 0040: Lean Tool Bootstrap and Ergonomic `lean-task` Defaults

- Status: Under review
- Authors: NMLT maintainers
- Target: `crates/nmlt-cli`

## Summary

Add `nmlt tools status` and `nmlt tools setup` to inspect and build the pinned
independent Lean verification toolchain (`lean4export` and `nanoda`) in a
verified local cache, add `elan` auto-resolution for `--lean-bin`, and
allow ergonomic `lean-task` defaults (`--task DIR`, `--record DIR`, and
`--proof FILE` directly on `lean-task prove`) while preserving every
cryptographic pin and explicit-flag override.

## Problem

Running a single independently checked proof previously required passing up to
seven explicit filesystem paths and hashes on every invocation (`--task`,
`--task-sha256`, `--candidate`, `--lean-bin`, `--exporter`, `--nanoda`,
`--output`) plus manually running `tools/prepare_lean_task_checkers.sh`. That
ergonomic friction blocks everyday interactive and agent-assisted use without
adding any security over resolving default paths and still pinning the exact
canonical executable SHA-256 digests into every retained artifact.

## Design

### 1. `nmlt tools status` and `nmlt tools setup`

`nmlt tools` manages the same pinned commits used by CI and
`tools/prepare_lean_task_checkers.sh`:

- Lean toolchain: pinned in `mechanization/lean/lean-toolchain` (`v4.33.1`)
- `lean4export`: `411dce7db58a3afc60ecab2d211acd1042b593dc`
- `nanoda_lib`: `05055695879dfebb6628a67da88ceca6cd6b0421`

Default cache directory:

1. `NMLT_TOOLS_DIR` if set and non-empty.
2. `$HOME/.nmlt/tools/v<lean-version>` (or `%USERPROFILE%\.nmlt\tools\v<lean-version>` on Windows).

`nmlt tools setup [--dir DIR]` fetches the exact commits into `DIR`, verifies
`git rev-parse HEAD` equals the pinned commit before building, builds
`lean4export` with `lake build` and `nanoda_bin` with `cargo build --locked --release`,
in a fresh installation directory, then verifies the binaries and exporter
library tree before publishing `DIR/checker-provenance-v2.json` atomically.
That file records the source pins, Rust version, generation, and SHA-256
digests. A failed replacement leaves the last published generation intact;
the command never removes older generations or pre-existing tool directories.
An old flat-layout cache is not trusted automatically. Run setup once to
publish a checked generation; those old files remain untouched.

`nmlt tools status [--dir DIR] [--json]` inspects whether Lean, `lean4export`,
and `nanoda_bin` resolve, reporting paths, digests, source labels, required pins,
and `ready: true|false`. Automatically resolved checkers must match the
published provenance. Environment or explicit overrides remain trusted local
selections, not verified cache entries. Readiness is not proof acceptance.

### 2. Auto-resolution without weakening cryptographic pins

Resolution order (strictest explicit override first):

- Lean executable: `--lean-bin` -> `NMLT_LEAN_BIN` ->
  `~/.elan/toolchains/leanprover--lean4---v<version>/bin/lean[.exe]` ->
  `elan which --toolchain leanprover/lean4:v<version> lean`.
- Independent checkers: `--exporter`/`--nanoda` ->
  `NMLT_EXPORTER_BIN`/`NMLT_NANODA_BIN` -> the published cache generation under
  `default_cache_dir()`, only if provenance and the selected checker’s file
  identities match.

Regardless of how a path is discovered, `lean_task` still canonicalizes every
binary, verifies the exact `lean::Toolchain` installation manifest, hashes
`lean4export` (binary + `lib/lean/Main.olean` tree) and `nanoda`, and verifies
unchanged digests after checking.

### 3. Ergonomic `lean-task` defaults

- `--task DIR`: if `DIR` is a directory containing `task.json`, resolves
  `DIR/task.json`. If `--task-sha256` is omitted and `DIR/task.sha256` exists,
  reads the 64-hex digest from `DIR/task.sha256` and still verifies
  `task.digest() == digest`.
- `--record DIR`: if `DIR` is a directory containing `result.json`, resolves
  `DIR/result.json` (and `DIR/task.sha256` when `--task-sha256` is omitted).
- `--proof FILE` on `lean-task prove`: mutually exclusive with `--candidate`.
  Constructs and validates `Candidate::from_file` directly without a separate
  `lean-task candidate` invocation, while retaining the exact `Candidate` object
  in `result.json`.

## Compatibility and Trust Boundary

- Every existing explicit flag invocation continues to work identically.
- `nmlt tools setup` invokes `git`, `lake`, and `cargo` over the network to
  populate the user's cache and is explicitly documented as a bootstrap tool,
  never part of proof acceptance. Proof checking still requires exact binary
  hashes and unchanged post-run verification.
