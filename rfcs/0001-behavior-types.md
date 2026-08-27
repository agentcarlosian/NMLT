# RFC 0001: Behavior types

- Status: Under review
- Authors: Carlosian <carlosian@agentmail.to>
- Created: 2026-07-18
- Candidate rules frozen: 2026-07-18
- Revised: 2026-08-27 (composition/hiding aligned with RFC 0008 counterexample;
  small-model weak safety lift recorded)

## Summary

Introduce `Behavior<S, I, O, G>` as the typed semantic object connecting
initial states, labeled actions, observations, infinite traces, temporal
properties, quantitative grades, composition, and refinement. This revision
replaces the original shape sketch with a candidate set-theoretic model and
mechanizable formation, property, composition, and weak-simulation rules.

The rules are a research candidate, not a soundness claim. Acceptance requires
mechanization and the negative controls listed below.

**Revision 2026-08-27:** Section 4 no longer states that a hidden synchronization
may silently map to the composite `silent` observation. That clause is refuted
by the mechanized counterexample in RFC 0008 /
`NMLT.Counterexamples.CompositionCongruence.noCompositeRefinement`. Composition
and refinement-through-composition now require interface-aware premises;
unconditional congruence is explicitly rejected.

## Motivation

Snapshot types cannot prevent a temporal formula for one transition system
from being silently reused for another. Untyped transition relations also hide
which changes are observable, which transitions count as stuttering, and which
resource algebra grades an action. NMLT needs these parameters at the type
boundary so that incompatible behaviors require an explicit mapping.

The candidate incorporates three established lessons without claiming their
combination is already new mathematics: TLA+-style behaviors and stuttering,
mode separation in Quint, and protocol-facing behavioral types. NMLT adds a
proof-relevant refinement and evidence layer, which remains a hypothesis until
mechanized and implemented.

## Goals

- Index predicates and temporal propositions by one behavior.
- Give nondeterminism a relational meaning independent of an explorer.
- Make observations, frames, stuttering, fairness, and grades explicit.
- Define a finite executable fragment without reducing the semantics to finite
  exploration.
- State checkable compatibility and refinement obligations.

## Non-goals

- Probability, continuous time, or hybrid dynamics in v1.
- General recursion or implicit partial functions.
- Treating one model-checking run as unbounded proof.
- Fixing final surface syntax in this RFC.

## 1. Mathematical parameters

Let `Type` be the selected total value universe. A grade algebra is an ordered
commutative monoid

```text
G = (|G|, tensor, epsilon, <=)
```

where `tensor` is associative and commutative, `epsilon` is its identity, and
`tensor` is monotone in both arguments. A concrete grade instance may count
calls, cost, latency budget, privacy exposure, or authority use. No theorem may
assume cancellation, inverses, or idempotence unless the instance declares it.

For state `S`, input `I`, output `O`, and grade `G`, an action meaning is a
finitely branching relation:

```text
A subseteq S x I x S x O x |G|
```

`(s, i, s', o, g) in A` means that input `i` permits one step from `s` to
`s'`, emitting `o` and consuming grade `g`. Finitely branching is required by
the executable v1 fragment, not by the abstract definition.

## 2. Behavior object

A behavior is the record

```text
B = <S, I, O, G, Init, Act, step, observe, silent, Fair>
```

with:

```text
Init    subseteq S
Act     finite set of labels
step    : Act -> P(S x I x S x O x |G|)
observe : S -> O
silent  in O
Fair    finite set of fairness obligations
```

The distinguished semantic stutter at `s` is
`(s, unit, s, silent, epsilon)`. It is available to temporal closure but is not
an ordinary source action. A non-identity internal transition is not silently
reclassified as stuttering merely because `observe(s) = observe(s')`; hiding it
requires a refinement mapping and proof obligation.

### Formation

```text
Gamma |- S : Type    Gamma |- I : Type    Gamma |- O : Type
Gamma |- G : GradeAlg
Gamma |- Init : S -> Prop
Gamma |- step : Act -> S -> I -> FinSet(S x O x |G|)
Gamma |- observe : S -> O    Gamma |- silent : O
fair_wf(B, Fair)
---------------------------------------------------------------- B-FORM
Gamma |- B : Behavior<S, I, O, G>
```

`fair_wf` requires each weak- or strong-fairness declaration to name an action
in `Act` and use the same behavior state and input types.

### Action typing and frames

The surface action checker produces an explicit write set `W`, capability
transition `Delta -> Delta'`, and grade expression `g`:

