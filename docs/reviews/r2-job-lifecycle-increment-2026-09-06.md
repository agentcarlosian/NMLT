# R2 second increment: bounded local jobs and recovery

Date: 2026-09-06. Working changes on R1 commit `c1404f1`, following the
[finite run/replay increment](r2-local-execution-increment-2026-09-06.md).
R2 remains **In progress**. [RFC 0018](../../rfcs/0018-bounded-local-job-lifecycle.md)
is Under review. See the [runtime guide](../../crates/nmlt-runtime/README.md).

## Implemented scope

- `nmlt-runtime` provides bounded run/task/slot/generation identities, typed
  request/response bindings, revisioned owner control, and immutable collected
  outcomes. Old controls fail after transfer, dispatch, settlement, collection,
  and recovery; a reused slot gets a new generation.
- Cancellation before dispatch refunds only the unused work reservation.
  Dispatched work stays charged through cancellation, failure, timeout, and
  restart. Adapter-reported usage remains separate and can be unknown or exceed
  the reservation; the library does not claim host CPU/time enforcement.
- A locked canonical JSONL journal records each transition before returning its
  receipt. Dispatch intent is synced before work can start. Opening validates
  the exact executable, run context, sequence, hash chain, derived receipts,
  and reconstructed state, then records recovery before exposing live control.
- Possible in-flight work becomes Uncertain. Recovery does not redispatch it.
  Reconciliation requires current control and matching adapter evidence;
  cancellation intent prevents a late success from becoming an accepted output.
- A real arithmetic subprocess demonstrates ownership transfer, a failed `-3`
  input, fallback to `5`, and two uses of the collected result `25`. Direct input
  `6` returns `36`; exhausting negative inputs returns failure.
- `make r2-jobs`, included in `make ci`, checks the three subprocess cases and
  their journal order, attempt counts, accounting, and result reuse.

This increment is a Rust API and adapter prototype. It does not add `.nmlt`
effects, change the finite interpreter, alter behavioral artifact schemas, or
add Lean definitions. Its outputs report `assurance: none`.

## Validation

| Check | Result |
|---|---|
| Rust formatting, workspace check, Clippy with warnings denied | Passed |
| Native Windows `cargo test --workspace --all-targets` | 234 passed, 0 failed |
| Linux `cargo test --workspace --all-targets` | 235 passed, 0 failed |
| New runtime tests | 19 Windows / 20 Linux, including the crash-child test entry point |
| Frozen bounded lifecycle exploration | 291 states, 520 accepted transitions; two slots, two generations, two allocations, four events |
| Response controls | Ten binding mutations plus unsupported schema/version, wrong output type, and unsolicited cancellation acknowledgement checked |
| Journal rejection controls | Twelve corrupted/reordered/torn logs rejected without automatic repair |
| Injected write/sync failures | No dispatch receipt or in-memory commit; handle poisoned; full surviving intent recovers uncertain; partial tail rejected |
| Process death at two barriers | Kill after synced dispatch and after worker output survives without journal completion; lock released, charge retained, no redispatch |
| Recovery after surviving output | Ordinary delivery remains ignored; explicit validated reconciliation permits collection of `25` with exactly one recorded dispatch |
| Local subprocess cases | Failure/fallback, direct success, and all-failed passed on Windows and Linux |
| Fresh pinned Lean metatheory gate | Build, decoder controls, placeholder policy, and focused axiom audit passed |
| Fresh NanoDA check | 8,655 declarations checked with no errors |
| V1 artifact reproduction | Byte-for-byte equality retained |
| Frozen value graph comparison | 1 initial, 4 reachable states, 10 transitions; unchanged digest |
| Frozen resource graph comparison | 1 initial, 8 reachable states, 12 transitions; unchanged digest |
| Existing v2 execution rejection controls | All 30 rejected by the actual Lean executable |
| R0 harness and real reference workflows | 14 harness tests; proof 3/3, discovery 2/2, worker 4/4 completed |
| Public-surface/link inventory and `git diff --check` | Passed |

The bounded alphabet is a Rust implementation regression, not an independent
model-check certificate or a proof over all configurations. Focused tests cover
longer cancellation, slot-reuse, and restart sequences outside its four-event
bound. The process tests cover process death, not arbitrary power loss.

The first Linux run exposed a real filesystem mismatch: on the tested WSL
Windows mount, a second hardlink opened the same inode without respecting the
first handle's lock. The test was retained. Unix journal creation, opening, and
each commit now reject multiply linked files; adding a hardlink poisons further
commits. Native Windows separately passes the hardlink locking test. Neither
result establishes locking across kernels or copied/replaced journals.

The full Linux gate ran from `2026-09-06T23:50:06Z` to
`2026-09-06T23:58:34Z`. Final Rust workspace checks were rerun on both platforms
after adding the second crash barrier and strict stale reconciliation rejection.
Lean used a fresh stage with all 29 package files matching the workspace by
SHA-256: 26 source/test/package/toolchain files plus three documentation/config
files. Existing proposition-definition linter warnings remain.

## Independent checker provenance

Lean remains pinned to 4.33.1. The fresh checker inputs were:

- `lean4export`: `411dce7db58a3afc60ecab2d211acd1042b593dc`
- NanoDA: `05055695879dfebb6628a67da88ceca6cd6b0421`
- Rust: 1.94.0
- Export: 2,354 selected module constants and their dependency closure;
  49,598,519 bytes, 959,988 NDJSON lines, 8,655 checked declarations
- Export SHA-256: `7122b8b23698904920885f73813c038dc718accb8945c99ab2bf69bcd368094f`
- Constant-list SHA-256: `e7dbc9f69915728ef80036a209f9393f8627968706592ddd8c5acd5dfce6c9c1`
- Checker-config SHA-256: `556fa3f342120c2d37b59685902010080222c973895988ff106ffd2dcf695d56`

The unchanged axiom policy permits `propext`, `Quot.sound`, and
`Classical.choice`, with unpermitted axioms treated as errors. This check covers
the existing Lean package; it does not verify the new Rust job lifecycle,
journal, subprocess transport, or a correspondence with host execution.

Local evidence is intentionally ignored by version control:

- `.cache/r2-jobs/full-gate.log` and `verify.sh`
- `.cache/r2-jobs/windows-tests.log` and `linux-final-rust.log`
- `.cache/r2-jobs/first-gate-lock-failure.log`
- `.cache/r2-jobs/lean-source-hashes.json` and `runtime-source-hashes.json`
- `.cache/r2-jobs/checker-artifacts/run.UG2J6v/`
- `target/r2-jobs/run-pq8rnwq7/` (final Windows example records)
- `target/r2-jobs/run-r9cdc1ty/` (Linux example records from the full gate)
- `target/r0-baselines/run.0tULC2/`

## Remaining gates

Next are source entry points, tagged outcomes, reusable definitions and typed
lowering, followed by native source effects and typed worker/Lean adapters.
Host cancellation delivery, process limits, recovery/reconciliation UX,
user-defined safety invariants, project/toolchain locks, structured diagnostics,
formatter, `init`/`test`, and the real-input R2 workflow exit gate remain open.

The journal assumes a trusted local coordinator, distinct run identities, and
an intact history on a cooperating filesystem. It does not fence copies or
rollback, authenticate owners or rewritten logs, guarantee power-loss durability,
or enforce exactly-once external effects. Event/byte exhaustion and torn logs
require external reconciliation. The blocking arithmetic example is not a
general sandbox or scheduler for untrusted programs.

Local implementation review only: no independent agent or cross-family
publication review is claimed. RFC acceptance and publication review remain
separate gates. No change was committed or published by this task.
