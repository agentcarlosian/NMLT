# RFC 0008: Lean mechanization and compositional refinement

- Status: Under review
- Authors: Carlosian <carlosian@agentmail.to>
- Created: 2026-07-18
- Mathematical-core backlog: `NMLT-P1-105`, `NMLT-P1-106`

## Summary

Select Lean 4, pinned to version 4.30.0, as the first NMLT mechanization
environment and place the proof project under `mechanization/lean/`.

The first artifact refutes unconditional congruence of RFC 0001's weak forward
simulation under synchronized composition. A concrete action can be hidden
because it maps to equality in its component, yet synchronize with a peer and
change the peer's observable state. The concrete sender refines the abstract
sender in isolation, while the composed systems admit no observation-
preserving refinement.

This RFC therefore replaces the unconditional target with an interface-aware
congruence conjecture whose proof requires connection preservation,
noninterference of hidden steps, capability partitioning, grade compatibility,
and separate fairness/rely-guarantee obligations.

## Decisions

1. **Environment:** Lean 4.30.0, managed by a checked-in `lean-toolchain`.
2. **Dependency baseline:** Lean standard library only until a concrete theorem
   requires a reviewed package; no Mathlib dependency is currently needed.
3. **Proof policy:** no `sorry`, `sorryAx`, project-defined axioms, native
   decision procedures, or unchecked external solver results in accepted
   metatheory.
4. **Congruence result:** the candidate rule as stated is **refuted**, not
   proved. The kernel-checked counterexample is the deliverable for
   `NMLT-P1-106`; a repaired conditional theorem remains open.

## Motivation

Paper rules do not establish soundness, and a broad proof-assistant project can
hide the core question under libraries and automation. The first mechanization
should instead be small, executable, and adversarial: encode the candidate
transition/refinement/composition definitions and ask whether the desired
congruence theorem is even true.

Lean is selected because it combines dependent types, inductive definitions,
proof checking, executable functions, and project-local extensibility in one
toolchain. The selection is pragmatic, not a claim that Lean is uniquely
suited to temporal metatheory. The 2026 MPST liveness mechanization in Rocq is a
strong counterpoint: its use of coinductive trees and `paco` shows that Rocq
may become preferable if Lean's infinite-trace development becomes dominated
by coinduction infrastructure.

## Goals

- Pin a reproducible proof checker and directory contract.
- Keep hand-reviewed definitions separate from generated proof exports.
- Encode a falsifiable form of the candidate semantics before proving derived
  theorems.
- Record a checked counterexample when a desired theorem is false.
- State sufficient-looking repair conditions as explicit proof obligations.
- Make foundational and custom-axiom dependencies inspectable.

## Non-goals

- Mechanizing the complete language in this RFC.
- Treating `lake build` as evidence that Rust implements the same semantics.
- Proving liveness, productivity, or fairness transport with the initial
  finite transition artifact.
- Using automation output as trusted evidence without kernel checking.

## 1. Repository layout

The proof project is:

```text
mechanization/lean/
  lean-toolchain
  lakefile.toml
  NMLT.lean
  NMLT/
    Core/
    Typing/
    Temporal/
    Refinement/
    Composition/
    Metatheory/
    Counterexamples/
    Generated/
```

Ownership of the directories is semantic:

- `Core`: small mathematical objects and operational relations;
- `Typing`: value, state, action, and capability judgments;
- `Temporal`: traces, observations, stuttering, and fairness;
- `Refinement`: simulation and property-transport definitions;
- `Composition`: interfaces, synchronization, grades, and authority flow;
- `Metatheory`: theorem statements and checked proofs;
- `Counterexamples`: executable failed conjectures and semantic mutants;
- `Generated`: tool-produced obligations, never imported by hand-reviewed core
  merely to make a theorem pass.

No generated file may declare an axiom, overwrite a hand-reviewed definition,
or be accepted without exact source/tool identity in evidence.

## 2. Minimal encoded candidate

The checked artifact defines a labelled transition system:

```text
LTS<L,V> = <State, Init, Step : State x L x State -> Prop,
            Observe : State -> V>
```

and the one-step weak forward simulation from RFC 0001:

```text
WeakRefines(C,A,hidden,labelMap) = <h, init, obs, hiddenStep, visibleStep>

init:
  Init_C(s) -> Init_A(h(s))

obs:
  Observe_C(s) = Observe_A(h(s))

hiddenStep:
  Step_C(s,l,t) -> hidden(l) -> h(s) = h(t)

visibleStep:
  Step_C(s,l,t) -> not hidden(l) ->
    Step_A(h(s),labelMap(l),h(t))
```

The synchronized product has left, right, and synchronized labels. A label
connected to a peer cannot step independently.

## 3. Refuted conjecture

The tempting rule is:

```text
C refines A
-------------------------- INVALID-CONGRUENCE
C ||_K D refines A ||_K D
```

for the same connection `K` and peer `D`.

### Counterexample

Let abstract sender `A` have one state, observation `false`, and no action. Let
concrete sender `C` have the same state and observation plus one action
`ping`. Declare `ping` hidden. It leaves the sender state unchanged, so
`C refines A` has the unique state map and satisfies the candidate rules.

Let receiver `D` have Boolean state initially `false`. Its `receive` action
changes that state to `true`. Connect sender `ping` to receiver `receive`.

```text
C || D:
  <unit,false> --sync(ping,receive)--> <unit,true>

A || D:
  no corresponding synchronized step
```

If the composite synchronization remains hidden, the refinement rule requires
the mapped composite states to be equal. Observation preservation then implies
`(false,false) = (false,true)`, a contradiction. If it is visible, the abstract
composite has no matching step. Either classification fails.

The checked Lean theorems are:

```text
senderRefinement          : WeakRefines C A ...
concreteSynchronization  : Step_(C||D) <unit,false> sync <unit,true>
noCompositeRefinement    : not Nonempty(WeakRefines (C||D) (A||D) ...)
```

`lake build` on Lean 4.30.0 checks all three. `#print axioms` reports no custom
axiom or `sorryAx`; the negative theorem uses Lean's standard `propext` axiom,
which is part of the selected foundational TCB and is reported explicitly.

## 4. Diagnosis

RFC 0001's local hidden-step rule constrains only the component state map. Its
composition rule allows declared synchronization. Nothing prevents a locally
hidden concrete action from using a connected port and causing a peer step.
Local state equality is therefore not contextual silence.

The existing `compatible` checklist's statement that a hidden synchronization
output maps to the composite silent observation is insufficient: the peer's
state transition may still change the composite observation, resources,
authority, or enabledness.

## 5. Repaired safety congruence conjecture

For a fixed peer `D`, connection maps `K_C` and `K_A`, simulations
`R : C refines A`, and product state map `H(c,d) = (R.h(c),d)`, require:

### Interface preservation

```text
I-LABEL
  every nonhidden concrete boundary label maps to an abstract label with the
  same direction, payload type, and mapped event

I-CONNECT
  K_C(l,d) iff K_A(labelMap(l),d) for every nonhidden l

I-NO-HIDDEN-BOUNDARY
  hidden(l) implies there is no d with K_C(l,d)
```

`I-NO-HIDDEN-BOUNDARY` directly excludes the checked counterexample.

### Transition and observation preservation

```text
I-OBS
  composite observation is componentwise and R preserves the left observation

I-INPUT
  D is receptive for every connected input admitted by the abstract interface,
  or a named rely condition proves the needed enabledness

I-SYNC
  synchronized payload/event maps commute with labelMap
```

### Resource preservation

```text
I-CAP
  component capability stores are disjoint; synchronization transfers rather
  than duplicates authority; hidden steps use no peer-owned authority

I-GRADE
  gradeMap is monotone and respects epsilon and tensor for the composed grades
```

### Assumption and fairness preservation

```text
I-RELY
  the peer guarantee discharges every concrete and abstract assumption used by
  the transported safety property

I-FAIR
  required only for liveness: action/task fairness maps through synchronization
  and hidden divergence is excluded or discharged separately
```

The repaired safety conjecture is:

```text
R : C refines A
interfaceCompatible(R,D,K_C,K_A)
resourceCompatible(R,D)
relyCompatible(R,D)
------------------------------------------------ CONDITIONAL-CONGRUENCE
lift(R,id_D) : (C ||_(K_C) D) refines (A ||_(K_A) D)
```

No liveness conclusion follows without `I-FAIR` and divergence obligations.

## 6. Proof plan for the repaired theorem

Use `H(c,d) = (R.h(c),d)`.

1. **Initial:** component initiality and `R.init` establish composite initiality.
2. **Observation:** product observation and `R.obs` establish `I-OBS`.
3. **Hidden left step:** `I-NO-HIDDEN-BOUNDARY` makes the step independent;
   `R.hiddenStep` gives left map equality and the peer is unchanged.
4. **Visible independent left step:** `R.visibleStep` supplies an abstract left
   step; `I-CONNECT` reflects that it remains independent.
5. **Independent peer step:** reuse the same `D` step; the left state is
   unchanged.
6. **Synchronization:** `I-NO-HIDDEN-BOUNDARY` makes the concrete left step
   visible. `R.visibleStep`, `I-CONNECT`, and `I-SYNC` supply the matching
   abstract synchronization with the same peer step.
7. **Capability and grade side facts:** `I-CAP` proves disjointness/transfer;
   `I-GRADE` proves the composite grade inequality.

This case split is a proof outline, not a proof. The current Lean artifact
mechanizes the false conjecture's counterexample only.

