# RFC 0015: Unified resource-bearing behavior

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-06
- Tracking issue: not assigned
- Scope: R1/M1 semantic increment; M2/M3 constraints remain explicit follow-up

## Summary

Introduce one resource-bearing behavior whose state combines control state
with a shared authority world, and whose transition applies a declared
resource effect exactly once. Binary composition preserves open interface
metadata and the identities of the atomic actors. Relate this new behavior to
the existing control and dynamic product relations without changing
`behavior-core-v1` or claiming that source-level affine continuation is already
supported. This is narrower than [Draft RFC 0014](0014-executable-workflow-profile.md),
whose runtime proposal remains Draft.

## Motivation

The current [control behavior](../mechanization/lean/NMLT/Behavior/ResourceBehavior.lean)
has initialization, observations, and resource profiles, while
[ResourceWorld](../mechanization/lean/NMLT/Behavior/ResourceWorld.lean) defines
actual ownership effects and a separate binary product state. The dynamic
layer lacks its own behavior initializer and observation. A conditional
dynamic lifting result does not by itself construct an initial execution.

Simply putting an authority world inside both children of a Cartesian product
would duplicate the world. Giving the entire product one synthetic actor would
lose the original owners. Applying each child's completed local step before
composition would reject an open transfer before an outer component could
synchronize with it. These are semantic design problems, not extra fields to
add to an artifact.

The current source compiler also treats `system.capabilities` as both the
known capability namespace and initial ownership. An input action's receive
binding is not available to a later source action. That is an M2/M3 boundary
to address deliberately after the unified semantics is checked.

## Goals

- One state and transition relation for the promoted dynamic behavior.
- Reuse the existing local and synchronized authority-effect rules.
- Preserve isolated left and right action metadata through binary products.
- Preserve atomic actor identities and apply effects once under nesting.
- Supply a precise relation to the legacy dynamic pair and explicit
  initialization/observation obligations.
- Support finite Bool/Unit/enum interpretation consistently in Lean and Rust.

## Non-goals

- A local job runtime, host effects, cancellation protocol, or new invariant
  syntax from RFC 0014.
- A committed `behavior-core-v2` wire format or a source-language continuation
  extension in this first increment.
- Arbitrary multiparty rendezvous, associativity without conditions, fairness,
  liveness, verified compilation, or production assurance.
- Treating the existing static profile summary as a complete dynamic effect.

## Guide-level explanation

Consider a sender that initially owns a permit, a receiver, and a later local
receiver action. A synchronized send/receive step must update both control
states and move the one permit in the same shared world. A later dynamic
receiver step can consume that permit only if the preceding world grants it
ownership. The sender cannot consume the permit after transferring it.

Now compose an intermediate component with a receiver while leaving its input
port open. An outer sender must still be able to connect to that input. The
inner product therefore preserves the open action's control transition,
direction, payload, resource descriptor, and original actor. It does not
perform an isolated transfer while constructing that open interface.

A completed synchronization inside the intermediate product is different: it
already describes an internal joint effect. An enclosing product inherits
that effect rather than reinterpreting it as a local action of a synthetic
product owner or applying it twice. Connecting that completed synchronization
to a third actor is outside this binary rendezvous profile.

These examples describe the mathematical target. A hand-constructed Lean
example does not establish a decoded artifact witness or support for the same
program in current `.nmlt` source.

## Reference semantics

### Control transitions and deferred effects

The new `ResourceDynamics` layer retains the existing control `Behavior` as
the control component. Each action also carries an effect descriptor whose
resource summary agrees with the control action's profile. An atomic effect
contains its leaf actor and profile. A synchronized effect contains the two
atomic participants and their profiles.

A full dynamic state contains exactly one control state and one
`AuthorityWorld`. A completed step requires both the control transition and
the effect relation on the before/after worlds. Atomic completed effects use
`LocalStep`; synchronized effects use `SyncStep`. The existing requirements,
affine disjointness, fresh reception, distinct synchronized actors, exact
transfer matching, and unaffected-authority frame conditions still apply.

An open control transition with a pending transfer is not an independently
completed dynamic local step. It remains available for composition. The final
behavior transition applies the resulting effect relation once to its shared
world. This distinction permits open composition without making unmatched
transfer or receive an executable local operation.

### Binary composition

Composition combines control transitions and effect descriptors, not two
already-completed child dynamic transitions with independent worlds.