```text
Gamma; Delta |- guard : Bool
Gamma; Delta |- updates : Update<S, W>
Gamma; Delta |- outputs : O
Gamma; Delta |- g : |G|
frame(updates, fields(S) - W)
linear_ok(Delta, updates, Delta')
---------------------------------------------------------------- ACT
Gamma; Delta |- action : Action<B, W, g> => Delta'
```

`frame` means every state field outside `W` is equal in pre- and post-state.
The elaborator must emit these equalities; omission is not implicit permission
to modify state. `linear_ok` forbids duplication of linear capabilities and
permits dropping only affine ones.

## 3. Traces and temporal meaning

A behavior trace is an infinite sequence

```text
s0, (a0, i0, o0, g0), s1, (a1, i1, o1, g1), s2, ...
```

where `s0 in Init` and every position is either a member of `step(a_k)` or the
distinguished stutter. `Traces(B)` is defined coinductively. The implementation
may store finite prefixes or lassos, but those representations do not redefine
`Traces(B)`.

State predicates and temporal properties are behavior-indexed:

```text
Gamma |- B : Behavior<S, I, O, G>
Gamma, state : S |- p : Bool
------------------------------------------- PRED
Gamma |- always_B(p) : Property<B>

Gamma |- phi : Property<B>    Gamma |- psi : Property<B>
--------------------------------------------------------- TEMP
Gamma |- not(phi), phi and psi, next(phi), eventually(phi) : Property<B>
```

There is deliberately no rule converting `Property<B1>` to `Property<B2>` by
structural equality of state records. An explicit accepted refinement or
property map is required.

Satisfaction is standard linear-time satisfaction over every trace:

```text
B |= phi  iff  for every rho in Traces(B), rho, 0 |= phi
```

Finite exploration can refute a safety property with a finite witness. It may
establish only `model_checked` within recorded bounds; a finite prefix cannot
construct `proved` evidence for unbounded liveness.

## 4. Composition candidate

An open behavior declares named input/output ports and assume/guarantee
properties. Formation of a composite behavior still needs port typing and
authority hygiene. **Separately**, transporting a refinement through a
composite requires interface premises that the original candidate omitted.

### 4.1 Well-formed composition (`compatible`)

`compatible(B1, B2, C)` requires:

1. every connected output type equals its input type;
2. each output has at most one producer unless `C` provides an explicit merge;
3. each component guarantee discharges the peer assumption it is used for;
4. linear authority is transferred, never copied;
5. the composed grade is `g1 tensor g2`;
6. every synchronization edge is an explicit element of the connection relation
   `C` (there is no implicit wire).

```text
Gamma |- B1 : Behavior<S1, I1, O1, G>
Gamma |- B2 : Behavior<S2, I2, O2, G>
compatible(B1, B2, C)
--------------------------------------------------------- COMPOSE
Gamma |- B1 ||_C B2 : Behavior<S1 x S2, I, O, G>
```

The product step is an interleaving step or a declared synchronization step.
This RFC does not assume that arbitrary local liveness properties survive
composition; preservation requires a separate rely/guarantee and fairness
obligation.

### 4.2 Refuted: unconditional congruence under hiding

The following rule is **not** part of the candidate. It is recorded as a
permanent negative control.

```text
C refines A
--------------------------  INVALID-CONGRUENCE  (refuted)
C ||_K D refines A ||_K D
```

**Counterexample (RFC 0008 / Lean):** abstract sender `A` has no steps.
Concrete sender `C` has a `ping` declared hidden that leaves `C`'s own state
and observation unchanged, so `C` weakly refines `A` in isolation. Peer `D`
flips an observed bit on `receive`. Connect `ping` to `receive`. The composite
`C || D` takes an observable synchronized step that `A || D` cannot match. If
the sync is classified hidden, observation preservation forces `false = true`.
If it is classified visible, the abstract product has no matching step.

Lean artifact:
`mechanization/lean/NMLT/Counterexamples/CompositionCongruence.lean`
(`senderRefinement`, `concreteSynchronization`, `noCompositeRefinement`).

**Diagnosis:** a local hidden-step rule that only constrains the component
state map does not imply contextual silence. A label that is hidden for
refinement may still be connected at the composition boundary and change a
peer's observation, authority, or enabledness.

The original clause "hidden synchronization outputs map to the composite
`silent` observation" is therefore **withdrawn**. Mapping a sync label to
`silent` does not repair the counterexample when the peer's state changes.

### 4.3 Conditional refinement lifting (repaired candidate)

