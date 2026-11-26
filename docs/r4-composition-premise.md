# R4 first increment: a composition premise control

This is a fixed two-case experiment about the static `ResourceBehavior.liftParallel`
theorem. It uses the existing sender, receiver, and sender refinement from
`NMLT.Examples.ResourceWorldTransfer`; it adds no source syntax or new semantics.
The selected question is whether the theorem's wiring-equivalence premise can
be dropped while keeping the component refinement and the product-formation
conditions for each side.

The R4/R5 review found existing prerequisites: the
[R0 discovery calibration](r0-baseline-contracts.md#contract-b-conjecture-counterexample-revised-statement),
[R2 finite invariant and counterexample checks](r2-safety-invariants.md), and
[R3 bound Lean tasks, revisions, and independent rechecks](r3-lean-tasks.md).
R3's source-import discovery is a build feature. The R4 gap is the experiment
that fixes a mathematical question and records checked outcomes. This first
increment supplies one such premise control using the current Lean definitions.

| Case | Connections | Checked outcome |
|---|---|---|
| Known positive control | Both products connect `send` to `receive` | `matchedWiring_lifts` reconstructs the existing conditional lift. |
| Weakened premise | The concrete product connects `send` to `receive`; the abstract product has no connection | `disconnectedWiring_blocksLift` proves there is no composite `ResourceWeakRefinement`, regardless of state map. |

The Lean source is
[`NMLT.Counterexamples.WiringPremise`](../mechanization/lean/NMLT/Counterexamples/WiringPremise.lean).
`wiringDiffers` identifies the removed premise. `disconnectedComposable`
checks that the abstract product remains formed. The existing
`senderReceiverComposable` checks the concrete product, and
`senderRefinement` is the retained component refinement.
`concreteInitialAndEnabled` supplies an admitted initial state and an enabled
synchronized step, so the countermodel does not rely on an empty behavior.
`abstractInitialAndEnabled` supplies an admitted initial state and an isolated
sender step on the disconnected abstract side; it is not a deadlocked model.
The negative theorem shows that the concrete synchronized step has no abstract
action match.
This is a counterexample to the proposed weakened lifting statement for this
instance. It does not show that wiring equivalence is required for every
possible composition theorem.

Run the focused check from `mechanization/lean` with the pinned Lean toolchain:

```sh
lake build NMLT.Counterexamples.WiringPremise
```

The two cases are development controls, not a held-out R5 evaluation corpus or
a novelty claim. The earlier R0 addition exercise remains a separate frozen
known-result calibration. Next R4 work should freeze a wider premise family,
record every attempted subset including unknowns, and compare deterministic
analysis with ordinary Lean automation and agent suggestions. Before R5
comparison, select a separate evaluation set and budgets; recheck every claimed
success and report the actual case denominator.

Validation used Lean 4.33.1 on `DESKTOP-3SHE0AI`. The focused checks ran on
2026-09-30; the complete run finished on 2026-10-01 (UTC):

| Check | Result |
|---|---|
| `make fmt` | Passed, exit 0; no additional source changes. |
| `make reproduce` | Passed, exit 0; every default target completed. |
| `lake build NMLT.Counterexamples.WiringPremise` | Passed. |
| `lake build NMLT` and the default `lake build` | Passed. |
| `PYTHON=python bash tools/check_metatheory.sh` | Passed, including the seven new axiom probes, artifact controls, and no-sorry check. |
| `bash tools/check_nanoda.sh mechanization/lean NMLT` | Passed: 9,013 declarations checked without errors; the file exporter matched the pinned CLI on eight roots. |
| `python tools/check_public_surface.py` and `git diff --check` | Passed. |

The independent check used the repository's pinned exporter
`411dce7db58a3afc60ecab2d211acd1042b593dc` and NanoDA
`05055695879dfebb6628a67da88ceca6cd6b0421`, built with Rust 1.94.0.
The export SHA-256 was
`82433eb465ea107420bc75400e75049964578697323f310cad28f15616a36646`.
The local logs are `target/r4-metatheory.log` and `target/r4-nanoda-final.log`;
the exact successful checker inputs are retained in
`target/r4-nanoda/run.BarxY8/` (ignored build artifacts).

The complete run started at `2026-09-30T22:47:17Z` and finished at
`2026-10-01T00:09:03Z`, including formatting (1 hour 21 minutes 46 seconds).
It used GNU Make 4.4.1 through MSYS2, Python 3.13.12, and Rust 1.94.0.
The Windows invocation set `SHELL=bash.exe`, `PYTHON=python`, and
`R2_LEAN_BIN` to the direct pinned `lean.exe`. Both Cargo development and test
profiles used `OPT_LEVEL=1` and `DEBUG=0`; no check or timeout was removed.

| Complete-gate group | Recorded result |
|---|---|
| Rust formatting, workspace check, Clippy, and tests | Passed; 387 tests across 49 test binaries, zero failures. |
| Python baseline harness | 14 tests passed. |
| Behavior artifact, public surface, Lean metatheory, and axiom audit | Passed, including the seven new theorem probes. |
| Independent NanoDA check | 9,013 declarations, no errors; same export digest as the focused run. |
| Finite value parity | 1 initial state, 4 reachable states, 10 transitions matched. |
| V2 execution | Three fixtures, 30 rejection controls, and parity of 1 initial state, 8 reachable states, and 12 transitions passed. |
| R0 workflows | Proof: 3 frozen tasks; discovery: 2; worker: 4. All completed. |
| R2 workflows | Runtime, asynchronous/source jobs, project loops, Lean adapters, and proof terms passed; all 15 invariant rejection controls passed. |
| R3 workflows | All seven runners passed: bound tasks, imports, exports, dependencies, inspection, Lake/LSP, and project jobs. Their summaries record 19 accepted proof cases, 16 fresh rechecks, and 72 expected rejection controls. |

The default gate excludes the optional Mathlib extension; it was not run here.
The complete logs are `target/r4-format.log` and `target/r4-reproduce.log`;
`target/r4-reproduce-environment.txt` records versions, exit codes, and UTC
timestamps. The exact invocation is retained in
`target/run-r4-verification.sh`, and this run's independent checker artifacts
are in `target/r4-reproduce-nanoda/run.GMCQ13/`. All are ignored local build
artifacts. There were no unexpected gate failures or unresolved environment
blockers. After recording these results, formatting, public-surface, and diff
checks were repeated on the final tree.

A separate read-only AI review on 2026-09-30 found no blocking proof or
code issues. It checked retained hypotheses, non-vacuity on both sides, the
arbitrary state-map claim, axiom use, and the documentation limits. Its minor
wording finding was corrected to name the concrete synchronized step explicitly.
No new RFC is required under `CONTRIBUTING.md`: this adds controls over existing
definitions and changes no syntax, typing, semantics, trusted components,
artifact interpretation, compatibility, or axiom allowlist.

The AI review is not independent human/domain evidence. Under the maintainer
policy revised on 2026-10-03, external feedback is optional; R4/R5 completion
instead requires the executed solo gates in [Plan.md](../Plan.md). This original
two-case control alone does not satisfy those broader gates.