- An isolated left action keeps the left action's hidden classification,
  direction, payload, actor provenance, and effect.
- An isolated right action keeps the same metadata from the right action.
- A connected synchronization has the combined control step and joint effect;
  it is internal with no remaining external payload obligation.
- An inherited internal synchronization remains that joint effect.

Connections must meet the reviewed direction, payload, visibility, and
resource-compatibility conditions. Only atomic open endpoints participate in
a new binary synchronization. A completed synchronized action is not silently
flattened into a multiparty boundary event. Wiring membership, rather than
whether a peer happens to be enabled, determines whether an action is isolated.

No composite actor is invented. Owner identities refer to the original
participants across nested control products. This does not imply unrestricted
associativity or allow repeated occurrences of one component to alias without
an explicit instance-identity rule.

### Initialization and observation

Initialization requires both the control initializer and an explicit
initial-world predicate. The dynamic behavior owns this combined initializer;
it is not an arbitrary world supplied only when a theorem is invoked.
Composition conjoins the component predicates over the same world. A
component predicate must allow authority held by its peers; requiring every
nonlocal capability to be vacant would generally make that conjunction
unsatisfiable. Initial execution examples must supply an actual witness.

The semantic observation pairs the selected control observation with the
full authority world. A product returns both control observations and the one
shared world. A future public display may choose to omit ownership, but such
a display does not make changes to authority stuttering in this semantics.

For a two-atomic-component case, prove the relationship between the new product
step and the legacy `ResourceWorld.ProductStep` after the explicit state
conversion. The old dynamic pair has no initializer or observation, so a
step-relation correspondence cannot be advertised as a pre-existing
initialization or observation theorem. State those additional preservation
results separately.

### Refinement and hiding

Maintain the exact world-enabledness requirements needed by dynamic
refinement. The current `WorldResourceRefines` requires both directions of
the requirement implication; static profile refinement alone is insufficient.
Any owner renaming across a concrete/abstract pair must be explicit. A control
state map cannot silently rename or identify owners.

Hidden weak matching must establish unchanged mapped control state **and**
unchanged authority. The summary produced by `ResourceProfile.parallel`
discharges internal transfers/receives and can therefore look empty even when
the joint dynamic effect moves ownership. Do not derive world preservation
from that summary alone. A dynamic effect-preservation premise or a direct
full-state agreement proof is required.

Formation conditions and hypotheses actually used by a lifting theorem are
documented separately. Rejection by formation does not prove that a rule is
necessary for the theorem after that hypothesis is removed.

### Finite interpretation

The Rust reference explorer should interpret the same finite Bool, Unit, and
declared enum values, closed initializers, guards, simultaneous updates, and
observations as Lean. Compare values by their exact type and constructor;
an enum constructor from a different enum is not interchangeable.

Action updates read the frozen pre-state, including both sides of a
synchronization. Exploration distinguishes a complete finite search from a
search stopped by a bound. Its current `assurance: none` remains unchanged.
Representation parity and comparison tests are implementation evidence, not
a compiler or interpreter correctness theorem.

## Evidence consequences

A checked M1 increment can establish the new behavior definitions and exact
composition/projection/refinement results that have proofs. Its examples must
exhibit real initial states and actual steps; an implication with an empty
antecedent is not the execution demonstration.

The existence of a dynamic receive-then-use example in Lean does not make the
current source compiler accept received names in later actions. A witness
derived from one v1 artifact under an explicit interpretation policy is scoped
to that artifact and policy; it does not establish the proposed v2 schema.

Rust remains outside the Lean proof boundary. Source hashes identify bytes
without proving faithful translation. The no-placeholder and transitive axiom
policies remain unchanged, and the new normative module and named theorem
results must enter the existing checking/export inventory before promotion.

## Negative controls

The M1 corpus needs positive and negative evidence for:

- a local consume before acquisition and a sender consume after transfer;
- an unmatched local transfer or receive;
- duplicated or fabricated ownership, and unexplained changes to unaffected
  capabilities;
- a resource-bearing hidden step claimed to be full-state stutter;
- a right-side hidden action, direction, or payload lost by composition;
- an inner open port that cannot be connected by an outer product;
- a completed inner synchronization re-applied or assigned a synthetic owner;
- an unsupported connection to a completed synchronized action;
- a malformed initial world, disabled claimed initial action, and unrelated
  initial control values;
