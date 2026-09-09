# R2 fifth increment: reusable source packages

Date: 2026-09-07 (America/Chicago). This extends the uncommitted pure workflow
and collection increments above `80e2a0c`. R2 remains **In progress** and
[RFC 0021](../../rfcs/0021-workflow-source-packages.md) is Under review.
See the [package guide](../r2-source-packages.md).

## Implemented scope

The workflow compiler now interprets canonical file-root imports and loads
sibling source libraries through a bounded reader. It visits a deterministic
import closure, loads shared dependencies once, rejects cycles and namespace
collisions, and checks the entire package under explicit per-file scopes.
Functions and nominal record types cross file boundaries through named imports.
Record-only dependencies are supported; unused imported declarations are checked.

The package limit is 32 source files, 128 KiB per file, 1 MiB combined source,
eight import edges on the longest path, and 64 functions and 64 records total.
The CLI enforces portable/exact-case names, regular sources, and package-directory
resolution, and rejects symbolic source links. These are path checks, not a
race-resistant filesystem sandbox or an atomic project snapshot.

Every lowered term retains a source index and original local byte span. Compiler
diagnostics name the originating file; runtime stops in a library point into
that library's source. A complete ordered source manifest joins the private
typed-program identity and the version 3 pure record contract. Run evaluates the
same captured program used to decode its arguments.

Replay reloads and rechecks all dependencies, including exact bytes of unused
code and comments. Package moves and entry-file renaming retain replay when
logical dependency names and bytes are unchanged. Missing/edited dependencies,
manifest changes, wrong source indices, and older version 1/2 records fail.
Old pure records require their original binaries or a fresh source run.

All new facilities are **executable-only**, with `assurance: none`. No shared
frontend definition, M9 rule, finite schema, or Lean source was changed. No
native source job effect, verified translation, provenance authentication,
or Lean theorem follows from package typing or replay.

## Validation

| Check | Result |
|---|---|
| Formatting / Clippy, warnings denied across all targets | Passed |
| Windows / Linux workspace Rust tests | 269 / 271 passed, no failures |
| Additional package tests | 7 library; 5 Windows CLI / 6 Linux CLI |
| Imports and scopes | Deterministic diamond imports, direct-import requirements, local lookup, record-only dependencies, and namespace collisions checked |
| Bounds and rejection | Cycles, case/device/path restrictions, file/byte/global declaration bounds, and longest paths through shared subtrees checked |
| Source locations | Imported type errors, unused invalid functions, missing dependency import sites, and runtime overflow locations checked |
| Replay | Moved package/renamed entry, changed or missing dependencies, manifests, duplicate keys, unknown fields, old versions, and incomplete stops checked |
| Filesystem controls | Exact-case, regular-file, UTF-8, and byte limits checked on both platforms; symbolic-source rejection also checked on Linux |
| Four-file batch | Accepted 2, rejected 1, total 41 in 110 steps; empty input returns zeros in 7 steps |
| Windows/Linux example agreement | Identical typed-program identity, source manifest, steps and result; both exact executables replay their own records |
| Existing job subprocess gate | 3 cases passed |
| Python R0 harness | 14 tests passed |
| Fresh Lean package | Build, decoder controls, no-sorry policy, and focused axiom audit passed |
| Independent NanoDA | 8,655 declarations checked with no errors |
| Frozen finite value graph | 1 initial, 4 states, 10 transitions; unchanged digest |
| Existing execution controls | 30 rejected by the real Lean executable |
| Frozen resource graph | 1 initial, 8 states, 12 transitions; unchanged digest |
| R0 real workflows | Proof 3/3, discovery 2/2, worker 4/4 completed |
| Public links, trust inventory, and whitespace checks | Passed |

The complete fresh gate ran from `2026-09-07T06:27:11Z` to
`2026-09-07T06:36:16Z` and exited successfully.

All 29 Lean package files match the fresh stage
`.cache/r2-packages/lean.QtfrhF/` by SHA-256. Existing Lean
proposition-definition linter warnings remain. The unchanged Lean checks do not
verify the new package reader, scope resolution, source manifest, or Rust
evaluator. No independent review is claimed.

## Identities and local evidence

The four-file example's typed program has SHA-256
`f7d7d4957d134044011cdf8a97df969b4ae960868e570e88ec8c841d4a807396`.
Its entry source has SHA-256
`66180c261f4b81b381334006de39b46a773f8eff79933fec1d8b6731d023955b`.
The run manifests retain all four source hashes and byte lengths. The Windows
executable used for the demonstration has SHA-256
`f576c95439aa1a6e43f4ea800f626c34e1cf277790c5065fe6750c9510e32a3a`.

The independent check used Lean 4.33.1, Rust 1.94.0, exporter
`411dce7db58a3afc60ecab2d211acd1042b593dc`, and NanoDA
`05055695879dfebb6628a67da88ceca6cd6b0421`. It selected 2,354 module constants.
The unchanged dependency export has SHA-256
`7122b8b23698904920885f73813c038dc718accb8945c99ab2bf69bcd368094f`.
Retained provenance includes the constant-list and checker-configuration hashes.

Local evidence is intentionally ignored by version control:

- `.cache/r2-packages/full-gate.log` and `verify.sh`
- `.cache/r2-packages/windows-tests.log` and `windows-clippy.log`
- `.cache/r2-packages/source-hashes.json` and `lean-source-hashes.json`
- `.cache/r2-packages/windows-typecheck.json`, `windows-batch.json`,
  `windows-empty.json`, and their replay records
- `.cache/r2-packages/linux-batch.json` and its replay record
- `.cache/r2-packages/exercise.py`, `linux-example.sh`, and `verify_identities.py`
- `.cache/r2-packages/checker-artifacts/run.evpxy7/`
- `target/r2-jobs/run-lvktn2ot/`
- `target/r0-baselines/run.HXEbjW/`

## Remaining work

Native source effects and supported job/Lean adapters, host cancellation and
process limits, recovery flows, user-defined safety invariants, project
commands/locks, and the complete real-input R2 exit gate remain outstanding.
RFC acceptance and independent publication review remain separate open gates.
The implementation and evidence remain uncommitted.
