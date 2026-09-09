# R1 finite execution completion evidence

Date: 2026-09-06. Status: R1/M1–M3 completed at finite scope. Validation ran on
working changes based on R0 commit `f966758`. [Plan.md](../../Plan.md) governs milestone
status. The [first increment](r1-unified-behavior-increment-2026-09-06.md) records
the earlier unified model and value work. RFCs [0015](../../rfcs/0015-unified-resource-bearing-behavior.md)
and [0016](../../rfcs/0016-decoded-finite-execution.md) remain Under review;
implementation evidence does not imply RFC acceptance or publication approval.

## Completed scope

| Milestone | Implemented result |
|---|---|
| M1 | One initialized control/authority behavior and completed-step relation, legacy leaf-pair equivalence, control projection, separate formation, conditional unified lifting, and constructed nested binary executions |
| M2 | Opt-in v2 known-capability and initial-authority maps; independently derived Lean maps; decoded formation, initializer, and actual synchronized transfer; an initialized abstract step through `ResourceDynamics.liftParallel` |
| M3 | Decoded finite paths with reachability, ownership uniqueness, vacancy preservation, and ownership-origin results; source-level receive then consume/retransfer; Rust/Lean finite value and resource graph comparisons |

V2 known capability types do not grant ownership. A later consume or retransfer
requires actual authority from an earlier receive. In the continuation fixture,
the receiver's consume action has no Boolean guard: ownership alone prevents
early use and reuse after consumption. The checked paths exercise synchronized,
local, and hidden steps. V2 completed execution blocks unmatched open transfers
and receives. Default v1 compilation, decoding, and canonical fixtures retain
their existing contract.