For a fixed peer `D`, connection maps `K_C` and `K_A`, and simulation
`R : C refines A`, lifting through parallel composition is allowed only under
an explicit interface judgment. The following premises are the current
candidate (see RFC 0008 §5 for the full checklist):

```text
I-NO-HIDDEN-BOUNDARY
  hidden(l) implies l is not connected in K_C

I-CONNECT
  every concrete connection maps to an abstract connection, every abstract
  connection is reflected, and connection endpoint domains agree
  (`WiringCovered` plus `IsolationReflects`: an unconnected concrete label
  stays unconnected after labelMap). Pointwise image of labelMap alone is
  insufficient; coverage without isolation is also insufficient when
  labelMap is non-injective.

I-LABEL / I-OBS / I-SYNC
  nonhidden boundary labels preserve direction and payload; composite
  observation is componentwise; sync payload maps commute with labelMap

I-INPUT / I-RELY
  receptiveness or named rely conditions for enabledness used by transported
  safety claims

I-CAP / I-GRADE   (resource-aware extension; not required for the pure
                   interface safety conjecture)
  capability stores remain disjoint under sync; gradeMap is monotone and
  respects epsilon and tensor

I-FAIR            (liveness only; never inferred from safety)
```

```text
R : WeakSimulation<C, A>
interfaceCompatible(R, D, K_C, K_A)
------------------------------------------------ CONDITIONAL-CONGRUENCE
lift(R, id_D) : WeakSimulation<(C ||_(K_C) D), (A ||_(K_A) D)>
```

**Mechanized positive fragments today:**

- Small LTS / `WeakRefines` / `parallel` model (`Core/Transition.lean`):
  `weakConditionalCongruence_safety` in
  `NMLT.Behavior.WeakConditionalCongruence` (2026-08-27). Premises:
  `I-NO-HIDDEN-BOUNDARY`, `WiringCovered`, and `IsolationReflects` (the
  I-CONNECT domain fragment: an unconnected concrete label stays unconnected
  after `mapLabel`). Default composite hidden/map classifiers. Axioms:
  `[propext]`. Necessity: ping/receive violates `I-NO-HIDDEN-BOUNDARY`;
  `CollidedAbstractWire` violates `IsolationReflects` while keeping coverage.
- Exact-action / strong refinement product congruence with whole-wiring
  equivalence (`OpenComposition.StrongRefinement.compositionCongruence`).
  That theorem claims nothing about weak hiding, capability partition, grades,
  fairness, or liveness.

**Open (residual `CONDITIONAL-CONGRUENCE`):** `I-CAP` / `I-GRADE` / `I-FAIR`
/ `I-RELY`; the OpenComposition port model under *weak* hiding; deriving
`IsolationReflects` from `WiringCovered` without injectivity of `mapLabel`.

## 5. Refinement candidate

For concrete `C` and abstract `A`, a refinement witness contains:

```text
R = <h, input_map, output_map, hidden, fair_map, obligations>
h : State(C) -> State(A)
```

The initial candidate is observation-preserving weak forward simulation:

```text
(R-INIT)  s in Init(C)  =>  h(s) in Init(A)

(R-OBS)   output_map(observe_C(s)) = observe_A(h(s))

(R-STEP)  s -[a,i/o,g]->C s' implies either
          (a in hidden and h(s) = h(s'))
          or h(s) -[a*,input_map(i)/output_map(o),gA]->A h(s')
             with gA <= grade_map(g)

(R-FAIR)  each abstract fairness obligation used by a preserved liveness
          claim is discharged by fair_map; it is never inferred from safety.
```

`a*` is exactly one matching abstract action in this candidate. Sequences of
abstract steps, prophecy variables, and backward simulation are postponed
extensions. Hidden concrete steps must map to equality of abstract state, not
merely equal observations.

```text
Gamma |- R : WeakSimulation<C, A>
------------------------------------------- REFINE
Gamma |- refine(R) : Refines<C, A>
```

Safety properties over abstract observations may be transported through
`Refines<C,A>`. Liveness transport additionally requires `R-FAIR`. Reflexivity
and transitivity are metatheorems to prove, not constructors accepted by fiat.

Contextual use of `Refines<C,A>` under parallel composition is **not** given by
`REFINE` alone. It requires the conditional lifting rule in §4.3. In particular,
`a in hidden` and `h(s) = h(s')` do not justify treating `a` as silent in a
composite where `a` is connected to a peer.

## 6. Decidable v1 fragment

