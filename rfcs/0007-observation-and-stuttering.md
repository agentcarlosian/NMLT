# RFC 0007: Observation and stuttering semantics v1

- Status: Under review
- Authors: Carlosian <carlosian@agentmail.to>
- Created: 2026-07-18
- Mathematical-core backlog: `NMLT-P1-104`

## Summary

Separate emitted events from state observations and distinguish three notions
that must not be conflated:

1. an **identity stutter**, which leaves the complete state unchanged and has
   no action, visible event, authority use, or non-identity grade;
2. an **observation-silent transition**, whose endpoint observations agree but
   which remains a real action for resources and fairness;
3. a **refinement-hidden transition**, which maps to equality of the abstract
   state and has no connected external event.

V1 temporal properties transported by stuttering refinement use the
observation-indexed LTL fragment without `next`. Action occurrence, resource,
and fairness claims remain intensional and require separate preservation
obligations.

## Motivation

RFC 0001 currently uses `O` for both emitted output and state observation. That
allows an output event to be mistaken for a snapshot and encourages the false
rule “equal observations imply stutter.” The distinction matters immediately:
an internal provider call may leave the public snapshot unchanged while
consuming authority, emitting a runtime event, and affecting fairness.

TLA's stuttering discipline supports implementation steps that do not change
the abstract variables, but it does not justify erasing arbitrary internal
work from operational or evidence semantics. NMLT needs both the quotient used
by temporal refinement and the intensional trace used by evidence.

## Goals

- Give exact state, event, and observation projections.
- Define stutter equivalence on infinite observation words.
- Select a syntactically stutter-invariant temporal fragment.
- Define the conditions under which a concrete step maps to abstract stutter.
- Preserve action/resource/fairness data outside the observation quotient.
- Identify divergence and composition side conditions explicitly.

## Non-goals

- Event-counting temporal formulas in the stutter-invariant fragment.
- Treating observation equivalence as state equality.
- Inferring fairness after hiding.
- Claiming all LTL formulas containing `next` are non-stutter-invariant; v1
  simply excludes `next` from the transportable syntax.

## 1. Revised behavior signature

The candidate behavior object becomes:

```text
Behavior<S, I, E, V, G> =
  <Init, Act, step, observe, visible, Fair>

Init    subseteq S
step    : Act -> P(S x I x S x E x |G|)
observe : S -> V
visible : Act x E -> Option ExternalEvent
Fair    : finite set of action-indexed fairness obligations
```

`E` is an emitted step event and `V` is a state observation. `visible(a,e)` is
`none` for a locally internal event and `some v` for an external interface
event. This replaces the overloaded output/observation parameter in RFC 0001;
until RFC 0001 is revised, this RFC is the narrower proposal for Phase 1
mechanization.

An intensional trace is:

```text
rho = s0, (a0,i0,e0,g0), s1, (a1,i1,e1,g1), s2, ...
```

where `s0` is initial and each tuple belongs to the corresponding action
relation, or is the distinguished identity stutter defined next.

## 2. Three notions of silence

### Identity stutter

For every state `s`, temporal closure admits:

```text
stut(s) = (s, unit, no_action, no_event, epsilon, s)
```

It satisfies all of:

```text
pre = post
no declared action occurred
no visible or internal event occurred
no capability changed owner, phase, or liveness
grade = epsilon
```

It is not a source action and does not count as an occurrence for action
fairness.

### Observation-silent transition

A declared transition `s -[a,i/e,g]-> s'` is observation-silent when:

```text
observe(s) = observe(s')
and visible(a,e) = none
```

It remains a real action even if `s != s'`. Its grade, capability transition,
enabledness, fairness occurrence, and evidence event are retained. Equal
endpoint observations do not permit replacing it with `stut(s)` in the
intensional semantics.

### Refinement-hidden transition

Given a concrete-to-abstract state map `h`, a concrete transition may take
zero abstract steps only when:

```text
h(s) = h(s')
visible_C(a,e) = none
the transition uses no port connected in the surrounding composition
its capability/grade effect satisfies the declared hidden-effect policy
```

The first condition is stronger than
`observe_A(h(s)) = observe_A(h(s'))`. The connected-port condition is required
by the congruence counterexample in RFC 0008.

## 3. Observation projections

The state-observation word is:

```text
pi_V(rho) = observe(s0), observe(s1), observe(s2), ... in V^omega
```

The external event trace removes events mapped to `none` but retains their
position in the intensional trace metadata:

```text
pi_ext(rho) = the ordered sequence of v where visible(ak,ek) = some v
```

`pi_ext` may be finite even when `rho` is infinite. It is not used as the sole
carrier for state temporal logic.

The resource projection retains every grade:

```text
pi_G(rho,n) = g0 tensor ... tensor g_(n-1)
```

Observation quotienting never changes `pi_G`, capability provenance, or the
action/fairness projections.

## 4. Stutter equivalence

For infinite words `u,v : Nat -> V`, write `u approx_st v` when there are
strictly increasing, unbounded functions `f,g : Nat -> Nat`, both starting at
zero, such that corresponding finite nonempty blocks are constant and agree:

```text
for every block k:
  u is constant on [f(k), f(k+1))
  v is constant on [g(k), g(k+1))
  u(f(k)) = v(g(k))
```

Eventually constant words use an infinite tail of finite blocks with the same
value, so the definition also equates different amounts of terminal
stuttering. This relation is intended to be proved reflexive, symmetric, and
transitive in the mechanization.

Two intensional behavior traces are observation-stutter-equivalent when their
`pi_V` words are related by `approx_st`. They may still differ in resource,
authority, event, or fairness projections; such differences cannot be erased
from evidence.

## 5. Stutter-invariant property fragment

V1 observation-temporal formulas are indexed by a behavior and have no
`next` constructor:

```text
phi ::= atom p
      | not phi
      | phi and phi
      | phi until phi

eventually phi = true until phi
always phi     = not (eventually (not phi))
```

An atom is a total predicate `p : V -> Bool`. Satisfaction is standard over
suffixes of `pi_V(rho)`. Required theorem:

```text
u approx_st v  =>  (u |= phi iff v |= phi)
```

for every formula generated above. Peled and Wilke characterize the
stutter-invariant LTL-expressible languages as those expressible without the
next-time operator. V1 adopts the smaller syntax directly instead of running a
recognizer over arbitrary LTL.

Properties over action occurrences, exact step counts, grades, or external
events use separate indexed logics and do not receive this transport theorem
automatically.

## 6. Weak refinement

A safety refinement witness from concrete `C` to abstract `A` contains:

```text
h          : State(C) -> State(A)
input_map  : I_C -> I_A
event_map  : ExternalEvent_C -> ExternalEvent_A
grade_map  : G_C -> G_A
hidden     : Act(C) -> Bool
```

with obligations:

```text
R-INIT
  s in Init_C => h(s) in Init_A

R-OBS
  observe_C(s) = observe_A(h(s))

R-HIDDEN
  s -[a,i/e,g]->C s' and hidden(a) =>
    h(s) = h(s')
    and visible_C(a,e) = none
    and boundary_use(a) = empty
    and hidden_grade_ok(g)

R-VISIBLE
  s -[a,i/e,g]->C s' and not hidden(a) =>
    exists abstract step
      h(s) -[a_A,input_map(i)/e_A,g_A]->A h(s')
    with event_map(visible_C(a,e)) = visible_A(a_A,e_A)
    and g_A <= grade_map(g)
```

Safety formulas in the observation fragment transport from `A` to `C`.
Liveness transport additionally requires divergence and fairness obligations:

```text
R-DIVERGENCE
  no fair concrete execution can perform infinitely many hidden steps while
  making only finitely many matching abstract steps, unless the abstract
  observation is legitimately eventually constant and the property does not
  rely on the suppressed action progress

R-FAIR
  every abstract weak/strong fairness premise used by the property is
  discharged from named concrete fairness premises
```

No tool may infer either obligation from `R-INIT` through `R-VISIBLE`.

## 7. Hiding and composition

Hiding changes `visible(a,e)` from `some v` to `none` for selected boundary
events. It does not:

- change a declared transition into an identity stutter;
- delete the event from evidence or resource projections;
- make a connected event local before composition;
- discharge fairness or divergence obligations.

Composition synchronizes before hiding. A synchronized event may become
externally invisible only after all participating state, capability, grade,
and fairness effects are retained in the composite intensional step.

## Evidence consequences

Evidence for a transported temporal claim identifies:

- the exact `observe`, `visible`, and hiding maps;
- whether the claim uses only the stutter-invariant property fragment;
- every hidden action and its boundary-use/grade policy;
- fairness and divergence obligations and their status;
- the intensional witness, even when a rendered view collapses repeated
  observations.

A UI may collapse identical observations for display, but the canonical
counterexample and evidence identity bind the uncollapsed trace.

## Negative controls

The implementation or mechanization must reject or refute:

- classifying a non-identity state change as identity stutter;
- erasing a provider call because public observations are equal;
- transporting a formula containing `next` through the v1 generic theorem;
- hiding an external event before checking connected-port synchronization;
- dropping grades or capability transitions from an observation-silent step;
- transporting liveness without named fairness and divergence obligations;
- accepting `R-HIDDEN` from equal observations when abstract states differ;
- canonicalizing a witness by deleting intensional steps.

## Compatibility

The split from `Behavior<S,I,O,G>` to `Behavior<S,I,E,V,G>` changes typed IR,
behavior identity, refinement witnesses, and evidence schemas. It should occur
before typed behavior artifacts are released. Observation-map or hiding-map
changes invalidate dependent refinement and temporal evidence.

## Alternatives

- **One output/observation type:** fewer parameters, but conflates step events
  and state snapshots.
- **Equal observation means stutter:** too weak for capability, fairness, and
  compositional reasoning.
- **LTL with unrestricted `next`:** expressive, but generic stuttering
  transport becomes false.
- **Delete hidden steps from canonical traces:** compact, but destroys resource
  and causality evidence.

## Risks and unresolved questions

- Event properties need a stutter-safe edge semantics or an explicitly
  stutter-sensitive logic.
- `R-DIVERGENCE` needs a tractable syntactic or ranking-function discipline.
- The block definition of `approx_st` and its eventually constant case require
  careful mechanization before use.
- Observation equality may be expensive for symbolic domains; backends must
  expose an obligation rather than assume decidability.

## Research basis

- Lamport's [Specifying
  Systems](https://lamport.azurewebsites.net/tla/book-02-03-04.pdf) defines TLA
  formulas around invariance under adding or deleting stuttering steps.
- Peled and Wilke's
  [stutter-invariance result](https://doi.org/10.1016/S0020-0190(97)00133-6)
  connects stutter-invariant LTL properties to formulas without `next`.
- The MIT [I/O Automata overview](https://groups.csail.mit.edu/tds/i-o-automata.html)
  keeps input, output, and internal actions explicit and reports both ordinary
  and fair trace notions as compositional under its conditions.

These sources motivate the boundaries; NMLT's exact event/evidence split still
requires its own proof.

## Implementation plan

1. Mechanize intensional traces, identity stutter, and observation projection.
2. Prove `approx_st` is an equivalence.
3. Prove the observation LTL fragment invariant under `approx_st`.
4. Mechanize safety transport for `R-INIT` through `R-VISIBLE`.
5. Add divergence and fairness structures before any liveness transport claim.
6. Preserve uncollapsed intensional traces in evidence and test every negative
   control above.
