# RFC 0010: Multiple verification engines and checked evidence composition

- Status: Draft
- Authors: Carlosian <carlosian@agentmail.to>
- Created: 2026-07-18
- Phase: 5

## Summary

Define one identity-bound verification-condition IR, two independent finite
safety engines, protocols for SMT and proof-assistant backends, a narrow
finite-invariant certificate, and fail-closed result composition.

An engine does not decide its public evidence class. It returns raw bytes,
scope, identities, a proposed status, and an optional witness or certificate.
The NMLT normalizer replays witnesses, checks certificates, enforces assurance
ceilings, and retains the raw return. A conflict or stale binding makes the
composed result `unknown`.

The first accepted domain is deliberately small: at most twenty Boolean state
variables, a Boolean initial predicate, a current/next transition predicate,
and a state invariant. It is sufficient to make the trust rules executable
without implying support for arbitrary symbolic mathematics.

## Motivation

Different verification methods fail differently. Explicit exploration can
miss behavior when a bound is reached; induction can fail for a true but
non-inductive property; an SMT solver may return `unknown`; a test generator
samples executions; a proof backend can report success for a stale obligation.
Converting all of these to a Boolean “verified” field would destroy the
information NMLT is designed to preserve.

The architectural requirement is therefore stronger than “run two tools”:

1. both tools consume the same exact claim and configuration identity;
2. their algorithms are genuinely distinct;
3. returned positive evidence is checked at the narrowest practical boundary;
4. scope and method constrain the maximum result class;
5. raw disagreement remains observable and prevents promotion.

## Research basis

### In the archive

The `search-the-archives` collector was run on 2026-07-18 for proof
certificates and SMT checking, multiple-engine disagreement, bounded model
checking and induction, and SMT-LIB proof trust. No relevant item was surfaced
in the local archive for those focused queries. One broad bounded-model query
returned lexical false positives about bounded-confidence opinion models and
unrelated recent work; they were excluded. The live arXiv leg also timed out or
was rate-limited for several queries. This RFC therefore does not attribute
any of its decisions to an archived work.

### New/current primary sources

