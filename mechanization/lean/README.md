# NMLT Lean mechanization

This directory is the pinned Lean 4.30.0 home for NMLT metatheory. It now
contains eight independent checked families:

1. the Phase-1 counterexample to unconditional refinement congruence under
   synchronized composition;
2. a small Phase-2 typed executable core for total state expressions,
   simultaneous updates, explicit affine provider authority, blocked actions,
   and system-indexed safety properties;
3. the Phase-7 mathematical cost/privacy/energy/uncertainty product algebra,
   conservative composition operations, product order, and Boolean budget
   predicate;
4. M10 behavior-indexed traces, constructive positive evidence, directed
   refinement laws, coinductive/up-to safety, and the selected typed-core bridge;
5. M11 input/output/internal interfaces, assume/guarantee message predicates,
   global input receptiveness, exact wiring coverage, and bounded exact-action
   composition congruence;
6. M11 finite truth-table contracts with exact payload identity,
   assumption/guarantee variance, and open-refinement identity/composition;
7. M11 affine-authority partition/transfer, product-grade, and rely/guarantee
   resource-refinement rules; and
8. an executable M11 canonical finite-table checker whose certificates carry
   typed `Fin` state/action maps.

The artifact intentionally depends only on Lean's standard library. It contains
no `sorry`, project-defined axiom, `native_decide`, native-code proof, or
external solver call. The `#print axioms` audit currently reports only Lean's
standard `propext` and, for the theorem using derived decidable equality,
`Quot.sound`. Several concrete provider and property-indexing theorems use no
axioms at all. Exact output is described below.

## Build