- cross-type enum equality, missing finite values, and sequential evaluation
  of simultaneous updates.

A necessity countermodel must satisfy the remaining assumptions and falsify
the fixed conclusion. A runtime exception or a formation error alone is not
such a countermodel. Bounds and denominators accompany finite controls.

## Compatibility

Leave v1 serialization, schema dispatch, source restrictions, and canonical
fixtures unchanged. Keep the legacy definitions available for their exact
existing claims and prove the scoped relation to the new layer. Corrected
open-interface metadata belongs to the new product; silently changing the
old product would change the subject of existing evidence.

This RFC does not change the accepted lexical rules or source identity rules
in RFCs 0003/0004, and does not extend the retained ordinary typed-core claims
of RFC 0013 to behavioral execution. It stays within ADR 0004's language and
mathematics direction. The broader runtime extension in RFC 0014 requires its
own disposition.

## Alternatives

- **Duplicate the world in each child state:** obscures the shared update and
  introduces consistency obligations instead of one ownership state.
- **Treat the product as one owner:** loses the identity needed to move
  authority between its original participants.
- **Execute children before composing:** prematurely rejects open transfers
  and can apply an inherited synchronization effect twice.
- **Rebuild legacy semantics in place:** risks changing v1 evidence before a
  correspondence result exists.
- **Freeze v2 before M1 compiles:** commits producers and consumers to an
  unvalidated representation. Establish the semantics first.

## Risks and unresolved questions

### Next M2/M3 contract constraints; no committed wire format

The following guide the next design review but are not implemented by this
increment:

1. Preserve `system.capabilities` as declared initial ownership. Derive a
   separate known-capability/type namespace from those declarations and the
   typed bindings of all input actions. Reject inconsistent types for one
   nominal capability. Being known must not grant initial ownership.
2. Allow later source actions to consume or retransmit a known received slot
   only with the dynamic ownership condition enforced by the step relation.
   Static source acceptance alone does not prove that the action is reachable
   or enabled. Existing `require expr` remains a Boolean guard; a capability
   requirement needs a separately specified source form or remains
   artifact-only.
3. Specify initial authority for a selected composition using its actual leaf
   instances. It must agree with the declared initial owners; other systems
   in a program, including alternative refinement models, cannot contribute
   ownership accidentally. Define coverage and vacancy explicitly.
4. Derive initial control values from the validated closed ASTs and prove
   finite-domain membership. Sentinel indices are not valid initial states or
   fresh capability names.
5. Instantiate execution independently of a concrete/abstract refinement
   application. Checking a path should not require inventing a dummy
   refinement pair.
6. Specify a finite witness format binding the artifact, selected composition,
   action provenance, and control/world states. Lean must establish the
   initializer and every actual step; an asserted post-state is not trusted.
   A path that starts at an arbitrary valid state is not a reachability
   witness. Failure to find a witness remains inconclusive.
7. Preserve v1 byte-for-byte, assign an explicit v2 schema, and make old
   decoders reject it. Agree on canonical encoding, additional validation,
   compatibility, and claim scope before a producer emits it.

General component-instance naming, owner transport under refinement, exact
v2 fields, and source binding lifetime remain open design decisions. They
must be resolved before the corresponding M2/M3 feature is promoted.

## Implementation plan

1. Check in the new dynamic behavior/effect definitions with actual local and
   synchronized examples, preserving all v1 definitions and bytes.
2. Prove the scoped legacy dynamic-step relation, initializer/observation
   obligations, and the intended dynamic refinement result. Record the exact
   scope of every completed theorem.
3. Check nested examples with inherited synchronization and an open boundary
   connected at the outer product. Include right-side metadata controls and
   full-world hidden-step controls.
4. Bring finite Rust values into Bool/Unit/enum parity and cross-check the
   shared corpus while preserving `assurance: none`.
5. Review these artifacts and record the M1 status accurately. R1 remains in
   progress until its complete M1--M3 exit gates are met.
6. Only then settle the v2/continuation contract, produce decoded initial
   execution and receive-then-use/transfer paths, and prove ownership/no
   fabrication over finite paths for M2/M3.

Under-review status records the proposed contract, not acceptance or theorem
completion. Runnable fixtures, checked proofs, unchanged-v1 reproduction, and
recorded disposition remain separate gates.
