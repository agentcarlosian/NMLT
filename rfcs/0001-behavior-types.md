# RFC 0001: Behavior types

- Status: Draft
- Authors: NMLT project
- Created: 2026-07-18

## Summary

Introduce behavior types as the semantic object connecting state, actions,
observations, traces, temporal properties, composition, and refinement.

## Motivation

Value types describe snapshots but do not classify how a system may evolve.
TLA+-style behavior semantics handles change well, while dependent and modal
type theories make additional structure available to the type checker. NMLT
needs one typed object that prevents properties and refinements from silently
mixing incompatible systems.

## Goals

- Index temporal propositions by their behavior.
- Make observations and stuttering policy explicit.
- Support executable finite exploration without defining semantics by a model
  checker implementation.
- Provide a foundation for open composition and refinement.

## Non-goals

- Full probability or continuous dynamics in the initial design.
- Unrestricted general recursion.
- A commitment to specific surface action syntax.

## Candidate core

```text
Behavior S E O
init       : Predicate S
step       : Relation (S × E) (S × O)
observe    : S → O
stutter    : Relation S S
```

This shape is illustrative. In particular, outputs, nondeterminism, grades, and
open ports may require a different categorical representation.

## Property indexing

```text
Property B
always     : Predicate (State B) → Property B
eventually : Predicate (State B) → Property B
```

A property over `B₁` cannot be applied to `B₂` without an explicit mapping or
refinement.

## Negative controls

- Mixing a state predicate from a different behavior must fail.
- Hiding an observable variable without changing the observation contract must
  fail.
- Treating internal non-stuttering change as stuttering must fail.
- Composing incompatible event ports must fail.
- A finite-prefix check must not produce unbounded liveness evidence.

## Open questions

- Whether behavior types are primitive, encoded through guarded types, or
  elaborated to a smaller coalgebraic core.
- Whether stuttering is built into every behavior or introduced by refinement.
- How fairness assumptions compose.
- Which conditions make refinement congruent under open-system composition.

## Evidence plan

Mechanize candidate definitions and prove basic typing, productivity,
stuttering, and refinement properties before acceptance.
