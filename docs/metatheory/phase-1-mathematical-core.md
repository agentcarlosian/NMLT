# Phase 1 mathematical-core decisions

- Scope: `NMLT-P1-102` through `NMLT-P1-106`
- Decision date: 2026-07-18
- Status: candidate rules under review; one conjecture mechanically refuted

## Outcome

The Phase-1 mathematical core now has concrete candidates rather than shape
sketches:

| Item | Decision or result | Normative artifact | Checked evidence |
|---|---|---|---|
| P1-102 | State is unrestricted finite record data; actions construct post-state by simultaneous explicit updates and emit frame equalities | [RFC 0005](../../rfcs/0005-state-and-action-typing.md) | Rules and proof obligations specified; preservation/frame proofs open |
| P1-103 | V1 capabilities are explicit affine tokens: no contraction, explicit discard, exact branch contexts, disjoint parallel partition | [RFC 0006](../../rfcs/0006-explicit-affine-capabilities.md) | No-duplication/provenance proofs open |
| P1-104 | Events and observations are distinct; identity, observation-silent, and refinement-hidden steps are distinct; generic temporal transport excludes `next` | [RFC 0007](../../rfcs/0007-observation-and-stuttering.md) | Stutter equivalence and temporal transport proofs open |
| P1-105 | Lean 4.30.0 is the first mechanization environment; proof sources live under `mechanization/lean/` | [RFC 0008](../../rfcs/0008-mechanization-and-compositional-refinement.md) | Clean `lake build` succeeds with the pinned toolchain |
| P1-106 | Unconditional weak-refinement congruence under synchronization is false | [RFC 0008](../../rfcs/0008-mechanization-and-compositional-refinement.md) | Lean checks the standalone refinement and the composite counterexample |

The status distinction matters. P1-106 has a kernel-checked negative result for
the encoded candidate. P1-102 through P1-104 are precise conjectural rules,
not checked soundness results. P1-105 is a tooling/layout decision, not a
semantic theorem.

Post-Phase-1 update (2026-07-19): M11 retains P1-106 and additionally proves a
restricted exact-action open-composition theorem. That later positive result
does not retroactively validate the unconditional rule or the full weak,
resource-aware conjecture.

## Cross-RFC kernel

The candidate behavior signature is:

```text
Behavior<S,I,E,V,G>
```

with unrestricted state `S`, input `I`, emitted step event `E`, observed state
value `V`, and grade algebra `G`. An action derivation has the shape:

```text
Gamma; Sigma; input:I; Delta_in
  |- c : Body<E,G> => <W,g,Delta_out>
```

and elaborates to a relation:

```text
State(Sigma) x I x CapStore(Delta_in)
  <-> State(Sigma) x E x |G| x CapStore(Delta_out)
```

The relation is constrained by four independent invariants:

1. every post-state field outside `W` equals its pre-state value;
2. every capability identity is live in at most one context and has at most one
   live affine descendant;
3. identity stutter has state equality, no event/action, grade `epsilon`, and no
   capability change;
4. a refinement-hidden transition maps to abstract state equality and uses no
   connected boundary port.

This separation prevents three invalid inferences:

- an unchanged observation does not prove that no effect occurred;
- consumed authority does not prove that an external provider committed;
- component-local hiddenness does not imply contextual hiddenness.

## The congruence failure

The rejected rule was:

```text
C refines A
-----------------
C || D refines A || D
```

with no constraint connecting refinement hiding to composition ports. The Lean
counterexample gives `C` a hidden `ping` that is a local state identity and
gives `D` a synchronized `receive` that changes an observed bit. `C` refines
the step-free `A` alone. In the product, the hidden `ping` causes an observable
peer transition that `A || D` cannot match.

The repaired conjecture requires at minimum:

```text
hidden(l) => l is not connected at the composition boundary
every concrete wire is preserved, every abstract wire is reflected, and both
connection endpoint domains agree
```

plus input/rely compatibility, event-map commutation, capability partition,
grade homomorphism, and separate fairness/divergence conditions for liveness.
Pointwise preservation only over mapped concrete labels is insufficient when
an extra abstract wire can block a peer-only step. RFC 0008 records both the
checked exact-action theorem and the remaining broader case-split proof plan.

## Mechanization boundary

The original Phase-1 slice checks:

- the minimal labelled-transition and one-step simulation definitions;
- synchronized product semantics for the counterexample;
- the standalone sender refinement;
- existence of the concrete synchronization;
- nonexistence of an observation-preserving composite refinement.

Later slices now encode typed actions and capabilities, infinite traces,
directed refinement laws, a coinductive safety seed, grades, and M11's
exact-action open-system theorem. M11 proves structural one-sided product
congruence for state-surjective strong refinement with equivalent whole wiring,
and separately proves composability preservation and product input
receptiveness. It still does not prove the full weak rule, fairness, divergence,
resource preservation through composition, or correspondence with Rust.
Evidence and public claims must retain those gaps.

## Next proof obligations

The next sequence is deliberately dependency ordered:

1. mechanize finite maps/records and simultaneous update elaboration;
2. prove state/action preservation and frame soundness;
3. mechanize exact affine contexts and prove no duplication;
4. add ports and prove repaired safety congruence;
5. define infinite observation words and stutter equivalence;
6. prove temporal transport for the no-`next` observation fragment;
7. add divergence and fairness structures before any liveness theorem;
8. prove a correspondence theorem or translation validation result before
   applying Lean results to compiler-produced IR.

Every stage retains a negative control. A desired theorem that fails becomes a
checked counterexample and a revision to the semantics, not an axiom.
