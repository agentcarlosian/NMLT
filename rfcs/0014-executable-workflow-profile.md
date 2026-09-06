# RFC 0014: Executable workflow profile

- Status: Draft
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-06
- Tracking issue: not assigned
- Design disposition: pending

## Summary

Propose a small local NMLT interpreter for programs that coordinate typed jobs,
exclusive resources, and reusable checked artifacts. The same profile should
serve Lean proof development, mathematical experiments, and resource-bearing
software. It extends the execution scope of
[ADR 0004](../docs/decisions/0004-language-mathematics-pivot.md) while retaining
Lean-owned behavioral semantics. Authorization to execute R0 establishes
baselines and this proposal; it does not accept the proposed semantics or
establish any new theorem.

## Motivation

The current language can emit a finite behavioral artifact and explore a
restricted model, but cannot run a general input-to-output workflow with typed
host effects. Its static behavior and dynamic authority worlds are separate;
received authority is not reusable by a later source action. Adding an agent
loop to that architecture would not resolve these gaps.

The immediate comparison is with small programs using existing Lean, Python,
and Rust tools. The [R0 contracts](../docs/r0-baseline-contracts.md) define that
baseline. NMLT should reduce practical friction through explicit composition
and evidence boundaries, rather than reproduce tool APIs under new syntax.

## Goals

- One reusable local execution profile for all three audiences.
- Shared source meaning and explicit interpreter/model correspondence limits.
- Typed job requests, outcomes, resource accounting, and interruption recovery.
- Persistent mathematical evidence distinct from affine control permissions.
- User-written invariant obligations with precise checking scope.
- Ordinary Lean artifacts that remain usable without a generating agent.

## Non-goals

- A replacement mathematical language, proof kernel, or foundation model.
- Distributed execution, exactly-once external effects, or unrestricted I/O.
- Proved compilation or host isolation merely from recording an artifact.
- Liveness or fairness transport before the existing safety/path milestones.
- Reactivating postponed runtime and agent experiments without review.

## Guide-level explanation

The following are behavioral contracts, not accepted NMLT syntax:

1. A proof workflow reads a fixed Lean target and candidate proof patches. It
   spends one attempt permit per submitted candidate, records rejection or
   checking results, and returns ordinary Lean source plus an evidence record.
   Changing the target creates a new obligation rather than completing the old
   one. Multiple later proofs can reuse an accepted lemma.
2. A discovery workflow examines a fixed predicate, records a checked
   counterexample to one conjecture, and explicitly revises that conjecture.
   A later proof applies only to the revised statement. Novelty review is a
   separate output from formal proof acceptance.
3. A worker workflow transfers an exclusive job permit to a receiver. The
   receiver later uses it to start local work. Completion, failure,
   cancellation, and late responses produce distinct outcomes. It can reuse a
   completed immutable value without recreating job authority.

The complete executable R0 examples are ordinary host-language baselines.
Equivalent NMLT examples become an implementation gate after the profile is
reviewed. Proposed commands such as `run` and `replay` are not existing CLI
capabilities.

## Reference semantics

### Source routes and values

Each accepted construct must have one source interpretation: common parsing,
resolution, source spans, typed expressions, and lowering where applicable.
The implementation plan must name the authoritative route for every construct
and close declaration/reference coverage gaps before promoting it. The
retained ordinary elaboration kernel is not automatically the behavioral
checker. Reuse its facilities only where their judgments apply.

The proposed profile includes modules, typed inputs/outputs, records, tagged
outcomes, pattern matching, total pure functions, and bounded iteration and
collections. It does not silently lower an unsupported property to ordinary
code or omit it. Concrete grammar and type rules require reviewed fixtures
before this RFC can be Accepted.

Immutable values and artifact references can be copied. Affine resources
control job transitions, budget reservations, and explicitly scoped writes.
A resource transition consumes the old control state and produces its named
successor; observing immutable job events need not consume control authority.
An accepted proof is a persistent value, not a one-use permission or an
authorization to perform an external action.

### Behavior, execution, and finite abstraction

R1/M1--M3 must supply one resource-bearing behavior state, initial condition,
observations, and product-step relation, including receive-then-use. A local
interpreter and explorer should share their common step operations. This is
implementation reuse; comparison with Lean is a separate correspondence
obligation.

