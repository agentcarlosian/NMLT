# R1 first implementation increment

Date: 2026-09-06. Status: implemented and checked as working changes on R0
commit `f966758`. R1 remains **In progress**; this record is not its exit gate.
[Plan.md](../../Plan.md) governs remaining work.
[RFC 0015](../../rfcs/0015-unified-resource-bearing-behavior.md) is Under review.

## Implemented scope

- `ResourceDynamics.Behavior` combines control and authority in its state,
  initialization, observations, and completed steps. Deferred effects retain
  leaf actors through binary composition; isolated actions retain direction,
  payload, and hiding. A completed inner synchronization executes once and
  cannot participate as an atomic actor in another rendezvous.
- `legacy_step_iff` proves exact agreement with the old dynamic relation for
  leaf pairs; `step_projects` gives the control-product projection.
  `liftParallel` has explicit wiring, connected-visibility, and synchronized
  effect premises. Product formation is a separate condition.
- `Path` and `Reachable` use the same completed-step relation. Vacancy persists
  along paths; reachable no-fabrication assumes vacancy in every admitted
  initial state. Ownership uniqueness is structural in the functional world.
- The main nested example has explicit formation and initialization witnesses,
  an outer synchronization completing an inner open transfer, exact-once
  authority movement, and a subsequent hidden monitor step. The enclosed
  inner-synchronization example separately proves a standalone step.
- Rust exploration supports Bool, Unit, and declared finite enum values,
  including typed equality, Boolean negation, closed initializers, and updates
  evaluated against the pre-state. Invalid terms are rejected even in disabled
  actions. `EvalState.values` now contains `EvalValue` rather than `bool`, a
  pre-alpha library API change. Exploration still reports `assurance: none`.

The finite comparison fixture is
[`finite_value_cycle.nmlt`](../../examples/pivot/finite_value_cycle.nmlt).
The gate regenerates its canonical v1 artifact and compares both implementations'
complete initial and reachable state/transition sets, including self-loops.
The Lean oracle decides the decoded behavior's actual `init` and `step`
relations. Truncation, duplicate rows, and coverage changes fail the gate.

This fixture is closed and resource-free: 1 initial state, 4 reachable states,
and 10 transitions within a 16-state exploration bound. It is a finite
differential regression check, not a general Rust/Lean correspondence theorem.

## Validation

All checks below passed against this increment on 2026-09-06:

| Gate | Result |
|---|---|
| Rust formatting, workspace check, Clippy with warnings denied | Passed |
| `cargo test --workspace --all-targets` | 196 passed, 0 failed |
| Fresh staged `tools/check_metatheory.sh` | Clean Lean build, artifact/decoder controls, source placeholder policy, focused axiom audit passed |
| Fresh `tools/check_nanoda.sh` export and independent check | 7,431 declarations checked without errors |
| `make behavior-artifact` | Primary v1 artifact reproduced; permit transfer explored |
| `tools/check_finite_parity.sh` | Rust/Lean graph equality: 1 initial, 4 states, 10 transitions |
| `make r0-baseline-tests` | 14 passed |
| `make r0-baselines` | Proof 3/3, discovery 2/2, worker 4/4 completed |
| Public-surface inventory/link check and `git diff --check` | Passed |

The full Linux gate ran from `2026-09-06T09:57:39Z` to
`2026-09-06T10:03:02Z`. The Lean package was copied without `.lake` into a fresh
stage. All 21 Lean source/test and package configuration files in that stage
matched the workspace by SHA-256 after the run. Existing proposition-definition
linter warnings remain; there were no build failures. The R1 focused probes
require only the already permitted `propext` and `Quot.sound` axioms; the package
and focused allowlists were not expanded.

Independent agent reviews checked the Lean model, nested witnesses, differential
oracle, and public claims. These were same-model development reviews, not an
external or cross-family publication review.

### Checker identity and retained evidence

| Input or result | Identity |
|---|---|
| Lean | `v4.33.1`, commit `819816b2e0a3bf405af45ae5c7af2491d8f5bee6` |
| lean4export | `411dce7db58a3afc60ecab2d211acd1042b593dc` |
| NanoDA | `05055695879dfebb6628a67da88ceca6cd6b0421` |
| Checker Rust compiler | `1.94.0` |
| Export size | 34,190,151 bytes; 658,578 lines |
| Enumerated NMLT module constants | 2,209 |
| Export SHA-256 | `d2b0dedcc4acaa489ec598ef883785dfea3097239231a2cb8c5e0e315a55aed6` |
| Constant-list SHA-256 | `02aa2626bc4dc7b1979de3dd95dc7259d5db336657f10380ee4904fcb7b216ca` |
| Checker-config SHA-256 | `556fa3f342120c2d37b59685902010080222c973895988ff106ffd2dcf695d56` |
| Sorted finite graph SHA-256 | `bcc6f4e6832eaa2d25330e6c79e324b0497915f2c4d0891fa333bab81bee2da0` |
| Finite fixture artifact SHA-256 | `687cef48e19b8fec83fd0ebe4a38b8943a520336c7d7e489097ef6b3e9122908` |
| `ResourceDynamics.lean` SHA-256 | `76b2450b2c6b0c1de4bb1ff9eb6d6d8b50e87b16b68d5045ba71660fb1303496` |
| `NestedResourceDynamics.lean` SHA-256 | `2f53b36e82cd13e2aad7c7c9f5cc7f6cd27186cecaab4a2262f36dd063bdf07b` |

Local run artifacts, intentionally outside version control:

- `.cache/r1-verification/logs/full-gate.log`
- `.cache/r1-verification/checker-artifacts/run.g1DTSf/`
- `target/r1-rust-gate.log`
- `target/r0-baselines/run.M1cE1J/`

Reproduce with `make reproduce` in the documented Rust/Lean environment. CI and
that target now include the finite comparison. The independent checker retains
the existing host, toolchain, specification, and delivery trust boundaries.

## Remaining R1 work

1. Settle and implement `behavior-core-v2`, including initial authority,
   dynamic transition data, migration, and unsupported-version behavior.
2. Construct decoded initial execution and exact authority movement in Lean,
   with meaningful invalid-world and claimed-step controls.
3. Distinguish typed known capabilities from initial ownership in the source
   and compiler so a received capability can be consumed or retransferred by
   a later action.
4. Check finite receive-then-use and receive-then-transfer paths from decoded
   initial states, and extend Rust/Lean comparisons to those resource paths.

The v1 artifact certificate still supplies conditional witnesses. This increment
does not prove source translation, general runtime correspondence, arbitrary
associativity or multiparty rendezvous, fairness, or liveness. It does not deliver
the R2 interpreter or host adapters.
