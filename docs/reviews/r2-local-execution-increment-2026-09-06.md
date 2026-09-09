# R2 first increment: finite local execution

Date: 2026-09-06. Implemented as working changes on R1 commit `c1404f1`.
R2 is **In progress**, not complete. [RFC 0017](../../rfcs/0017-finite-local-run-and-replay.md)
is Under review; this record does not accept that RFC or the broader workflow
profile. See the [run/replay guide](../r2-local-execution.md).

## Implemented scope

- A shared prepared initializer and successor operation serves both graph
  exploration and direct finite execution. V1 operations and canonical fixtures
  retain their previous contract; `run` requires the v2 source route.
- `nmlt run` selects a named leaf or binary composition directly from source,
  uses either deterministic first-enabled scheduling or an explicit label list,
  and enforces a required step bound including self-loops.
- Structured records preserve states, authority, step grades, cumulative model
  grade, and distinct quiescent/schedule-complete/limit/unavailable/overflow
  outcomes. Failed selections and run limits retain the executed prefix.
- `nmlt replay` checks source, artifact, exact executable identity, configuration,
  and the complete reconstructed trace. Successful replay of an incomplete run
  preserves its incomplete outcome. New output files never overwrite existing
  records or sources.
- The finite retry example transfers authority, simulates failure, retries once,
  consumes the permit, and reuses a Boolean output. Changing its source Boolean
  takes a shorter path. It does not execute a host job.

The scheduler, bounds, cumulative accounting, and run-record/replay policy are
executable-only. Both commands report `assurance: none`. No source grammar,
Lean definition, behavioral artifact schema, or existing canonical JSON fixture
changed. The source routes and stop-rule precedence are specified in RFC 0017.

## Validation

| Check | Result |
|---|---|
| Rust formatting, workspace check, Clippy with warnings denied | Passed |
| `cargo test --workspace --all-targets` | 215 passed, 0 failed; 13 new interpreter/CLI tests |
| New interpreter edge coverage | All 22 edges in the two frozen value/resource graphs execute from initial paths |
| New replay controls | 17 identity/data/outcome mutations and 7 unknown-field locations rejected |
| Pinned Lean metatheory gate | Build, decoder controls, placeholder policy, and focused axiom audit passed |
| V1 artifact reproduction | Byte-for-byte equality retained |
| Frozen value graph comparison | 1 initial, 4 states, 10 transitions; unchanged digest |
| Frozen resource graph comparison | 1 initial, 8 states, 12 transitions; unchanged digest |
| Existing v2 execution controls | All 30 rejected by the real Lean checker |
| New finite retry witness | Separately generated five-step v2 path accepted from the decoded initializer |
| R0 harness and real baselines | 14 tests; proof 3/3, discovery 2/2, worker 4/4 completed |
| Native Windows and Linux run/replay demos | Five steps, model work 2, quiescent, matched replay, assurance none |
| Public-surface/link inventory and `git diff --check` | Passed |

The Linux verification run completed from `2026-09-06T18:15:35Z` to
`2026-09-06T18:18:50Z`. It used Rust 1.94.0 and the cached pinned Lean 4.33.1
toolchain, with the existing R1 Lean stage. All 26 staged Lean source, test,
and package/toolchain files matched the current workspace by SHA-256. Existing
proposition-definition linter warnings remain. No fresh clean Lean stage or
NanoDA export was performed for this Rust-only increment; the unchanged Lean
package's independent-check evidence remains the [R1 record](r1-finite-execution-2026-09-06.md).

The new witness is separate from the run-record JSON and was generated through
the existing `elaborate`/`trace` path. Its acceptance does not prove the runner,
serializer, scheduler, source translation, or host correspondence.

Local evidence, intentionally ignored by version control:

- `.cache/r2-start/rust-tests.log`
- `.cache/r2-start/verification.log` and `verify.sh`
- `.cache/r2-start/windows-run.json`
- `.cache/r2-start/demo.8NPMNj/` (Linux run record, v2 core, and Lean-accepted path)
- `target/r0-baselines/run.hdqNaZ/`

The new source SHA-256 is
`67f032f715d3c16f976de73f3c53335f8580ac307c8b24dd2d27066679427d7d`;
its generated core SHA-256 is
`95b2699b4595143954215d7a3c63ae03d5516ca5fb7208d5e344829b581b36ee`.
These identify the tested bytes, not verified compilation.

## Remaining gates

Next: bounded job identities, ownership, settlement, cancellation and uncertain
restart recovery, followed by source/value facilities and typed Lean/worker
adapters. User-defined invariants, project/toolchain locks, source-located
structured diagnostics, formatter, `init`/`test`, and the real-input workflow
exit gate remain required by [Plan.md](../../Plan.md).

This increment received local implementation review. No independent agent or
cross-family publication review is claimed. RFC acceptance and publication
review remain separate gates. No change was committed or published by this task.