Host requests and responses are explicit events. A request is not proof that
an effect happened, and an adapter response is not a mathematical theorem.
Inputs such as Lean files and large collections may remain opaque references.
Every finite model declares its state domains, abstraction function, job
bounds, permitted environment behavior, and mapping from execution events.
Finite evidence applies to that declared model. A theorem about all runtime
behavior additionally needs a justified abstraction/simulation relation and
its host assumptions.

Composition preserves relevant observations and remaining open ports,
direction, payload, and hiding. Nested binary products require tests and
theorems before broader composition is advertised. Refinement must preserve
the exact property and environment assumptions used by a downstream claim.

### Typed effects and bounded identities

A proposed job request names the adapter/protocol version, input schema and
identity, target context when relevant, output contract, authorized scope,
and resource reservation. The implementation accepts typed variants for
completed work, acknowledged failure, timeout, cancellation request,
cancellation acknowledgement, and uncertain completion. Unsupported versions
and malformed responses fail explicitly.

An attempt is identified by `(run, task, slot, generation)`, never by a reusable
slot alone. A finite model declares maximum slots and generations; exhausting
either bound ends with an explicit resource-limit outcome rather than
wrapping or silently reusing an identity. Restart preserves the recorded
identity or creates a distinct generation. Responses must match the exact
attempt and input/context identity. Duplicate collection and stale
generations cannot create a second artifact acceptance or regain authority.

Job control is affine; collecting a successful immutable result can release
that control while retaining the result. Exclusive-write permissions name a
canonical scope and must be enforced at the host boundary. Their formal model
does not establish that unrelated host processes obey the same policy.

Cancellation is a request until acknowledged. A late completion can be
recorded for reconciliation without being accepted as the active attempt's
result. A crash can leave external completion unknown. Idempotency keys are
used only with adapters that implement a documented idempotency contract;
local ownership alone gives no exactly-once external execution guarantee.

### Budgets, outcomes, and replay

Declared model grades, reserved attempts, and observed expenditure are
different quantities. Resource accounting records reservation, dispatch,
settlement, and any permitted refund. Cancellation or failure does not erase
work already spent; only a documented unused reservation may be returned.
Unknown actual usage stays unknown. The first executable profile uses explicit
bounds rather than unbounded retry.

Records distinguish candidate, observed outcome, checked finite witness,
checked theorem, and human review. They bind exact inputs, implementation and
adapter versions, checker/toolchain configuration, relevant dependencies,
limits, and outputs. A timeout or failed proof attempt remains unknown about
the mathematical claim.

Replay validates recorded decisions and artifacts under the recorded
identities. It does not assert that an external effect ran again, or that an
LLM would reproduce the same response. Resume may reuse completed work only
when its complete acceptance context matches and its recorded evidence can
be revalidated. A changed target, definition, input, adapter, or policy makes
old acceptance stale. Records alone do not provide independent verification
of the producer that wrote them.

### User-defined invariant path

A user can propose a state predicate over a named behavior or finite
abstraction. The artifact must carry the predicate's exact identity, state
and observation interpretation, assumptions, and bounds. Checking then follows
one explicitly selected route:

- evaluate it on a recorded finite trace, reporting only that trace;
- exhaustively inspect the declared finite reachable model, recording complete
  coverage and returning unknown if a search limit prevents completion; or
- prove initialization and step preservation in Lean and derive the invariant
  over the specified reachable states.

A refutation requires an initial-to-failure path and a witness that actually
falsifies the indexed predicate. A disconnected bad state is not a reachable
counterexample. Empty state spaces or disabled actions must be disclosed;
the pilot contracts require admitted initial states and meaningful execution.
Arbitrary temporal syntax, runtime enforcement, and liveness are separate
extensions, not consequences of this invariant path.

### Native Lean checker contract

A proof target binds the exact declaration statement and definition/import
environment. Candidate proof patches are checked against that fixed target;
successful compilation of a different theorem is insufficient. Final checking
reconstructs ordinary Lean files in a pinned environment, examines transitive
axioms under the project's written policy, and follows the compatible
independent checking/target-comparison path for stronger acceptance claims.
Process exit, an empty goal list, and a scheduler completion bit cannot replace
that evidence. Draft dependencies cannot close a theorem obligation.

## Evidence consequences

