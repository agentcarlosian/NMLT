# M11 contract-sound label-aware refinement research

Date: 2026-07-19

Scope: M11-001b canonical finite contracts, payload identity, variance, and
identity/composition laws for open refinement.

This note freezes the first contract-refinement profile before implementation.
It distinguishes material surfaced in Carlosian's local archive from primary
sources consulted separately. Neither category proves an NMLT implementation
or theorem.

## Research question

What is the smallest finite open-refinement relation that preserves explicit
labels and payload types, weakens assumptions, strengthens guarantees, and
composes without admitting payload substitution or unsupported circular
assume/guarantee reasoning?

## In the local archive

Four focused lexical searches covered interface/alternating refinement,
assume/guarantee variance, typed payload labels, and circular discharge.

The archive surfaced
[Interface Simulation Distances](https://arxiv.org/abs/1210.2450), whose
alternating-refinement distance is nonexpansive under parallel composition;
[Faster Algorithms for Alternating Refinement Relations](https://arxiv.org/abs/1201.4449),
which treats alternating refinement as a game relation; and
[Verifying Compositional Refinement of Assume/Guarantee Contracts](https://arxiv.org/abs/2103.13743),
which gives a domain-specific compositional contract-refinement result for
linear dynamical systems.

The archive returned no useful match for typed payload substitution and no
useful match for circular contract discharge. Those are archive coverage gaps,
not evidence that the theories do not exist. The live collector returned no
additional arXiv result for the four focused queries, so primary pages and
papers were consulted separately rather than treating that failure as negative
evidence.

## Current primary sources consulted separately

The original
[Interface Automata](https://research-explorer.ista.ac.at/record/4622)
formulation characterizes refinement as weaker input assumptions and stronger
output guarantees. Its alternating game semantics is broader than the finite
set-inclusion profile adopted below; M11-001b does not claim to implement
interface-automata games.

[Contracts for System Design](https://hal.science/hal-00757488) defines
contract refinement by implementation containment and reverse environment
containment. For saturated assume/guarantee contracts this yields the familiar
variance rule: the refining assumption contains the abstract assumption while
the refining guarantee is contained in the abstract guarantee.

[From Relational Interfaces to Assume-Guarantee](https://www2.eecs.berkeley.edu/Pubs/TechRpts/2014/EECS-2014-21.pdf)
states the same finite-set direction explicitly and also requires alphabet
equalization before contracts over different variables are compared. This
supports NMLT's conservative decision to require exact payload-type identity
before predicate inclusion is meaningful.

[Modal Interface Automata](https://arxiv.org/abs/1306.3050) retains implicit
input permission, explicit output obligations, and compositional operators.
It reinforces the separation between input and output refinement, but does not
justify silently importing modal must/may transitions into this milestone.

Circular assume/guarantee rules are not free consequences of ordinary
contracts. Viswanathan and Viswanathan's
[Foundations for Circular Compositional Reasoning](https://experts.illinois.edu/en/publications/foundations-for-circular-compositional-reasoning/)
gives sound circular rules only after defining least/greatest-fixed-point
semantics for the relevant properties. The noncircular rule documented in
[Assume-Guarantee Reasoning for Deadlock](https://insights.sei.cmu.edu/documents/2116/2006_004_001_14717.pdf)
instead discharges a separately established assumption. M11-001b keeps the
existing conservative noncircular boundary and does not pretend that mutual
predicate links are self-justifying.

## Frozen M11-001b profile

1. A payload type is a finite nominal enumeration with a nonempty ASCII name
   and a nonempty, duplicate-free set of nonempty ASCII variants. Its canonical
   identity is SHA-256 over a versioned domain, the nominal name, and the
   variants in UTF-8 byte order with explicit lengths. Reordering a declaration
   does not change identity; renaming the type or changing a variant does.
2. A payload predicate is a canonical finite subset of one exact payload type.
   Its identity binds the payload-type identity and the sorted accepted
   variants. Unknown variants and duplicate inputs are rejected rather than
   repaired.
3. Every boundary action has exactly one payload type. Every input contract
   clause is an assumption on its input action; every output clause is a
   guarantee on its output action. Internal actions have neither payload types
   nor contract clauses. Missing, extra, polarity-mismatched, or type-mismatched
   clauses make an open system invalid.
4. A contract link names a consumer input label and provider output label. It
   is valid only when those actions are the endpoints of the same explicit
   synchronous connection, their channel and payload identities agree, and the
   provider guarantee set is included in the consumer assumption set.
5. The finite composition checker retains M11-001a's strict noncircular rule:
   a guarantee used for discharge must come from a component with no
   assumptions. Every assumption must be discharged exactly once. Circular or
   otherwise conditional discharge remains rejected.
6. A label-aware open refinement contains the existing finite state/action
   forward simulation plus a total injective map of visible boundary labels.
   The visible map covers the complete abstract boundary, preserves polarity
   and channel, and requires exact payload-type identity. Boundary actions may
   be renamed but not hidden or aliased. Internal actions retain the existing
   exact/hidden safety treatment and do not carry contracts.
7. For each mapped input, the abstract assumption predicate is a subset of the
   concrete predicate. The concrete component therefore accepts at least every
   payload admitted by the abstract contract: assumptions are contravariant.
8. For each mapped output, the concrete guarantee predicate is a subset of the
   abstract predicate. The concrete component therefore emits no payload
   forbidden by the abstract contract: guarantees are covariant.
9. The Lean model proves reflexivity and transitivity/composition for this
   relation without new axioms. Its finite predicates are `Finset`s; the Rust
   canonical byte identity is an executable representation obligation, not a
   theorem that Lean's `Finset` encoding and Rust bytes correspond.
10. M11-001b does not prove product congruence for the new relation. M11-001c
    owns two-sided lifting, composite contract soundness, invariant transport,
    and Rust/Lean correspondence.

## Required controls

- assumption strengthening in the concrete contract is rejected;
- guarantee weakening in the concrete contract is rejected;
- payload-type substitution is rejected even when variant spellings overlap;
- noninjective or incomplete visible-label maps are rejected;
- a predicate mentioning a value outside its payload type is rejected;
- a contract link not backed by the named connection is rejected;
- circular discharge remains rejected; and
- Lean retains nonidentity positive refinement plus failed-premise controls for
  payload identity and both variance directions.

## Evidence boundary

The Rust checker establishes only the implemented finite set and graph checks.
The Lean theorems establish only the encoded abstract identity and composition
laws. Evidence must bind their exact sources, theorem/control handles,
toolchains, checkers, canonical identity domains, limitations, and the absence
of a Rust/Lean correspondence theorem. No payload subtyping, semantic trace
contract satisfaction, fairness, liveness, divergence, capability, grade, or
resource claim follows from this slice.