`nmlt elaborate --core-version v2` produces canonical core artifacts;
`nmlt trace` produces separately checked finite witnesses. The Lean executable
accepts `<core-v2.json> <source.nmlt> <path.json>`. The
[getting-started guide](../getting-started.md#finite-v2-execution) contains the
commands and migration rules. The interpreter and host effects remain R2 work.

## Validation

All checks below passed on the completed implementation:

| Gate | Observed result |
|---|---|
| Rust formatting and Clippy with warnings denied | Passed |
| `cargo test --workspace --all-targets` | 202 passed, 0 failed |
| Fresh staged `tools/check_metatheory.sh` | Clean Lean build (33 jobs), existing artifact/decoder controls, placeholder policy, and focused axiom audit passed |
| Fresh `tools/check_nanoda.sh` | 8,655 declarations checked with no errors |
| `make behavior-artifact` | Primary v1 artifact reproduced and transfer explored |
| `tools/check_finite_parity.sh` | Exact Rust/Lean graph equality: 1 initial state, 4 reachable states, 10 transitions |
| `tools/check_execution.sh` reproduction | Two v2 cores and three path files reproduced byte-for-byte |
| Actual Lean execution checks | Primary transfer: 1 step and 1 initial synchronized refinement application; consume and retransfer: 3 steps each |
| Execution rejection controls | 30/30 rejected through the real Lean executable |
| Resource graph comparison | Exact Rust/Lean equality: 1 initial state, 8 reachable states, 12 transitions |
| R0 harness and workflows | 14 tests; proof 3/3, discovery 2/2, worker 4/4 completed |
| Public-surface inventory/link check and `git diff --check` | Passed |

The clean Linux gate ran from `2026-09-06T11:19:35Z` to
`2026-09-06T11:26:51Z`. The package was copied without `.lake` into a fresh stage.
All 26 Lean source/test and package configuration files matched the workspace
by SHA-256 after the run. Existing proposition-definition linter warnings
remain; there were no build failures.

The twelve added execution and lifting axiom probes report only `propext`.
The package and focused behavioral allowlists are unchanged. The earlier nested
example still uses the already permitted `propext` and `Quot.sound`.

Rejection controls cover wrong initialization, unknown/missing/extra authority,
sentinel controls, invalid or absent actions, retained or lost transfer,
fabrication, hidden-world mutation, peer-control mutation, unmatched reception,
early and repeated consumption, use after transfer, invalid capability maps,
unsupported schemas, stale source/artifact identity, and failed abstract
refinement despite an otherwise valid concrete path. The resource oracle also
enumerates sentinel control indices and sentinel-capability ownership; these
must remain unreachable. These are finite regression comparisons, not a general
Rust/Lean compiler or evaluator correspondence theorem.

This completion increment received local implementation review. The earlier
increment's same-model agent reviews are recorded separately. No new independent
or authenticated cross-family publication review is claimed.

## Checker and artifact identity

| Input or result | Identity |
|---|---|
| Lean | `v4.33.1`, commit `819816b2e0a3bf405af45ae5c7af2491d8f5bee6` |
| lean4export | `411dce7db58a3afc60ecab2d211acd1042b593dc` |
| NanoDA | `05055695879dfebb6628a67da88ceca6cd6b0421` |
| Checker Rust compiler | `1.94.0` |
| Export size | 49,598,519 bytes; 959,988 lines |
| Enumerated NMLT module constants | 2,354 |
| Export SHA-256 | `7122b8b23698904920885f73813c038dc718accb8945c99ab2bf69bcd368094f` |
| Constant-list SHA-256 | `e7dbc9f69915728ef80036a209f9393f8627968706592ddd8c5acd5dfce6c9c1` |
| Checker-config SHA-256 | `556fa3f342120c2d37b59685902010080222c973895988ff106ffd2dcf695d56` |
| NanoDA Cargo.lock SHA-256 | `892346d3da3a6d728b34447e4cd8b6f063ac6acd9b514262000d85d58d17d9e4` |
| NanoDA binary SHA-256 | `37006cb9cb336c33ce72c37e73d1dd6eeb141b7f077eaebe820c4da329b3591f` |
| Sorted value graph SHA-256 | `bcc6f4e6832eaa2d25330e6c79e324b0497915f2c4d0891fa333bab81bee2da0` |
| Sorted resource graph SHA-256 | `e027879e69c8ff5ccae51cffacc7980f1493aeff427124546bd9b017436c53a2` |
| Primary v2 core SHA-256 | `1678d546eae7d956344633bf7011d7a497ba72fd9909acbfb14ae5c14c7fec56` |
| Continuation v2 core SHA-256 | `5e6d9113065f51d387fbca77b9a17405c3e656ca69c1f22fd3578f4dfa582aa2` |
| Primary path SHA-256 | `699fbab14c2153d59f21cfe5fb0fcd1639566e6f48a1da01b7eaeb4f873d3ae9` |
| Consume path SHA-256 | `530fc1ff582f31bac945eb8681f68c510dc67a732b2f8473df1672364698d511` |
| Retransfer path SHA-256 | `bb63114a83018d189c48ea9afcbbc82d2c5b06a666dc459c28684c287b8ab366` |

Local evidence is intentionally outside version control:

- `.cache/r1-completion/logs/full-gate.log`
- `.cache/r1-completion/lean.K9vEyb/`
- `.cache/r1-completion/checker-artifacts/run.loAqxh/`
- `target/r1-completion-rust.log`
- `target/r0-baselines/run.jzDRGo/`

Reproduce with `make reproduce` in the documented Rust/Lean environment.
CI includes the finite value and decoded execution gates.

## Limits and next milestone

The v2 path profile selects a binary composition of distinct named leaf systems.
Nested products have constructed Lean examples; nested source composition and
general n-ary execution are not delivered here. The initial refinement bridge
requires matching component/peer order and initial authority under an explicit
owner correspondence. It constructs an initialized abstract synchronization;
it does not transport every supplied path or support arbitrary owner renaming.

NanoDA checks exported package declarations. The compiled Lean JSON decoder and
decision procedures remain a runtime trust boundary, and accepted invocations
do not produce separately exported proof artifacts. Source and artifact hashes
identify exact bytes; they do not establish source translation correctness.
Rust exploration and path generation still report `assurance: none`.

R1 does not supply a general interpreter, host-runtime correspondence, liveness,
or user-defined behavioral property checking. R2 is next and has not started.
RFC acceptance, a reviewable public PR stack, and the independent publication
review requirements remain separate outstanding gates.
