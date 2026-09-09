# R2 fourth increment: records and bounded collections

Date: 2026-09-06 (America/Chicago). This extends the third pure-workflow
increment in the working tree above `80e2a0c`. No new commit or push was made.
R2 remains **In progress** and [RFC 0020](../../rfcs/0020-workflow-records-and-collections.md)
is Under review. See the [guide](../r2-pure-workflows.md).

This records the version 2 implementation before the
[fifth increment](r2-package-increment-2026-09-07.md), which adds source packages
and moves the current pure formats to version 3. Identities and counts below
describe the earlier completed gate.

## Implemented scope

The canonical declaration projection now routes retained record slices to the
pure workflow compiler. Nominal record types support forward and module-qualified
references, exact named fields, constructors, and projections. Recursive record
dependencies and excessive expanded type depth fail before execution.

`List<T>` contains at most 256 elements. List literals, length, safe lookup,
and ordered folds operate on immutable values. Fold bodies keep the accumulator
type, observe lexical scope, participate in recursion checking, and share the
existing step budget. Empty folds return their initial value without executing
their checked body. CLI inputs decode ordinary JSON against the entry signature,
including records, lists, and outcomes; duplicate keys and extra fields fail.

Per-value node, payload-byte, and depth bounds limit aggregates. Cumulative
value-work accounting limits repeated production and copying. Both new runtime
stops retain a source span and step count and replay as incomplete outcomes.
These are deterministic evaluator limits, not host process quotas.

The profile and pure typecheck/run/replay schemas explicitly move to version 2.
Version 1 pure records require their original executable or a fresh source run;
there is no silent evidence migration. Finite schemas, M9, the shared frontend,
and all Lean sources remain unchanged. New constructs are **executable-only**
and always retain `assurance: none`.

## Executed validation

| Check | Result |
|---|---|
| Formatting and Clippy with warnings denied, all targets | Passed |
| Native Windows / Linux workspace Rust tests | 257 / 258 passed, no failures |
| Added collection library / CLI tests | 7 / 3; 10 additional tests |
| Record semantics | Forward/module resolution, arbitrary projection, empty records, nominal mismatch, unknown/duplicate fields, recursive types checked |
| Collection semantics | Ordered/empty/nested folds, binder shadowing, 256-item boundary, homogeneous types, safe lookup endpoints, shared steps and arithmetic stops checked |
| Inputs and growth | Exact structured inputs, nested outcomes, text/list/node/byte/depth limits, and repeated-copy limits checked |
| CLI and replay | Structured batches, nested duplicate keys, nominal/result mutations, v1 rejection, and both new incomplete stops checked |
| Windows batch demonstration | Three input records yield accepted 2, rejected 1, total 41 in 100 steps; empty list yields zeros in 6 steps; both replay |
| Existing subprocess job gate | All 3 cases passed |
| Python R0 harness | All 14 tests passed |
| Fresh Lean metatheory | Build, decoder controls, no-sorry policy, and focused axiom audit passed |
| Fresh independent NanoDA | 8,655 declarations checked with no errors |
| Frozen value graph | 1 initial, 4 states, 10 transitions; unchanged digest |
| Frozen resource graph | 1 initial, 8 states, 12 transitions; unchanged digest |
| Existing execution controls | 30 rejected by the real Lean executable |
| R0 real workflows | Proof 3/3, discovery 2/2, worker 4/4 completed |
| Public links, trust inventory, whitespace | Passed |

The complete gate ran from `2026-09-07T01:05:01Z` to
`2026-09-07T01:13:26Z` and exited successfully. Its fresh stage was
`.cache/r2-collections/lean.BrcfHV/`.

The existing Lean proposition-definition linter warnings remain. All 29 Lean
package files matched the fresh build stage by SHA-256. These checks preserve
the existing mathematical baseline; they do not prove the new Rust typing or
evaluation rules. No independent review is claimed.

## Identities and local evidence

The [batch source](../../examples/pivot/batch_summary.nmlt) has SHA-256
`9084b30cb3ede75d4f43b35018a333a2a2a49fc9f1e8b32ef7f239958c5c7682`.
Its version 2 typed program has SHA-256
`55109040824bc862c6d05ddf93707a6c2c7e859edc9f911c49ace65bc9dd412d`.
The demonstrated Windows executable has SHA-256
`3f43ecfca661800b08534a9f32b9ec441cf286d43d88f3e7315e58f442d16970`.
These identities are reproducibility evidence, not verified translation or
authenticated provenance.

The fresh checker used Lean 4.33.1, Rust 1.94.0, exporter
`411dce7db58a3afc60ecab2d211acd1042b593dc`, and NanoDA
`05055695879dfebb6628a67da88ceca6cd6b0421`. It selected 2,354 module constants.
Its unchanged dependency export has SHA-256
`7122b8b23698904920885f73813c038dc718accb8945c99ab2bf69bcd368094f`;
the retained provenance records the constant-list and checker-config hashes.

Local evidence is intentionally ignored by version control:

- `.cache/r2-collections/full-gate.log` and `verify.sh`
- `.cache/r2-collections/windows-clippy.log` and `windows-tests.log`
- `.cache/r2-collections/source-hashes.json` and `lean-source-hashes.json`
- `.cache/r2-collections/windows-typecheck.json`, `windows-batch.json`,
  `windows-empty.json`, and their replay records
- `.cache/r2-collections/exercise.py` and `verify_identities.py`
- `.cache/r2-collections/checker-artifacts/run.lTygoy/`
- `target/r2-jobs/run-g7ak27u4/`
- `target/r0-baselines/run.qF8DdF/`

## Remaining gates

Cross-file packages, native source effects and supported job/Lean adapters,
host cancellation/process limits, recovery flows, user-defined invariants,
project commands/locks, and the complete real-input R2 workflow exit gate remain
outstanding. RFC acceptance and publication review are separate open gates.
