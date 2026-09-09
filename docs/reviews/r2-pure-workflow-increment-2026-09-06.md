# R2 third increment: pure source workflows

Date: 2026-09-06 (America/Chicago). The preceding finite execution and job
lifecycle increments were committed as `80e2a0c` at the user's request before
this work began. The changes described here are a new working increment on
that commit. R2 remains **In progress**; [RFC 0019](../../rfcs/0019-pure-workflow-source.md)
is Under review. See the [user guide](../r2-pure-workflows.md).

This records the version 1 implementation before the
[fourth increment](r2-collections-increment-2026-09-06.md). The current profile
and pure record formats are version 3 after the [package increment](../r2-source-packages.md);
the historical identities and counts
below describe the earlier completed gate.

## Implemented scope

`nmlt-workflow` interprets the canonical frontend's retained function slices.
It reuses the lossless CST, full declaration projection, lexer, and byte spans;
the frozen M9 and finite behavior routes keep their existing meanings.

The new profile supports local module wrappers, named entries with real scalar
inputs, typed parameters/results, reusable acyclic functions, immutable bindings,
`Int`/`Bool`/`Text`, `Outcome<T>`, `Ok`/`Err`, exhaustive matching, conditional
branches, checked arithmetic, and scalar/Boolean operators. Every declaration
and branch is checked, including unused functions. Lexical references and calls
lower to resolved indices in a private typed tree; recursive call cycles fail.

`nmlt run --entry` evaluates that tree with explicit step/depth bounds. Structured
results distinguish returned values from step limits, depth limits, and integer
overflow. A returned domain Err remains Err. `typecheck --profile workflow`
selects this profile explicitly; ordinary `check` still requires a system.

Pure run/replay records bind exact source and executable bytes, the typed
program, entry, inputs, budget, steps, and outcome. Replay accepts record JSON
formatting changes but rejects duplicate keys, unknown fields, inconsistent
results, and unsupported formats. Source can move without changing its bytes.
New output files do not overwrite existing files or source aliases.

All new constructs are **executable-only**. Pure outcomes do not carry affine
job control, execute a subprocess, or become accepted proofs. There is no new
Lean definition, behavioral artifact schema, M9 certificate, or claim of
correspondence with the host job runtime.

## Validation

| Check | Result |
|---|---|
| Formatting, workspace check, Clippy with warnings denied | Passed |
| Native Windows workspace tests, all targets | 247 passed, 0 failed |
| Linux workspace tests, all targets | 248 passed, 0 failed |
| New library / CLI tests | 7 / 6; 13 new tests total |
| Source negative controls | 24 fixed invalid programs, plus operator/string and structural/input limit controls; errors retain valid source spans |
| Pure replay mutation controls | 12 version/identity/input/budget/result changes, 6 unknown-field locations, and 3 duplicate-key cases rejected |
| Record preservation | Existing sources/records retained; invalid inputs/options create no run record |
| Incomplete outcomes | Step limit and integer overflow recorded and replayed without becoming returned values; runtime depth limit covered by library test |
| Input-sensitive example | `-3, 5` returns Ok(50) in 26 steps; `4, 5` returns Ok(32); `-3, -5` retains Err("negative input") |
| Native Text/Bool demonstration | Unicode Text and Bool inputs run and replay successfully |
| Existing finite CLI commands | Regression tests and explicit cross-profile rejection controls passed |
| Existing job subprocess gate | All 3 cases passed |
| Fresh Lean metatheory | Build, decoder controls, placeholder policy, and focused axiom audit passed |
| Fresh independent NanoDA check | 8,655 declarations checked with no errors |
| V1 artifact reproduction | Byte-for-byte equality retained |
| Frozen value graph | 1 initial, 4 states, 10 transitions; unchanged digest |
| Frozen resource graph | 1 initial, 8 states, 12 transitions; unchanged digest |
| Existing v2 execution controls | All 30 rejected by the actual Lean executable |
| R0 harness / real workflows | 14 harness tests; proof 3/3, discovery 2/2, worker 4/4 completed |
| Public links, trusted-component inventory, `git diff --check` | Passed |

The initial Linux verification was deliberately interrupted when the oversized
record control exposed slow unbuffered format detection on the WSL filesystem.
The reader now buffers file access while streaming past unrelated fields. The
oversized-record rejection control was retained and passes; the entire gate
was restarted after the fix. This is a correctness/regression record, not a
performance benchmark or a general host resource guarantee.

The successful full gate ran from `2026-09-07T00:30:41Z` to
`2026-09-07T00:38:58Z`, using Rust 1.94.0 and Lean 4.33.1. It used a fresh Lean
stage. All 29 package files matched the workspace by SHA-256, including 26
source/test/package/toolchain files and three documentation/configuration files.
Existing proposition-definition linter warnings remain.

## Identities and local evidence

The pure fallback source SHA-256 is
`4b934cb9a80e10d106e15cadcf26df2902d2dfe95b1d5501dd03b89cb23224a6`.
Its typed-program SHA-256 is
`648d5a01ae2c17b082a9efce63ea94e085cfe131940bef817f30aca773a25be8`.
These are reproducibility identities, not verified-translation evidence.

The fresh independent checker uses exporter
`411dce7db58a3afc60ecab2d211acd1042b593dc` and NanoDA
`05055695879dfebb6628a67da88ceca6cd6b0421`, with Rust 1.94.0. It selects
2,354 NMLT module constants and exports their dependency closure: 49,598,519
bytes and 959,988 lines. The unchanged axiom policy permits only `propext`,
`Quot.sound`, and `Classical.choice`, with other axioms treated as errors.

- Export SHA-256: `7122b8b23698904920885f73813c038dc718accb8945c99ab2bf69bcd368094f`
- Constant-list SHA-256: `e7dbc9f69915728ef80036a209f9393f8627968706592ddd8c5acd5dfce6c9c1`
- Checker-config SHA-256: `556fa3f342120c2d37b59685902010080222c973895988ff106ffd2dcf695d56`

This fresh check concerns the unchanged Lean package. It does not check the
new function parser, typing rules, evaluator, or run-record implementation.

Local evidence, intentionally ignored by version control:

- `.cache/r2-pure/full-gate.log`, `verify.sh`, and `windows-tests.log`
- `.cache/r2-pure/interrupted-unbuffered-read.log`
- `.cache/r2-pure/source-hashes.json` and `lean-source-hashes.json`
- `.cache/r2-pure/windows-run.json`, `windows-text.json`, and `text.nmlt`
- `.cache/r2-pure/checker-artifacts/run.t2Z33X/`
- `target/r2-jobs/run-7t4tuq1m/`
- `target/r0-baselines/run.RRXMdG/`

## Remaining gates

Record types, bounded collections/iteration, cross-file packages, native source
effects, supported Lean adapters, host cancellation/limits, recovery UX,
user-defined invariants, and the complete project loop remain required by
[Plan.md](../../Plan.md). The pure example exercises real input and result
handling, but does not complete the host-job workflow or all R2 requirements.

Local implementation review only. No independent agent or cross-family
publication review is claimed. RFC acceptance and publication review remain
separate. This new increment was not committed or published by the continuation.
