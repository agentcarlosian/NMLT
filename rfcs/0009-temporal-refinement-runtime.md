# RFC 0009: Finite temporal, refinement, and runtime semantics

- Status: Draft
- Authors: NMLT project
- Created: 2026-07-18
- Phase: 4 candidate implementation

## Summary

Define a small executable Phase 4 layer over canonical finite transition graphs:

1. every state receives a distinguished identity-stutter self-loop;
2. eventuality and leads-to are checked by reachable SCC and fair-lasso analysis;
3. weak and strong fairness are explicit, action-indexed assumptions;
4. observations, hidden actions, and finite stutter projection are separate objects;
5. refinement checking is a localized finite one-step forward-simulation check;
6. concrete journals receive `accepted`, `rejected`, or `unknown` finite-prefix
   conformance verdicts.

The implementation is `crates/nmlt-temporal`. Its acceptance claims concern only
the supplied finite graph and mapping. They do not prove the parser, elaborator,
an unbounded system, liveness refinement, or the deployed runtime.

## Motivation

The Phase 1 rules distinguish identity stutter, observation-silent actions, and
refinement-hidden actions. Phase 4 needs executable consequences of those
distinctions. In particular:

- deadlock must not be silently confused with successful termination;
- a liveness counterexample must expose a repeatable cycle, not just a prefix;
- weak and strong fairness must give observably different answers;
- hiding must not erase an abstract state change;
- incomplete runtime data must not be upgraded to positive evidence.

## Research basis

### In the archive

The `search-the-archives` collector was run on 2026-07-18 with queries for
finite liveness/SCC checking, action fairness, stuttering refinement, and
runtime monitoring under partial observation. No directly relevant work was
surfaced in the local archive. The temporal query returned unrelated or much
more specialized 2026 papers; those results did not justify this design. Live
arXiv retrieval also timed out or returned HTTP 429 for several queries. This
absence is recorded rather than treating a lexical miss as evidence that no
relevant work exists.

### New/current primary sources

- Lamport's [Temporal Logic of Actions](https://www.microsoft.com/en-us/research/publication/the-temporal-logic-of-actions/)
  provides the state/action temporal framing and explicit stuttering closure.
- Lamport's [What Good Is Temporal Logic?](https://www.microsoft.com/en-us/research/publication/good-temporal-logic/)
  motivates invariance under stuttering as a specification requirement.