## 7. Mechanization gates

An NMLT theorem is reportable as `proved` only when:

- the pinned Lean kernel checks it from a clean checkout;
- `#print axioms` contains only the explicitly allowed foundational axioms;
- the theorem statement is bound to the exact semantic definitions;
- there is no `sorry`, custom axiom, `native_decide`, or unchecked oracle;
- generated files and dependencies are identified;
- negative controls show that weakening a required premise breaks the result;
- a separate correspondence artifact links Rust IR/semantics to the Lean
  definitions before the theorem is attributed to an implementation.

## Evidence consequences

The current artifact supports a checked **refutation** of unconditional
congruence for the encoded candidate. It does not support:

- congruence of the repaired rule;
- soundness of RFC 0001 as a whole;
- correspondence with the Rust compiler;
- liveness or fairness transport;
- novelty of the counterexample or repair conditions.

Evidence should name Lean 4.30.0, the exact source digest, foundational axiom
set, theorem name, and the fact that the result is a counterexample.

## Negative controls

The mechanization program must retain:

- the checked hidden-synchronization counterexample;
- a variant omitting `I-NO-HIDDEN-BOUNDARY`, for which congruence remains false;
- a variant breaking connection reflection;
- a variant sharing one affine capability between components;
- a variant with a nonmonotone or non-homomorphic grade map;
- a liveness example where infinite hidden steps invalidate progress transport;
- a theorem file containing `sorry` that the evidence gate rejects.

## Compatibility

The Lean version is part of theorem-checker identity. Upgrading it requires a
clean rebuild, axiom/dependency review, and new evidence. Changing transition,
observation, hiding, composition, or simulation definitions invalidates all
dependent proof artifacts even if theorem names remain unchanged.

## Alternatives

- **Rocq:** mature coinduction ecosystems such as `paco` and strong PL
  mechanization precedent. It remains the fallback if infinite-trace proofs in
  Lean become disproportionately difficult.
- **Agda:** excellent executable dependent types, but a smaller immediate
  ecosystem for the intended automation and evidence tooling.
- **Isabelle/HOL:** strong automation and refinement traditions, but a larger
  integration boundary for the Rust-centered repository.
- **TLA+/TLAPS only:** directly aligned with temporal specifications, but less
  convenient as the sole home for syntax-directed substructural typing
  metatheory.
- **Prove the desired theorem on paper first:** the checked counterexample shows
  why executable falsification must precede a long proof attempt.

## Risks and unresolved questions

- Lean's treatment of infinite traces and guarded productivity needs a
  concrete prototype before the environment choice is permanent.
- Standard-library-only development may duplicate useful libraries; any new
  dependency must be justified against TCB and maintenance cost.
- Forward simulation remains incomplete for refinements needing prophecy,
  history, or multiple abstract steps.
- Input receptiveness and assume/guarantee discharge are not yet formalized.
- The sufficient conditions above may be stronger than necessary; weakening
  them is future research after the theorem is proved.

## Research basis

- De Moura and Ullrich's [Lean 4 system
  description](https://doi.org/10.1007/978-3-030-79876-5_37) documents Lean's
  combined theorem-prover and efficient programming-language design.
- The official [Lean language
  reference](https://lean-lang.org/doc/reference/latest) and [Elan toolchain
  guide](https://lean-lang.org/doc/reference/latest/Build-Tools-and-Distribution/Managing-Toolchains-with-Elan/)
  support a project-pinned, reproducible checker workflow.
- The MIT [I/O Automata overview](https://groups.csail.mit.edu/tds/i-o-automata.html)
  states compositional trace results for a model with explicit input, output,
  internal actions, receptiveness, and fairness structure—the kind of interface
  conditions missing from the refuted candidate.
- Lynch, Segala, Vaandrager, and Weinberg's [Hybrid I/O Automata
  work](https://groups.csail.mit.edu/tds/papers/Lynch/LSVW.html) explicitly
  proves composition respects its implementation relation under compatibility
  conditions.
- [Formally Verified Liveness with Multiparty Session Types in
  Rocq](https://arxiv.org/abs/2605.23633) is a 2026 preprint and mechanization
  whose coinductive proof scale is evidence for keeping liveness obligations
  separate from the initial safety result.

## Implementation plan

1. Keep the checked counterexample permanently as a regression test.
2. Encode ports, direction, connection maps, and `I-NO-HIDDEN-BOUNDARY`.
3. Prove the repaired safety theorem by the six transition cases above.
4. Add capability partition and grade homomorphism structures and proofs.
5. Define finite prefixes and infinite observation words; then prove
   stuttering safety transport.
6. Add fairness/divergence only after the safety theorem is stable.
7. Build a typed-IR correspondence test before attributing Lean theorems to
   compiler output.
