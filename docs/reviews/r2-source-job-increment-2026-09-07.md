# R2 sixth increment: bounded source job effects

Date: 2026-09-07 (America/Chicago). This extends the uncommitted source workflow,
collection, and package increments above `80e2a0c`. R2 remains **In progress**;
[RFC 0022](../../rfcs/0022-source-local-job-effects.md) is Under review.
See the [source job guide](../r2-source-jobs.md).

## Implemented scope

`job_square(Int)` now lowers to a private typed effect node with result
`Outcome<Int>`. Conservative effects propagate through all calls and imports,
including unselected branches. The pure executor rejects effectful entries.
An explicit host interface passes only scalar results/domain errors into source
values; reusable values cannot carry job control.

The CLI binds source closure, typed program, entry, inputs, exact executable,
adapter, and explicit limits into a context saved before dispatch. Each job
uses the existing locked journal to reserve, dispatch, settle, and collect.
The fixed current-executable worker runs over bounded concurrent pipes with
an elapsed-time limit and direct-child termination/reaping checks. No source
command selection or shell was added.

The first source adapter is synchronous. Negative inputs and square overflow
produce ordinary typed domain failures. A source branch can select a bounded
fallback and reuse its collected value. Host failures instead stop evaluation,
record uncertain charged work, and retain unknown observed work. A job limit
stops before new allocation. Journal I/O failures abort with context/journal
evidence and may leave an incomplete final record.

Job records contain ordered source/input/result events and exact canonical
journal bytes. Replay recompiles the complete source package, validates the
context and journal, consumes events at the same locations/inputs, and checks
the exact lifecycle commands, state, accounting, expression steps, and stop.
It has no process launcher or live journal authority. `jobs-recover` opens the
durable context/journal and classifies unfinished work without redispatch or
source resumption. Pure run/replay remains version 3; workflow typecheck output
is version 4; local job runs use a separate version 1 profile.

All additions are **executable-only**, with `assurance: none`. No shared parser,
M9 definition, finite artifact schema, or Lean source changed. Replay does not
authenticate provenance or establish that recorded physical host observations
occurred. The supervisor does not enforce CPU/memory quotas or process-tree
isolation. Recovery cannot discover/kill surviving workers or reconcile results.

## Validation

| Check | Result |
|---|---|
| Windows formatting / Clippy, warnings denied | Passed |
| Windows / Linux workspace Rust tests | 287 / 289 passed |
| Additional controls | 6 workflow, 6 CLI integration, 5 supervisor (including subprocess probe), 1 host-failure test |
| Static effects | Transitive/dead-branch/entry-specific/imported effects and invalid built-in types/names checked |
| Evaluation | Fold order, source locations, input/step preflight, typed fallback, and result reuse checked |
| Real subprocess | Success, negative input, square overflow, fallback, collection, and exhausted job budget checked |
| Process supervision | Blocked stdin, both output pipes, stderr, failed spawn, nonzero exit, timeout and direct-child reaping checked |
| Uncertain host failure | Injected cleanup-success/failure observations retain one charged uncertain attempt, prevent fallback, and replay without journal mutation |
| Replay rejection | Context, inputs, limits, executable, program, source location, response binding/value, event counts, accounting, steps, torn journal, unknown/duplicate fields checked |
| Recovery | Complete unfinished dispatch prefix becomes uncertain; torn prefix rejected; no new dispatch |
| Windows/Linux example | Both return `Ok(50)` in 12 expression steps with two charged attempts and matching program/source identity |
| Existing subprocess gate / Python R0 harness | 3 subprocess cases / 14 tests passed |
| Fresh Lean package | Build, decoder controls, no-sorry policy, and focused axiom audit passed |
| Independent NanoDA | 8,655 declarations checked with no errors |
| Frozen value graph | 1 initial, 4 states, 10 transitions; unchanged digest |
| Existing execution controls | 30 rejected by the actual Lean executable |
| Frozen resource graph | 1 initial, 8 states, 12 transitions; unchanged digest |
| R0 real workflows | Proof 3/3, discovery 2/2, worker 4/4 completed |
| Public surface and whitespace | Links, trust inventory, repository hygiene, and `git diff --check` passed |

The complete fresh gate ran from `2026-09-07T07:03:22Z` to
`2026-09-07T07:14:00Z` and exited successfully.

All 29 Lean package files match the fresh stage
`.cache/r2-effects/lean.Qvr5iK/` by SHA-256. Existing Lean proposition-definition
linter warnings remain. These regression checks do not verify the new source
effect typing, evaluator host boundary, supervisor, journal capture, or CLI replay.

## Identities and local evidence

The source job example's typed program has SHA-256
`a6e92c2f7c47f7e36b375a6eaad7043b12edc90e75073c8d7e0f815b9ce07076`.
Its 260-byte source has SHA-256
`f608f0d5801793663d369a9904bc6c5fa80b216047ed8511f52e0ad690407ee3`.
The Windows demonstration executable has SHA-256
`1b69e11fb73637559c284bf7721596edfe90eccfe0fcb93f7ec29bdd9b7d1209`.
The recorded worker observations are separately bound to each run identifier,
generation, source location, input, adapter, and context. Cross-platform runs
have distinct executable identities and replay with their own exact binaries.

The independent check used Lean 4.33.1, Rust 1.94.0, exporter
`411dce7db58a3afc60ecab2d211acd1042b593dc`, and NanoDA
`05055695879dfebb6628a67da88ceca6cd6b0421`, selecting 2,354 module constants.
The unchanged dependency export has SHA-256
`7122b8b23698904920885f73813c038dc718accb8945c99ab2bf69bcd368094f`.
The constant-list and checker-configuration hashes are retained in provenance.

Local evidence is intentionally ignored by version control:

- `.cache/r2-effects/verify.sh` and `full-gate.log`
- `.cache/r2-effects/windows-tests.log` and `windows-clippy.log`
- `.cache/r2-effects/windows-run.json`, `windows-replay.json`, and `windows-typecheck.json`
- `.cache/r2-effects/linux-example.sh`, `linux-run.json`, `linux-replay.json`, and `linux-typecheck.json`
- `.cache/r2-effects/windows-jobs/` and `linux-jobs/`
- `.cache/r2-effects/platform-agreement.json`
- `.cache/r2-effects/verify-identities.ps1`, `source-hashes.json`, and `lean-source-hashes.json`
- `.cache/r2-effects/checker-artifacts/run.OAl4WT/`
- `target/r2-jobs/run-gk3_dunm/`
- `target/r0-baselines/run.lW6DHD/`
- `target/r2-source-job-tests/` and `target/r2-source-host-failure/`

## Remaining work

Supported Lean jobs, asynchronous source start/observe/cancel/collect controls,
stronger containment, reconciliation/resumption, user-defined safety invariants,
project/toolchain locks and commands, and the complete real-input R2 exit gate
remain outstanding. RFC acceptance and independent publication review are open
gates. No independent review is claimed. These changes remain uncommitted.
