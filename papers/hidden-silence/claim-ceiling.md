# Paper 1 claim ceiling

**Working title:** Resource-Aware Weak Refinement for an Open Programming
Language

**Status:** post-pivot draft, bounded by the checked Lean modules and source
fixture on the language-math pivot branch

## Claims allowed

The paper may claim that:

1. NMLT defines one resource-bearing behavior object in Lean with actual
   states, steps, observations, action visibility, typed boundary data,
   capability ownership, grades, and rely/guarantee profiles.
2. NMLT.Behavior.ResourceBehavior.liftParallel is a checked conditional
   binary-composition theorem over the actual product step relation.
3. Product formation requires compatible directions and payloads, no hidden
   connected boundary, whole-wiring preservation and reflection, disjoint
   affine ownership, exact transfer/receive matching, and rely discharge.
4. Resource refinement preserves authority effects exactly, is pointwise
   grade-monotone, and treats hidden stutter as resource-free.
5. NMLT.Examples.VisibleResourceSync.visibleResourceSync_lifts instantiates
   the theorem for the primary source fixture and its canonical
   behavior-core-v1 artifact.
6. The seven controls in
   NMLT.Counterexamples.ResourceBehaviorControls witness failures caused by
   independently omitting the stated premises.
7. The Rust explorer reproduces the fixture's synchronized state change,
   one-time authority transfer, and total work grade three as non-authoritative
   operational inspection.

## Required citations

Every theorem-level presentation must identify:

- mechanization/lean/NMLT/Behavior/ResourceBehavior.lean
- mechanization/lean/NMLT/Examples/VisibleResourceSync.lean
- examples/pivot/visible_resource_sync.nmlt
- examples/pivot/visible_resource_sync.behavior-core-v1.json
- mechanization/lean/AXIOMS.md

## Non-claims

The paper must not claim:

- fairness, divergence, infinite-trace, or liveness transport;
- a fairness counterexample from HiddenDivergence;
- arbitrary composition, arbitrary grade algebras, or infinite state;
- a verified Rust-to-Lean compiler or source-semantics correspondence proof;
- proof, model-check, evidence, or certificate authority for the Rust
  explorer;
- that deterministic artifacts alone verify compilation;
- that the quarantined WeakResourceCongruence experiment was a theorem over
  the active semantics.

HiddenDivergence remains future work until behavior-indexed fairness is
defined over the unified semantics.

## Trusted base

- Lean 4.30.0, pinned by mechanization/lean/lean-toolchain
- the active hand-written Lean definitions and theorem witnesses
- Lean standard propext for the two composition declarations
- documented Quot.sound only in the retained typed-elaboration metatheory

The repository gate must reject sorry, sorryAx, admit, project axioms, and
unchecked native decisions.