The first implementation restricts executable checking to finite state types,
finite input domains, finite branching action relations, total non-recursive
functions, and decidable grade comparison. Infinite types may appear in
mathematical specifications only when a backend supplies separately checked
evidence. The frontend must label an obligation outside this fragment instead
of approximating it silently.

## Evidence consequences

- A successful type derivation supports only a `type_safety` claim.
- Explicit-state exploration supports `model_checked` with exact bounds and
  engine identity.
- `proved` requires a proof object accepted by a named proof checker.
- A refinement result identifies both behavior identities, witness `R`,
  observation contract, preserved property class, and fairness obligations.
- The type checker, elaborator, grade-algebra implementation, and proof/result
  checker are trusted only for the claims they construct.

## Negative controls

The mechanization and implementation must reject or refute:

- applying a `Property<B1>` to `B2` without an explicit map;
- omitting a frame equality and then changing the omitted state;
- duplicating a linear provider capability in a branch or composition;
- hiding an observable field without an observation map;
- calling a non-identity internal change a stutter solely because observations
  are equal;
- composing incompatible ports or two linear-capability producers;
- transporting liveness through a simulation with no fairness proof;
- promoting bounded exploration to `proved`;
- accepting a refinement whose mapped concrete step has no abstract match;
- accepting `INVALID-CONGRUENCE` (unconditional lift of weak refinement through
  `||_K`); the ping/receive composite is the permanent witness;
- accepting conditional lifting when `I-NO-HIDDEN-BOUNDARY` is dropped (a
  hidden label remains connected);
- accepting conditional lifting when whole-wiring coverage fails (an extra
  abstract wire blocks a peer-only step that the concrete product still takes);
- accepting conditional lifting when `IsolationReflects` fails while coverage
  holds (non-injective `mapLabel` collapses an independent concrete label onto
  a connected abstract name; `CollidedAbstractWire`);
- treating "sync maps to composite `silent`" as sufficient for contextual
  silence when the peer observation changes.

## Alternatives

- **Primitive modal type:** elegant, but postpones executable meaning and makes
  the first mechanization harder to falsify.
- **Coalgebra only:** attractive for infinite behavior, but frame, grade, and
  open-port obligations still need an explicit surface-to-core judgment.
- **Session types only:** strong for communication protocols but do not by
  themselves cover arbitrary state predicates, hiding, or TLA+-style temporal
  refinement.
- **Untyped transition IR:** simpler to build but loses the central prevention
  of cross-behavior property misuse.

## Risks and unresolved questions

- Forward simulation is incomplete for some valid refinements.
- Fairness under open composition remains the largest semantic risk.
- Residual weak `CONDITIONAL-CONGRUENCE` (resources, grades, fairness,
  OpenComposition weak hiding) is still open. The small-model safety lift is
  mechanized; do not describe it as the full RFC rule.
- `I-CAP` / `I-GRADE` may be stronger than necessary; weaken only after the
  pure interface theorem is proved, and keep necessity counterexamples.
- `O` currently serves as both state observation and emitted output; the
  mechanization may separate them if examples show a real ambiguity.
- Grade comparison direction and authority grades need concrete instances.
- Finitely branching semantics may be too restrictive for symbolic domains;
  any relaxation must preserve an executable sublanguage boundary.

## Implementation and evidence plan

1. Encode this object and rules in a selected proof assistant.
2. Prove formation inversion and action-frame preservation.
3. Prove or refute refinement reflexivity and transitivity.
4. Model the provider-attempt capability and all four semantic mutants.
5. Test composition with bounded channel and two-phase commit.
6. Accept or revise the RFC only after the failed obligations are recorded.
7. **Done (2026-07-18):** refute `INVALID-CONGRUENCE`; retain as regression
   (`NMLT-P1-106` / RFC 0008).
8. **Done (2026-07-19):** mechanize exact-action strong product congruence
   under whole-wiring equivalence.
9. **Done (2026-08-27):** small-model weak safety lift with
   `I-NO-HIDDEN-BOUNDARY`, `WiringCovered`, and `IsolationReflects`;
   necessity for the first and third premises.
10. **Done (2026-08-27):** omit-whole-wiring peer-only control on the small
    model (`ExtraAbstractWire`).
11. **Next:** residual weak congruence (`I-CAP` / `I-GRADE` / `I-FAIR`,
    OpenComposition weak hiding).
12. **Next:** lowering `hide action` to label hiding (RFC 0007); keep
    unconditional congruence out of the docs.
