# Axiom audit — Paper 1 theorems

**Date:** 2026-08-27  
**Toolchain:** Lean 4.30.0 (`mechanization/lean/lean-toolchain`)  
**Command:** `lake build` from `mechanization/lean`  
**Source:** `#print axioms` info lines emitted while compiling the modules

## Counterexample (T1–T3)

| Declaration | Axioms |
|---|---|
| `NMLT.Counterexamples.CompositionCongruence.senderRefinement` | none |
| `NMLT.Counterexamples.CompositionCongruence.concreteSynchronization` | none |
| `NMLT.Counterexamples.CompositionCongruence.compositeRefinementImpossible` | `[propext]` |
| `NMLT.Counterexamples.CompositionCongruence.noCompositeRefinement` | `[propext]` |
| `NMLT.Counterexamples.CompositionCongruence.abstractSyncImpossible` | none |
| `NMLT.Counterexamples.CompositionCongruence.visibleClassification_fails_on_hidden_ping_wire` | none |

## Strong / exact-action repair (T4)

| Declaration | Axioms |
|---|---|
| `StrongRefinement.preservesInputReceptive` | none |
| `StrongRefinement.outputCanSynchronize` | none |
| `StrongRefinement.rightOutputCanSynchronize` | none |
| `parallelInputReceptive` | none |
| `StrongRefinement.preservesComposableLeft` | none |
| `WiringEquivalent.reflectRightIsolation` | none |
| `StrongRefinement.preservesComposableLeftUnderWiring` | none |
| `StrongRefinement.parallelStepCongruence` | none |
| **`StrongRefinement.compositionCongruence`** | **none** |
| `StrongRefinement.identity` | none |
| `StrongRefinement.compose` | none |
| `StrongRefinement.parallelStepCongruenceRight` | none |
| `StrongRefinement.liftParallelRight` | none |
| `StrongRefinement.preservesComposableRightUnderWiring` | none |
| `StrongRefinement.twoSidedCompositionCongruence` | none |
| `StrongRefinement.mapReachable` | none |
| `StrongRefinement.transportInvariant` | none |
| `Examples.positiveCompositionCongruence` | none |
| `Examples.positiveTwoSidedCompositionCongruence` | none |
| `Examples.positiveProductInvariantTransport` | none |
| `Examples.positiveConcreteSynchronization` | none |
| `Examples.peerOnlyWithEmptyWiring` | none |
| `Examples.brokenWiringBlocksPeerOnly` | none |
| `Examples.brokenWiringNotEquivalent` | none |


## Weak conditional interface

| Declaration | Axioms |
|---|---|
| `pingReceive_violates_noHiddenBoundary` | none |
| `visiblePing_breaks_senderRefinement` | none |
| `visibleClassification_fails_default_lift` | none |
| `pingReceive_wiring_id` | none |
| `isolation_of_wiring_injective` | (lemma; no `#print axioms` line) |
| `connected_not_hidden` | (lemma) |
| **`weakConditionalCongruence_safety`** | **`[propext]`** |
| `weakConditionalCongruence_safety_nonempty` | `[propext]` |
| `VisibleSync.visibleSync_productRefinement` | `[propext]` |
| `EmptyWiringTau.emptyWiring_productRefinement` | `[propext]` |
| `CollidedAbstractWire.isolation_fails` | none |
| `CollidedAbstractWire.collided_noProductRefinement` | none |
| `CollidedAbstractWire.isolation_necessary_for_default_lift` | none |
| `ExtraAbstractWire.wiring_necessary_for_default_lift` | none |

T5 is a small-model safety lift, not residual C1 (resources/grades/fairness /
OpenComposition). Necessity lemmas remain axiom-free.

## Finite observation-trace inclusion (Corollary)

| Declaration | Axioms |
|---|---|
| `weakRefines_finite_observation_trace_inclusion` | none |
| `weakRefines_finite_observation_trace_inclusion_from_init` | none |
| `weakRefines_hidden_preserves_observe` | none |

Finite traces only. Not LTL, infinite words, fairness, or liveness.

## Notes

- No `sorry`, `sorryAx`, or project-defined axioms on these declarations.
- `propext` on T3 is Lean's standard propositional extensionality.
- Full workspace build also compiles Aeneas/Mathlib-backed modules; those are
  outside Paper 1's TCB for T1–T4.