The Rust frontend, interpreter, serializer, host adapters, process runner,
abstraction relation, and checker integration remain named trust boundaries
until specific verification removes them from a specific claim. The existing
ordinary typed-core results and behavioral results retain their present scope.

An R0 baseline result is experimental implementation evidence, not a new NMLT
certificate or theorem. A Lean-checked baseline proof is about its exact
target under recorded assumptions, subject to the stated toolchain checks.
User-defined invariants must not inherit a universal claim from a bounded
search. Formal proof, faithful statement interpretation, and research novelty
have distinct review gates.

## Negative controls

The conformance corpus must cover:

- omitted source constructs, inconsistent resolution routes, and unsupported
  invariants accepted without interpretation;
- receive-then-use without ownership, duplicated job control, and collecting
  a result twice;
- stale input/context, slot-generation collision, duplicate response, and
  cancellation followed by a late response;
- refunding dispatched work, retry past a declared bound, or unknown usage
  recorded as zero;
- a wrong Lean conclusion, changed imported definition, prohibited axiom, or
  draft lemma promoted to accepted evidence;
- invalid counterexamples, failed proof search called refutation, and bounded
  instances promoted to a universal theorem;
- a property attached to another behavior, a disconnected bad state called
  reachable, and incomplete enumeration called exhaustive;
- a protocol optimization that skips required work, always fails cheaply, or
  adds deadlock on an admissible input.

Countermodels for necessity must satisfy the retained hypotheses and falsify
the conclusion. Formation rejection alone is not enough.

## Compatibility

Keep `behavior-core-v1` as versioned regression evidence. Specify
`behavior-core-v2` initial ownership, dynamic state/steps, source maps, decoding
and unsupported-version policy before changing defaults. Runtime event records
and adapter messages have their own versions; they are not implicitly accepted
behavior artifacts. Semantic changes invalidate dependent evidence unless a
specific migration establishes the required relation.

Acceptance would extend ADR 0004's exploration-only promoted path with scoped
local execution. A maintainer must record that decision and its trust-boundary
changes explicitly. Historical postponed RFCs remain postponed unless
separately changed. This Draft does not override current architecture.

## Alternatives

- **Lean/Python library only:** fastest useful baseline. Retain it if the new
  surface adds ceremony without measurable composition or usability benefit.
- **General-purpose application language now:** too many runtime and library
  obligations before the initial use cases are validated.
- **Agent workflow service first:** can coordinate tools but leaves NMLT's
  source and resource semantics disconnected from execution.
- **Finish verified compilation first:** would delay feedback from real users;
  scoped implementation evidence is useful while correspondence remains open.

## Risks and unresolved questions

Grammar, type/effect rules, compatible incremental Lean tooling, and the exact
abstraction/trace-validation interface require design review. Job cancellation
and recovery depend on concrete host contracts. The value of affine workflow
control beyond ordinary libraries is unmeasured. None of these uncertainties
is resolved by accepting a broad roadmap.

Before acceptance, reviewers must resolve the smallest syntax/type slice,
outcome and accounting transition table, target-comparison contract, finite
identity bounds, invariant acceptance methods, and ADR change. Any unresolved
part remains explicitly deferred rather than implied by a generic success
record.

## Implementation plan

1. R0: run three deterministic baseline contracts, preserve their limitations,
   establish checker compatibility, and prepare all three user pilots.
2. Review this Draft using concrete examples and its unresolved decisions.
   Record disposition without claiming implementation conformance.
3. R1/M1--M3: unify state/steps, obtain decoded initial execution witnesses,
   and prove finite-path ownership with affine continuation.
4. R2: implement the smallest reviewed local interpreter, common source
   facilities, and typed adapter/event protocol. Compare its common finite
   operations with Lean under explicit bounds.
5. R3--R4: integrate pinned Lean checking, explicit statement revisions,
   invariant/counterexample paths, and the three discovery experiments.
6. R5: use the [pilot protocol](../docs/r0-pilot-protocol.md) and frozen
   comparisons. Every reported accepted artifact must recheck; every required
   invalid control must be distinguished. All three audiences must complete a
   useful workflow before an all-three alpha claim.

The acceptance gate for this RFC is a reviewed semantic contract and recorded
disposition. The implementation gate is runnable conformance evidence. The
mathematical gate is the exact required Lean theorem and witness corpus. These
are separate milestones.