- Abadi and Lamport's
  [The Existence of Refinement Mappings](https://www.microsoft.com/en-us/research/publication/the-existence-of-refinement-mappings/)
  is the basis for concrete-to-abstract state mappings and the warning that
  auxiliary variables may be needed.
- Vardi and Wolper's
  [automata-theoretic model-checking paper](https://ieee-ceda.org/media/automata-theoretic-approach-automatic-program-verification)
  grounds finite-state temporal checking in accepting infinite computations.
- Tarjan's
  [depth-first SCC algorithm](https://epubs.siam.org/doi/abs/10.1137/0201010)
  supplies the graph decomposition used by the implementation.
- Kallwies, Leucker, and Sanchez's preprint on
  [monitoring under uncertainties and assumptions](https://arxiv.org/abs/2207.05678)
  directly supports retaining uncertainty when inputs are imprecise or absent.
- Bauer, Leucker, and Schallhart's
  [comparison of finite-trace LTL semantics](https://christian.schallhart.net/publications/2010--jlc--comparing-ltl-semantics-for-runtime-verification.pdf)
  supports an inconclusive outcome for finite observations instead of forcing
  every prefix into Boolean truth or falsehood.

### Implication for NMLT

NMLT should emit explicit lassos and partial-observation verdicts rather than a
bare Boolean. Forward simulation and observation agreement are useful finite
safety checks, but liveness transport needs additional divergence and fairness
obligations. The Phase 4 library therefore refuses to label its finite
refinement report as a proof of temporal refinement.

## Goals

- Make graph and witness ordering deterministic.
- Decide `eventually` and `leads_to` for finite graphs under named fairness.
- Distinguish weak from strong action fairness in both rules and tests.
- Give identity stutter a representation that cannot carry an action.
- Project finite observation words without erasing intensional trace metadata.
- Reject hidden concrete steps that change the mapped abstract state.
- Localize refinement and journal mismatches.
- Preserve `unknown` whenever required runtime data is absent or explicitly
  unknown.

## Non-goals

- A complete LTL parser or Büchi-automaton compiler.
- Symbolic or infinite-state checking.
- Proving that source-language elaboration produces the represented graph.
- Inferring fairness from scheduling or implementation behavior.
- Treating one-step forward simulation as liveness refinement.
- Cryptographic journal identity, provenance, or attestation.
- Erasing capability, resource-grade, event, or trust metadata through stutter
  projection.

## 1. Canonical finite graph

A graph is:

```text
G = <S, Init, T>
T subseteq S x (Action + IdentityStutter) x S
```

State identifiers are vector indices. Construction validates all indices, sorts
and deduplicates initial states and transitions, and derives sorted outgoing
transition identifiers. Thus BFS paths, SCC enumeration, and counterexamples
are repeatable for the same logical graph.

An identity stutter is the distinguished transition:

```text
(s, IdentityStutter, s)
```

It has no action label. A non-self-loop identity stutter is malformed. Temporal
checking closes every state by adding exactly this self-loop when it is absent,
including states with enabled declared actions. Consequently, progress needs an
explicit liveness or fairness assumption. Internal declared actions remain
`Action(name)` even when their endpoints have equal observations.

## 2. Fairness semantics

For infinite behavior `rho` and named action `a`:

```text
enabled(a, s)   iff an outgoing transition from s has action a
occurs(a, k)    iff transition k has action a

WF(a) = eventually always enabled(a)  implies  always eventually occurs(a)
SF(a) = always eventually enabled(a)  implies  always eventually occurs(a)
```

Weak fairness prevents an action that becomes continuously enabled from being
ignored forever. Strong fairness also prevents an action enabled infinitely
often, but intermittently, from being ignored forever. Identity stutters are
not action occurrences.

On a finite lasso loop `C`:

- `WF(a)` holds when either the loop visits a state disabling `a`, or an `a`
  transition occurs on the loop;
- `SF(a)` holds when either no loop state enables `a`, or an `a` transition
  occurs on the loop.

The checker never invents fairness. The caller supplies a `FairnessSet` and the
report is scoped to that exact set.

## 3. SCC and lasso checking

An ultimately periodic witness is:

```text
stem: s0 --t0--> ... --tn--> q
loop: q  --u0--> ... --um--> q
```

Both state and transition identifiers are recorded, and the structure is
validated against the graph.

### Eventuality

To check `eventually Q`:

1. retain only states where `Q` is false;
2. compute those reachable from an initial state without passing through `Q`;
3. decompose the induced graph into SCCs;
4. find a cyclic SCC admitting a fair closed walk;
5. return a shortest deterministic stem and a fair loop.

No such SCC means the property holds on every fair behavior of the stutter-closed
finite graph.

### Leads-to

`P leads_to Q` abbreviates `always (P implies eventually Q)`. The checker finds
each reachable trigger state satisfying `P and not Q`, then applies the
eventuality search to its `not Q` suffix. A counterexample records the trigger
state plus the complete initial stem and avoiding lasso.

### Fair component construction

For a candidate SCC, an unmet strong-fairness action with no internal occurrence
cannot remain enabled infinitely often. The algorithm removes states enabling
that action and recursively decomposes the remainder. An SCC is rejected for
weak fairness only when every state enables the action and no internal
occurrence exists. The witness constructor includes required action edges and
weak-fairness disabling states in one deterministic closed walk.

## 4. Observation, hiding, and stutter projection

`ObservationMap` is an explicit total field projection and renaming:

```text
observe : ModelState -> Observation
```

Missing source fields are errors. `stutter_project` collapses adjacent equal
observations in a finite word. It does not alter the underlying action path,
events, capabilities, grades, or fairness accounting. Two finite observation
prefixes are stutter-equivalent when their collapsed words are equal.

`ActionHiding` distinguishes:

```text
concrete action -> some abstract action
concrete action -> hidden
missing mapping -> error
```

Equal endpoint observations never imply that an action is hidden.

## 5. Finite refinement checking

A candidate contains:

```text
h          : State_C -> State_A
observe_C  : State_C -> Observation
observe_A  : State_A -> Observation
action_map : Action_C -> Option Action_A
```

The checker validates:

```text
R-INIT
  c in Init_C => h(c) in Init_A

R-OBS
  observe_C(c) = observe_A(h(c))

R-HIDDEN
  c -[a]-> c' and action_map(a) = hidden => h(c) = h(c')

R-VISIBLE
  c -[a]-> c' and action_map(a) = some b =>
    h(c) -[b]-> h(c') exists in A
```

Every mismatch contains the concrete state or transition identifier, mapped
endpoints, expected abstract action when applicable, and a stable category.
Reports may contain multiple mismatches to avoid repair-one-error-at-a-time
feedback.

Acceptance establishes only these finite one-step obligations. In particular,
it does not discharge hidden divergence, fairness mapping, environment
assumptions, connected-port noninterference, capability flow, or resource-grade
preservation from RFCs 0007 and 0008.

## 6. Journal-to-model trace adapter

A journal is an exact sequence of snapshots:

```text
record 0: <sequence, Initial, partial observation>
record i: <previous sequence + 1, Action(name) | IdentityStutter | Unknown,
           partial observation>
```

The mapping names each required model field and its journal field. Candidate
model states are propagated from the initial set through matching transitions.
Known observations constrain candidates. Missing or explicitly unknown values
do not constrain them and create an uncertainty record.

Verdicts are:

```text
accepted  all required action/observation data is known and a model path exists
rejected  known data contradicts every model path, or journal structure is invalid
unknown   at least one model path exists, but required data is missing/unknown
```

`unknown` is epistemic, not a wildcard form of acceptance. A known
contradiction still rejects when other fields are unknown. A rejection reports
the record index, sequence number, candidates before failure, and reason.

This adapter establishes finite-prefix trace conformance only. It cannot accept
an eventuality from a finite prefix, authenticate records, prove completeness of
logging, or show that the runtime implementation is the checked model.

## Evidence consequences and TCB

A valid Phase 4 evidence claim must bind:

- canonical graph identity;
- property/predicate identity;
- fairness-set identity;
- observation, state, and action mapping identity;
- checker artifact identity and version;
- exact verdict and lasso or mismatch payload;
- runtime journal identity and completeness assumptions when applicable.

The current library uses Rust predicates supplied as closures. Without a
canonical predicate serialization, a report is diagnostic output rather than a
portable proof certificate. Rust compilation/execution, graph construction,
and caller-supplied mappings remain in the trusted computing base. Independent
replay should validate the lasso transitions and predicate evaluations.

## Negative controls

The implementation tests must reject or distinguish:

- a non-self-loop labeled as identity stutter;
- a perpetually idle loop excluded by weak fairness;
- intermittent enabledness allowed by weak fairness but excluded by strong
  fairness;
- a fair action occurrence that still never reaches the goal;
- a hidden concrete transition that changes abstract state;
- a visible concrete transition with no matching abstract step;
- equal graph shape with unequal mapped observations;
- a journal action or known state value inconsistent with every model step;
- a nonconsecutive journal sequence;
- missing and explicitly unknown journal values, which must produce `unknown`.

## Compatibility

The crate is standard-library-only and does not change source syntax. RFC 0007
remains authoritative for intensional traces, boundary effects, and infinite
stutter equivalence. RFC 0008 remains authoritative for why local refinement is
not automatically compositional. A later engine integration must translate its
state/value domain into this crate without relying on debug string equality.

## Alternatives

- **General LTL-to-Büchi translation now:** deferred until predicate and formula
  identity are canonical. The narrower operators give inspectable witnesses.
- **Treat all cycles as fair:** rejected because it makes progress properties
  scheduler-dependent without recording the assumption.
- **Collapse weak and strong fairness:** rejected by the intermittent-enabledness
  negative control.
- **Boolean runtime verdicts:** rejected because absent observations cannot
  justify acceptance and need not justify rejection.
- **Observation equality as hiding:** rejected because an equal public snapshot
  can conceal authority use, events, or a different abstract state.

## Risks and unresolved questions

- Recursive SCC traversal can exhaust the call stack on adversarially deep
  graphs; an iterative implementation may be required before hostile inputs.
- The explicit graph representation is unsuitable for large symbolic models.
- Fairness is action-name indexed; parameterized or process/task fairness needs
  canonical action-instance identities.
- Runtime acceptance is existential over model nondeterminism. Applications may
  require universal or unique-path conformance.
- Observation maps currently project fields, not arbitrary verified functions.
- Liveness-preserving refinement remains an open, separately mechanized target.

## Implementation and exit gates

1. Canonical graph, universal identity-stutter closure, deterministic BFS/SCC: unit
   tested.
2. Eventuality, leads-to, weak/strong fairness, structured lassos: unit tested
   with semantic negative controls.
3. Observation projection, action hiding, finite stutter equivalence: unit
   tested.
4. Localized finite forward simulation: unit tested for acceptance and each
   principal mismatch.
5. Three-valued runtime adapter: unit tested for full, partial, contradictory,
   and malformed journals.
6. Workspace integration, evidence serialization, benchmark execution, and
   independent replay: required before Phase 4 project exit; not claimed by
   this RFC alone.