Install the pinned toolchain with
[Elan](https://lean-lang.org/doc/reference/latest/Build-Tools-and-Distribution/Managing-Toolchains-with-Elan/),
then run:

```sh
cd mechanization/lean
lake build
```

Direct checking is also possible:

```sh
lake env lean NMLT.lean
```

The `#print axioms` commands make foundational dependencies visible and must
never report `sorryAx` or a project-defined axiom. A build proves only that
Lean's kernel accepts the definitions and proofs in this directory; it does not
prove that the model is a faithful implementation of future Rust code.

## Checked Phase-2 slice

The sources are deliberately separated by responsibility:

```text
NMLT/Core/TypedCore.lean       values, dependent state, actions, evaluator
NMLT/Typing/Judgments.lean     affine action/context judgment
NMLT/Metatheory/Soundness.lean preservation, blocked, frame, affine theorems
NMLT/Core/Provider.lean        executable provider instance and controls
```

The core establishes the following properties for the encoded fragment:

- typed expressions evaluate to the Lean interpretation of their exact type;
- post-state construction evaluates every update RHS against the frozen
  pre-state, and action formation rejects duplicate targets;
- `frame_soundness` copies every field outside the action's syntactic write
  set;
- `progress_or_blocked`, `blocked_iff_no_step`, and
  `blocking_reason_exact` classify every action as having its unique step or
  being blocked by a false guard or missing targeted capability;
- `action_preservation` retains the system-indexed state signature and realizes
  the exact inferred capability output context;
- `no_duplication_of_affine_capability` proves both at-most-one multiplicity
  and no authority fabrication across a step;
- `Property system signature` plus `Property.transport` makes an explicit
  system-identity equality proof necessary for re-indexing;
- the provider dispatch action type-checks, executes, consumes its authority,
  and sets its dispatch flag, while the missing-authority static control is
  refuted.

This is an intrinsically typed executable model, not yet a compiler
correctness theorem. It has only `Bool` and `Nat`, one explicit capability
effect per action, no branching command language, no authority-returning
provider operation, and no temporal behavior. Its characteristic-function
capability store is an extensional set abstraction; finiteness of the generic
identity domain is not yet encoded (the concrete provider identity type is
finite). A later implementation must prove correspondence from its concrete
finite map and elaborated IR. Progress deliberately does not assert deadlock
freedom.

## Checked Phase-7 grade-algebra slice

`NMLT/Grades/Algebra.lean` models exact mathematical grades as three unbounded
natural coordinates and one natural bounded by 1,000,000 ppm. It checks:

- identity, associativity, and commutativity of sequential and conservative
  parallel composition, including saturated uncertainty addition;
- identity, associativity, commutativity, and idempotence of componentwise
  choice;
- least-zero, componentwise-order, least-upper-bound, and join-induced-order
  theorems;
- monotonicity plus binary and nonempty-finite distribution of sequencing over
  choice;
- exact equivalence between the encoded Boolean budget predicate and product
  order, with a soundness projection from acceptance to all four inequalities.

This is an algebra theorem, not a verified implementation theorem. Lean uses
unbounded `Nat` for cost, privacy, and energy; Rust uses checked `u64` and
returns `unknown` on overflow. The alignment of operation definitions is
manual. The capsule does not encode the `.nmltg` parser, plan analyzer,
annotations, differential-privacy sensitivity, physical energy, uncertainty
calibration, extraction, or compiler correctness.

## Checked M11 open-composition slice

`NMLT/Behavior/OpenComposition.lean` defines open actions, port/message
assumption and guarantee predicates, global input receptiveness, bidirectional
wiring, and synchronous products in which connected boundary actions cannot
interleave independently. `WiringEquivalent` compares the complete concrete
and abstract wiring relations, including the peer endpoint domain; this blocks
the extra-abstract-connection defect missed by pointwise mapped-edge checks.

For exact-action, state-surjective `StrongRefinement`, Lean proves structural
step and product congruence from equality of the whole wiring relation.
Separately, predicate-contract compatibility and global receptiveness prove
component receptiveness transport, both connected-output synchronization
enabledness directions, composability preservation, and product
receptiveness. Lean wiring is an arbitrary relation, not the Rust executable
profile's one-to-one representation. These theorems use no axioms. A
nonidentity positive refinement performs a real synchronization, and an exact
negative control shows that an extra abstract wire blocks a peer-only step and
violates wiring equivalence. The claim-specific
[M11 evidence manifest](../../benchmarks/results/open-composition/m11-001a-evidence.json)
binds those theorem handles and controls to the exact sources, toolchain,
checkers, TCB inventory, and axiom output. The profile deliberately omits weak
hiding, label maps, payload type identities, capabilities, grades,
temporal/circular contracts, fairness, divergence, liveness, and Rust
correspondence. The original unconditional counterexample remains imported as
a permanent negative control.

`NMLT/Behavior/OpenRefinement.lean` adds the M11-001b relation. Its contract
predicates are Boolean truth tables over a finite index type; boundary label
maps are complete and injective, payload identities must be equal, abstract
assumptions are included in concrete assumptions, and concrete guarantees are
included in abstract guarantees. Identity and composition are proved without
added axioms. The executable Rust checker uses separately named finite enum
types and finite accepted-value sets; correspondence between those
representations and Lean is not proved. The
[M11-001b evidence manifest](../../benchmarks/results/open-refinement/m11-001b-evidence.json)
binds the theorem handles and Rust controls. Product congruence for this new
relation is supplied at the abstract label-mapped level by
`NMLT/Behavior/OpenMappedCongruence.lean`. That file combines operational
simulation with complete typed port bijections, direction preservation,
assumption contravariance, guarantee covariance, mapped whole wiring, and
invariant transport. Its positive control uses distinct concrete and abstract
port types on both sides and a real synchronization. Rust's string labels,
payload hashes, and finite table encodings remain outside that theorem.
`NMLT/Behavior/OpenResourceCongruence.lean` now defines one bundled
`ResourceAwareMappedRefinement` and lifts it through all eight structural
product-action constructors. `NMLT/Behavior/OpenEncodingCorrespondence.lean`
checks the normalized finite certificate and proves a general semantic contract
for every accepted certificate: common payload identity, surjective typed maps,
pointwise contract/resource variance, authority narrowing, and whole wiring.
The Rust encoder and normalized validator are not verified extraction.

## Axiom audit

On the pinned toolchain, `lake build` reports:

- no axioms for provider action typing, duplicate-update rejection,
  missing-capability rejection, canonical-system disequality, or
  property/system indexing;
- `propext` for frame, progress/blocked, and action-preservation proofs;
- `propext` and `Quot.sound` for the generic no-duplication theorem;
- `propext` for the pre-existing negative composition-congruence result;
- `propext` and `Quot.sound` for the grade associativity, distribution, order,
  and monotonicity audit, and `propext` alone for Boolean budget soundness.
- no axioms for the audited M11 input-receptiveness, synchronization,
  wiring-isolation, step-congruence, and composition-congruence theorems.
- no axioms for the audited M11-001b predicate inclusion, open-refinement
  identity/composition, exact-payload, and variance declarations.
- no axioms for the audited M11-001c mapped-wiring isolation, label-aware
  product lifting, composite contract variance, and invariant transport
  declarations.
- `propext` and `Quot.sound` for the all-case resource lift and accepted
  canonical-certificate semantic contract.

These are Lean foundational dependencies, not NMLT assumptions. The audit
rejects `sorryAx` and any project-defined axiom.

## Research basis

The `search-the-archives` workflow was rerun on 2026-07-18 with queries for
mechanized progress/preservation, affine capability uniqueness, dependent
property indexing, and frame soundness. **In the local archive**, no close
match surfaced; lexical recall was weak and the arXiv collector timed out, so
that absence is not evidence of novelty.

**New/current primary-source leads consulted separately** were
[Oxide](https://arxiv.org/abs/1903.00982), whose syntactic ownership calculus
uses progress and preservation;
[RustBelt](https://doi.org/10.1145/3158154), which motivates keeping privileged
operations in the trusted boundary; and the
[usage-aware semantics for GraD](https://arxiv.org/abs/2011.04070), which
derives single-pointer and resource-accounting results from a semantics that
tracks usage. A 2026 formalization of
[graded modal dependent type theory](https://arxiv.org/abs/2603.29716) further
reinforces that full grade preservation deserves its own mechanized layer.

**Implication for NMLT:** this slice uses an exact affine set semantics and
explicit consume/discard events, while postponing borrowing and general grades.
The cited systems provide proof techniques and boundary warnings; none proves
the soundness or novelty of NMLT's encoding.

## Repository layout

```text
NMLT/
  Core/             semantic objects and operational relations
  Typing/           value, state, action, and capability judgments
  Temporal/         traces, observations, stuttering, and fairness
  Behavior/         indexed traces, refinements, coinduction, open composition
  Refinement/       simulations, hiding, and property transport
  Composition/      ports, synchronization, grades, and authority
  Grades/           conservative quantitative algebra and budget order
  Metatheory/       preservation, frame, productivity, and soundness proofs
  Counterexamples/  executable semantic mutants and failed conjectures
```

Files move out of `Counterexamples` only when the repaired statement and proof
are stable. Generated NMLT proof exports, if added later, belong under
`mechanization/lean/Generated/` and must never overwrite hand-reviewed kernel
definitions.
