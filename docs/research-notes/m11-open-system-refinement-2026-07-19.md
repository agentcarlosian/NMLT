# M11 open-system refinement research

Date: 2026-07-19

Researcher: Carlosian <carlosian@agentmail.to>, with AI-assisted retrieval and
implementation review.

This note fixes the research boundary for the first M11 open-system refinement
slice. It distinguishes Carlosian's local archive from current primary sources;
neither category is itself evidence that the NMLT implementation or theorem is
correct.

## Archive findings and gap

Four focused searches were first run against
`~/research-archive/math_frontier_search_index.json` for interface automata,
assume/guarantee contracts, alternating simulation, and compositional I/O
refinement. The archive recovered
[compositional refinement of assume/guarantee contracts](https://arxiv.org/abs/2103.13743),
but it did not recover a primary interface-automata or I/O-automata foundation.
The remaining hits were unrelated control and optimization papers. This is an
archive coverage gap, not evidence that the missing theories do not exist.

That discovered gap is now repaired in the local source manifests. The
curated supplement adds `1201.4449`, `1210.2450`, and `1306.3050`; loop 2 now
names the official I/O-automata references and the correctly titled
Chilton--Jonsson--Kwiatkowska article. The deterministic search view contains
2,347 records (2,334 original plus 13 curated additions), and an archive-only
readback now retrieves *Modal Interface Automata* for the query above. The
pre-update absence remains recorded here so the research decision is auditable.

The regenerated view has build identity
`a380ca306603c27052d25780037821a4b5db8b2ba7716da03b49d44c5e912a7d`
and exact-file SHA-256
`ec47047fa958a39cfb466989e9d2f866c0e288b02aa6845f7b4ac7dcd459eb4a`.
The build identity now includes the generator itself as well as the immutable
offline source, curated-source manifest, and research-loop manifest. The local
`~/research-archive` directory is not yet its own version-controlled repository;
these identities provide readback, not remote durability or authorship.

## Current primary sources consulted separately

Lynch and Tuttle's I/O-automata model classifies actions as input, output, or
internal, requires every input action to remain enabled, and gives
compositional trace semantics. The
[MIT overview](https://groups.csail.mit.edu/tds/i-o-automata.html), the
[original introduction](https://groups.csail.mit.edu/tds/papers/Lynch/TM-373.pdf),
and its [bibliographic record](https://groups.csail.mit.edu/tds/papers/Lynch/CWI89.html)
are the foundation for M11's first action-polarity and receptiveness rules.

[Modal Interface Automata](https://arxiv.org/abs/1306.3050) provides a more
expressive interface theory with explicit output obligations, implicitly
allowed inputs, and compositional parallel operators. Its diagnosis of a
composition defect in an earlier I/O modal theory reinforces NMLT's decision
to prove one deliberately restricted composition operator before adding modal
or alternating refinement.

[Verifying Compositional Refinement of Assume/Guarantee Contracts](https://arxiv.org/abs/2103.13743)
establishes compositional contract-refinement conditions for a linear
dynamical-system setting. It supports treating assumption discharge as a real
side condition, but it does not justify transplanting that paper's linear
programming result into NMLT's transition semantics.

[Interface Simulation Distances](https://arxiv.org/abs/1210.2450) shows a
quantitative alternating-refinement distance that is nonexpansive under
parallel composition. Alternating refinement remains a later candidate once
the finite safety theorem below is stable; the first NMLT slice does not claim
that game semantics.

## Frozen M11-001a profile

The first supported profile is intentionally smaller than all of the theories
above:

1. Every named action has exactly one local polarity: `input`, `output`, or
   `internal`.
2. Inputs are controlled by the environment. Receptiveness means that every
   declared input has an outgoing transition at every finite model state.
3. Outputs and internal actions are locally controlled. The Rust executable
   profile uses explicit one-output/one-input connections with matching opaque
   channel identities; every connected action has exactly one peer.
   Unconnected action names are side-namespaced in the product. Lean instead
   permits an arbitrary bidirectional wiring relation and requires equality of
   that whole relation across refinement; its theorem does not establish the
   Rust one-to-one restriction.
4. An explicitly connected output/input pair synchronizes. Unconnected
   input/output actions and all internal actions interleave. A synchronized
   boundary action becomes internal in the product; that is distinct from a
   refinement-hidden action.
5. The finite executable contract shell records exact symbolic safety-claim
   identifiers. Every local assumption must have exactly one declared peer
   guarantee discharge, and that provider must have no assumptions of its own.
   This deliberately rejects mutual or otherwise conditional symbolic
   discharge. Identifier equality is not logical implication, and the shell
   does not establish contract satisfaction over infinite behaviors. The Lean
   model instead states assumption and guarantee predicates over port messages
   and uses compatibility plus global receptiveness to prove synchronization
   enabledness; it does not model circular contract dependencies.
6. The Rust component refinement may be lifted through a fixed peer only when
   visible boundary mappings are injective, action polarity and connection
   status are reflected, the peer is receptive, and a concrete action hidden
   by the component refinement is not connected to the peer boundary. Lean's
   structural theorem is exact-action: it has neither label maps nor hidden
   component steps, and requires equality of the complete wiring relations.
7. Rust product construction uses checked cardinality/index arithmetic and
   defaults to at most 100,000 states and 1,000,000 generated transition
   candidates, with a conservative 50,000,000 work-item preflight before
   compatibility and product enumeration. A work item bounds the module's
   explicit logical loops, not wall-clock time, bytes, allocator behavior, or
   standard-library comparison/sort internals; callers may choose smaller
   explicit limits.
8. The congruence conclusion is finite safety forward simulation. It says
   nothing about fairness, liveness, progress, divergence, capabilities,
   grades, payload subtyping, or resource transfer.

This profile directly excludes RFC 0008's checked hidden-synchronization
counterexample: a locally hidden action cannot become a boundary
synchronization after composition.

## Promotion boundary

M11-001a may be reported complete only when:

- Rust rejects ill-directed, channel-mismatched, or multiply connected
  channels; nonreceptive inputs, including at unreachable states; missing or
  circular symbolic discharge; hidden connected actions; noninjective visible
  boundary maps; non-reflected concrete or abstract connections; arithmetic
  overflow; and products above configured state, transition, or conservative
  work-item limits;
- the supported product is deterministic in construction and accepted by the
  existing finite refinement checker under the lifted state map;
- Lean proves structural exact-action, state-surjective product congruence
  from `StrongRefinement` and `WiringEquivalent`, and separately proves
  composability preservation and product receptiveness from compatibility and
  global receptiveness, without `sorry`, custom axioms, or unchecked decision
  procedures;
- Lean retains a nonidentity positive refinement with a real synchronization,
  an exact-semantics broken-wiring negative control, and RFC 0008's original
  unconditional hidden-synchronization counterexample; and
- the trusted-component inventory and the claim-specific
  [M11 evidence manifest](../../benchmarks/results/open-composition/m11-001a-evidence.json)
  bind the exact theorem handles, controls, sources, checkers, Lean toolchain,
  and audited axiom sets.

The parent M11-001 remains open after this slice. Typed payload compatibility,
semantic assume/guarantee satisfaction, capabilities, grades, and rely/fairness
transport require separate extensions and must not be inferred from M11-001a.
The Rust instance checker and Lean theorem also use related but not identical
contract and interface representations; no compiler-correspondence theorem
between them is claimed.
