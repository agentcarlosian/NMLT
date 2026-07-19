# Research synthesis: Phase 1 mathematical core

Historical scope note: this synthesis records the 2026-07-18 Phase-1 state.
M11 added a bounded exact-action open-composition theorem on 2026-07-19; see
[RFC 0008](../../rfcs/0008-mechanization-and-compositional-refinement.md) and
the [M11 research note](../research-notes/m11-open-system-refinement-2026-07-19.md).
Its older “current artifact” limitations below remain the audit record for the
earlier research pass, not the repository's latest inventory.

- Search date: 2026-07-18 (America/Chicago; collector timestamps crossed into
  2026-07-19 UTC)
- Focus: substructural action typing, observation/stuttering, compositional
  refinement, and a first mechanization environment
- Method: `search-the-archives` lexical collector over the local JSON archive,
  current Hugging Face Papers and arXiv, followed by primary papers and official
  project documentation

## Research question

What is the smallest formal state/action and capability discipline that can
support TLA-style stuttering refinement and synchronized open-system
composition, and which proof environment can falsify or establish its first
metatheorems?

The collector queries covered:

- `linear capability types state transition systems refinement`;
- `stuttering simulation compositional refinement observation`;
- `rely guarantee refinement congruence parallel composition`;
- `Lean mechanization labeled transition systems linear types`;
- broader recall queries for `linear types`, `TLA refinement`,
  `compositional verification`, and `proof assistant mechanization`;
- `multiparty session types liveness Rocq` and `behavioral types formal
  verification`.

## In the archive

No locally archived item closely matched the mathematical-core question. The
broad lexical searches returned mostly unrelated uses of “linear,”
“refinement,” and “verification.” No separate semantic archive retriever was
available to improve conceptual recall.

This absence is weak evidence about the archive and no evidence of novelty,
prior exposure, or endorsement. The archive is recent and broad; classic
programming-language and concurrency papers are especially likely to be
missing from its lexical index.

## New/current leads

### Substructural authority

- [Oxide: The Essence of Rust](https://arxiv.org/abs/1903.00982) gives a
  source-level, substructural account of ownership with syntactic progress and
  preservation. It supports using explicit contexts and context transition
  judgments, but its borrowing/lifetime system is far larger than NMLT's first
  provider capability.
- [RustBelt](https://doi.org/10.1145/3158154) gives a machine-checked semantic
  account of an ownership-based core extended by privileged libraries. Its key
  implication is that operation implementations and safe-extension conditions
  belong in the trusted boundary; a surface affine checker alone does not
  justify external-effect claims.
- [A graded dependent type system with a usage-aware
  semantics](https://arxiv.org/abs/2011.04070) derives resource properties,
  including a single-pointer property for linear resources, from a semantics
  that actually tracks usage. This argues against treating NMLT grades and
  capabilities as decorative annotations.

### Stuttering and observation

- Lamport's [Specifying
  Systems](https://lamport.azurewebsites.net/tla/book-02-03-04.pdf) makes
  invariance under adding or deleting stuttering steps central to TLA. It also
  distinguishes action formulas from stuttering-invariant temporal formulas.
- Peled and Wilke's
  [stutter-invariance characterization](https://doi.org/10.1016/S0020-0190(97)00133-6)
  connects the stutter-invariant LTL-expressible properties to formulas without
  the next-time operator. NMLT v1 therefore selects a no-`next` fragment for
  generic observation-refinement transport.
- Lamport's recent [A Science of Concurrent
  Programs](https://lamport.azurewebsites.net/tla/science.pdf) again treats
  lower-level implementation steps as abstract stuttering under a refinement
  map. NMLT strengthens that local condition with boundary noninterference
  because the composition counterexample shows state-map equality alone is not
  contextual silence.

### Composition and refinement

- The MIT [I/O Automata
  overview](https://groups.csail.mit.edu/tds/i-o-automata.html) explicitly
  separates input, output, and internal actions, requires input receptiveness,
  and reports compositional ordinary/fair trace notions. These are substantive
  premises, not consequences of an untyped product transition relation.
- Lynch, Segala, Vaandrager, and Weinberg's [Hybrid I/O Automata
  work](https://groups.csail.mit.edu/tds/papers/Lynch/LSVW.html) defines an
  implementation relation and proves composition respects it under stated
  compatibility conditions. This reinforces the repaired direction:
  interface-aware substitutivity, not unconditional local weak simulation.
- The current NMLT candidate was tested directly rather than accepted from
  analogy. The checked hidden-synchronization example refutes unconditional
  congruence and pinpoints the missing premise: a hidden component action must
  use no connected boundary port.

### Mechanization environment

- The official [Lean 4 system
  description](https://doi.org/10.1007/978-3-030-79876-5_37) describes Lean as
  both an extensible theorem prover and efficient functional language. That
  supports executable semantic definitions and checked proofs in one project.
- The official [Lean language
  reference](https://lean-lang.org/doc/reference/latest) and [Elan
  guide](https://lean-lang.org/doc/reference/latest/Build-Tools-and-Distribution/Managing-Toolchains-with-Elan/)
  support pinning exact project toolchains. Lean 4.30.0 was the latest stable
  release shown by the official repository on the search date and is pinned.
- [Formally Verified Liveness with Multiparty Session Types in
  Rocq](https://arxiv.org/abs/2605.23633) is a 2026 preprint found through live
  arXiv, not the local archive. Its roughly 14K-line Rocq development uses
  coinductive trees and `paco` for projection, operational correspondence,
  safety, and liveness. It is evidence that NMLT should not fold fairness and
  liveness into an untested safety simulation, and it keeps Rocq as a credible
  fallback if Lean's coinduction story becomes the dominant cost.
- [Behavioral Program Logic](https://arxiv.org/abs/1904.13338) was another live
  arXiv result. Its integration of trace-oriented behavioral specifications
  with deductive reasoning supports keeping behavior indices explicit, but it
  does not directly settle NMLT's capability or stuttering choices.

## Implication for NMLT

The evidence changed five design choices:

1. Use a small exact context discipline instead of adopting borrowing: v1
   capabilities are affine, noncopyable, and explicitly discarded.
2. Track capability use in the operational semantics and trusted operation
   boundary; types alone do not establish external commitment.
3. Split step events from state observations and retain intensional traces even
   when observation rendering collapses repetitions.
4. Restrict generic temporal transport to observation formulas without
   `next`; action, event, resource, and fairness claims have separate proof
   obligations.
5. Replace unconditional congruence with an interface-aware conjecture and keep
   the refuting example as a permanent mechanized negative control.

The defensible contribution remains a proposed integration and experimental
kernel. The searched sources contain strong precedents for each ingredient.
Novelty, usability, and soundness can only be evaluated after the repaired
rules are mechanized and compared on the canonical examples.

## Exact limitations

- Archive retrieval was lexical and returned no relevant local hit; conceptual
  matches may have been missed.
- Live source discovery was not a systematic literature review.
- RFCs 0005–0007 contain formal candidates and proof obligations, not proofs.
- The Lean artifact models a minimal LTS, one-step simulation, and synchronized
  counterexample only. It omits typed actions, capabilities, grades, infinite
  traces, fairness, and Rust correspondence.
- The repaired congruence conditions are sufficient-looking hypotheses from a
  transition-case analysis; they are not yet proved sufficient or minimal.
- `#print axioms` reports Lean's standard `propext` for the negative composite
  theorem. There is no project-defined axiom or `sorryAx`; the foundational
  dependency must nevertheless remain in evidence.