- The official [SMT-LIB 2.7 standard](https://smt-lib.org/papers/smt-lib-reference-v2.7-r2025-02-05.pdf)
  supplies the external symbolic request language. NMLT records the exact
  standard version instead of writing only “SMT-LIB.”
- Biere, Cimatti, Clarke, and Zhu introduced SAT-based bounded symbolic model
  checking in [*Symbolic Model Checking without BDDs*](https://doi.org/10.1007/3-540-49059-0_14).
  It motivates a symbolic route but also reinforces that a bound is part of
  the claim, not a proof detail.
- [SMTCoq](https://arxiv.org/abs/1606.05947) demonstrates the architectural
  pattern of accepting external solver answers through a generic checker
  proved correct in a proof assistant.
- The [DRAT-trim project and system paper](https://www.cs.utexas.edu/~marijn/drat-trim/)
  demonstrate that an expressive solver can be separated from a smaller proof
  checker using a certificate format.
- Claessen and Hughes' original
  [QuickCheck paper](https://doi.org/10.1145/351240.351266) establishes the
  value of generated property tests while making no exhaustive-verification
  claim.

The NMLT policy is an inference from these sources, not a result proved by
them: keep search engines outside the trusted boundary when a small checker
can replay their result, and preserve `unknown` when no accepted certificate
is available.

## Goals

- Bind every backend invocation and return to exact model, claim,
  configuration, and verification-condition identities.
- Check one finite safety claim through deterministic reachability and finite
  inductiveness enumeration.
- Export a backend-neutral obligation to SMT-LIB 2.7 and a closed Lean 4
  inductiveness proposition.
- Accept only the narrow certificate format defined below for `proved` in the
  first implementation.
- Integrate deterministic model-based testing without raising its result above
  `tested`.
- Normalize and compose evidence without discarding backend bytes.
- Fail closed on stale bindings, malformed identities, rejected certificates,
  backend failure, or disagreement.

## Non-goals

- General SMT theorem proving, quantifiers, arithmetic, arrays, or
  uninterpreted data in the first VC.
- Treating a solver's `unsat`, a process exit code, or a proof assistant's
  stdout as proof by itself.
- Concluding that two implementations are statistically or logically
  independent merely because they have different names.
- Turning sampled tests into model checking.
- Claiming source-language soundness from a VC result; elaboration soundness
  remains a separate obligation.

## Verification-condition IR

The accepted VC is:

```text
FiniteSafetyVC = {
  schema             = "nmlt-vc/1",
  model_id           : sha256,
  claim_id           : sha256,
  configuration_id   : sha256,
  configuration      : finite-domain/stutter/assumption record,
  variables          : ordered unique Boolean names,
  initial(s)         : Bool,
  transition(s,s')   : Bool,
  property(s)        : Bool
}
```

`initial` and `property` may reference only current-state variables.
`transition` may reference current and next variables. Indices must lie in the
declared variable vector. The implementation rejects empty, duplicate,
malformed, or over-limit declarations and checks that `configuration_id` is
the digest of the configuration value.

Canonical VC serialization length-prefixes identities and variable names,
uses a constructor-tagged expression form, preserves variable order, and is
SHA-256 addressed. Caller-supplied model, claim, and configuration IDs are
inputs to that digest. Changing any one produces a distinct VC.

## Independent routes

### Route A: explicit reachability

The explicit engine:

1. enumerates every initial valuation;
2. explores successor valuations with deterministic breadth-first search;
3. checks the property when a state is dequeued;
4. returns a replayable shortest-state witness on violation;
5. when the queue is exhausted, returns the sorted reachable set as a
   `nmlt-finite-invariant/1` coverage certificate;
6. returns `unknown` when its depth or state budget cuts off a successor.

The normalizer independently checks that a refutation begins at an initial
state, every adjacent pair satisfies the transition predicate, and the final
state violates the property. It accepts `model_checked` only for
`CompleteFinite` scope with a certificate whose state count matches the scope
and whose set contains every initial state, satisfies the property, and is
closed under every transition. Thus an untrusted backend cannot promote a bare
success flag or self-declared coverage count.

### Route B: finite inductiveness

The inductive engine does not traverse the reachable graph. It enumerates all
current and next valuations and checks:

```text
Init(s)     => P(s)
P(s) /\ T(s,s') => P(s')
```

If initiation fails, the initial valuation is a real counterexample. If only
consecution fails, the engine returns `unknown`: that predecessor may be
unreachable, so reporting a refutation would be unsound. If both obligations
hold, the engine emits the finite-invariant certificate below.

The two routes intentionally have separate evaluators, iteration structures,
and raw-result construction. They share the reviewed VC datatype because
otherwise “same claim” would itself become ambiguous. Their final public
classification is performed only by the common normalizer.

## Certificate and checked return

The accepted proof object is:

```text
FiniteInvariantCertificate = {
  format             = "nmlt-finite-invariant/1",
  vc_digest          : sha256,
  model_id           : sha256,
  claim_id           : sha256,
  configuration_id   : sha256,
  invariant_states   : strictly increasing [u64]
}
```

The checker rejects an unknown format, stale identity, duplicate or
out-of-range state, omitted initial state, included property violation, or an
outgoing transition to an omitted state. Acceptance establishes that every
reachable state lies in the supplied inductive set and satisfies the safety
claim for the exact finite VC.

This checker is the accepted return path for the finite inductive engine, SMT
adapter, and proof-assistant adapter. Opaque proof text is retained but does
not establish `proved`.

## Backend protocols

### SMT-LIB

The SMT adapter emits an SMT-LIB 2.7 QF_UF query for a counterexample to
initiation or consecution. Its comments bind the VC, model, claim, and
configuration identities; declarations are derived from validated variable
names. It requests proofs and the solver version.

The status parser accepts exactly one line equal to `sat`, `unsat`, or
`unknown`:

- `sat` means only “the property is not inductive,” not “the property is
  reachable and false”; the normalized result is `unknown` without a replayed
  reachability witness.
- `unknown` remains `unknown`.
- `unsat` without an NMLT-checkable certificate remains `unknown`.
- malformed or multiple statuses are a backend failure and normalize to
  `unknown`.

### Proof assistant

The Lean 4 exporter emits closed definitions for the finite VC and a
`NMLTInductiveObligation` proposition. It emits no `axiom`, `sorry`, or claimed
theorem. The request records the exact Lean version and all four identities.

A proof backend may search for a proof or derive an invariant, but the first
accepted return is still `nmlt-finite-invariant/1`. A stdout line such as
“theorem checked” without that object remains `unknown`. Future RFCs may admit
kernel-checked native proof artifacts under their own formats and TCB records.

## Property- and model-based testing

`ModelTestPlan` records a 64-bit seed, case count, and maximum steps. A
deterministic generator selects initial states and enabled successors. A
passing run is `tested` with `Sampled` scope. A failing run returns the same
replayable state-witness form used by reachability and may normalize to
`refuted` after replay.

This hook is intentionally not reused by either exhaustive engine.

## Result normalization

Every raw result contains:

- exact backend name, version, build digest, and protocol;
- at least one exact trusted-component name, version, digest, and role;
- VC, model, claim, and configuration identities;
- method and scope;
- proposed status;
- optional witness or certificate;
- exact stdout/stderr or equivalent raw bytes.

The normalizer applies these ceilings:

| Raw evidence | Maximum normalized class |
| --- | --- |
| checked complete finite invariant certificate | `proved` |
| checked complete finite reachability certificate | `model_checked` |
| bounded reachability with no violation | `unknown` |
| sampled generated traces with no violation | `tested` |
| independently replayed violation trace | `refuted` |
| uncheckable solver/proof output | `unknown` |

Any `Bounded` or `Sampled` scope requesting `proved` is rejected before its
certificate is considered. This local invariant prevents a later evidence
composer from laundering bounded output into proof.

## Evidence composition

Composition retains all raw and normalized results and unions exact trusted
component records. It then applies:

1. no results => `unknown`;
2. any normalized `unknown` => aggregate `unknown` with reasons;
3. any positive/refuted disagreement, including raw proposed disagreement =>
   `unknown`;
4. otherwise a replayed refutation => `refuted`;
5. otherwise an accepted proof may yield `proved` only when every supplied
   positive result has complete-finite scope;
6. otherwise independently checked complete exploration => `model_checked`;
7. otherwise passing samples => `tested`.

An engine cannot be dropped merely because it disagrees. Selecting a subset
of results is a new composition input and therefore requires a distinct
configuration identity and an auditable reason.

## Trusted computing base

For the implemented finite domain, the TCB includes:

- VC validation and canonical SHA-256 identity;
- Boolean expression evaluation used by certificate and witness checking;
- the finite-invariant certificate checker;
- result normalization and evidence composition;
- the Rust compiler and target/runtime named by repository evidence.

The explicit explorer, inductive certificate producer, model-test generator,
SMT solver, proof search tactic, and AI assistance need not be trusted for a
positive result when their returned artifact is independently checked.
Translation from typed NMLT source to this VC remains trusted until separately
proved or translation-validated.

## Negative controls

The implementation tests that:

- a forged certificate omitting the initial state is rejected;
- a positive/refuted engine disagreement composes to `unknown` and retains
  both raw returns;
- a stale configuration binding is rejected;
- bounded evidence requesting `proved` becomes `unknown`;
- bounded or uncertified evidence requesting `model_checked` becomes `unknown`;
- sampled evidence requesting `model_checked` becomes `unknown`;
- missing or `unknown` backend versions and missing trusted components are
  rejected;
- `unsat` without an accepted certificate remains `unknown`;
- a property that is true initially but not inductive is not mislabeled as a
  reachability refutation by the inductive engine;
- proof-assistant stdout without a certificate remains `unknown`.

## Compatibility

The schema and protocol strings are versioned. Any change to expression
semantics, canonical serialization, certificate meaning, or normalization
ceilings requires a new version. Adding variables or changing their order
changes the VC digest even if the names form the same set.

Backend version and trusted-component fields do not accept “unknown.” A
backend upgrade changes evidence identity. Consumers that do not understand a
new result or certificate format must return `unknown`.

## Alternatives

### Trust two agreeing backends

Rejected. Agreement reduces some implementation risk but does not prove that
the tools consumed the same claim, avoided a common translation error, or made
a sound classification.

### Accept solver `unsat` directly

Rejected for `proved`. It would move the solver, encoding adapter, process
protocol, and output parser into the proof TCB. Such evidence may be recorded
under a lower, explicitly externally trusted class in a future RFC.

### Use one engine implementation with two traversal orders

Rejected as the Phase 5 independence demonstration. The selected routes have
different proof principles and different incompleteness modes.

### Generalize the VC before testing the boundary

Rejected. A broad expression language would obscure certificate soundness and
make exhaustive negative controls expensive. Arithmetic and structured values
should be admitted one theory at a time.

## Risks and unresolved questions

- Exhaustive valuation is exponential, and transition closure is quadratic in
  the state count. The twenty-variable validation limit prevents accidental
  unbounded resource use but is not a performance claim.
- The current certificate lists states explicitly and is unsuitable for large
  symbolic invariants.
- Native solver proof formats and native Lean proof objects are not yet
  accepted return formats.
- Backend independence is documented structurally, not quantified.
- The SHA-256 implementation and canonical serializer need differential test
  vectors before stable evidence compatibility is declared.
- Source-to-VC translation soundness and signed evidence remain future work.

## Implementation and exit gate

The implementation lives in `crates/nmlt-verify` and is independently runnable
with:

```sh
cargo test --manifest-path crates/nmlt-verify/Cargo.toml
```

Phase 5's narrow exit gate is met when that command demonstrates two distinct
engines on one exact claim, all negative controls above pass, the SMT and Lean
exports are identity-bound, and backend/trusted-component records are exact.
