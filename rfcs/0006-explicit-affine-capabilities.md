# RFC 0006: Explicit affine capabilities v1

- Status: Under review
- Authors: NMLT project
- Created: 2026-07-18
- Mathematical-core backlog: `NMLT-P1-103`

## Summary

Select an explicit affine capability discipline for NMLT v1. Capability
contexts admit exchange but no contraction. Weakening is represented by an
explicit, auditable `discard` operation rather than an invisible typing rule.
Every capability has a unique runtime identity, protocol, phase, and binding
key. An operation consumes one capability and may return one uniquely derived
capability in a new phase; parallel composition must partition capability
contexts.

This gives the provider-effect slice the required “at most once” authority
property without introducing borrowing, aliasing, or general linear logic into
the first kernel.

## Motivation

The provider-attempt model has a safety-critical distinction between knowing
that a call may be attempted and possessing authority to make that attempt.
An unrestricted value cannot express that distinction: it can be copied into
two branches or two components. A fully linear discipline would forbid
abandoning an attempt even during cancellation or shutdown. V1 therefore uses
affine authority—use at most once—with explicit discard so abandonment remains
visible in source and evidence.

Oxide gives a syntactic precedent for substructural ownership judgments and
progress/preservation proofs, while RustBelt shows why the trusted boundary of
an ownership discipline must include the semantics of privileged operations,
not only the surface type checker. NMLT deliberately selects a much smaller
system: owned tokens without references or borrowing.

## Goals

- Prevent duplication and concurrent reuse of provider authority.
- Make cancellation and abandonment explicit.
- Represent protocol phase changes as capability type changes.
- Keep capabilities out of unrestricted values and serializable state.
- Define context rules that are implementable without general proof search.
- Bind every external operation and terminal record to one capability identity.

## Non-goals

- Borrowing, lifetimes, fractional permissions, or shared mutable references.
- Proving that a capability will eventually be used.
- Treating capability possession as evidence that an external effect occurred.
- Inferring authority from a state predicate or ambient module privilege.
- Copying capabilities through serialization or runtime adapters.

## 1. Capability types and identities

A capability type is:

```text
Cap<Q, P, K>
```

where `Q` names a protocol, `P` is its current phase, and `K` is a type-level
binding key such as an attempt identity or source digest. A runtime capability
is an opaque tuple:

```text
<cap_id, protocol = Q, phase = P, binding = k, parent_id?>
```

`cap_id` is globally unique within an execution/evidence graph. It is never
available to ordinary equality, hashing, display, JSON serialization, or pure
pattern matching. Trusted runtime and evidence code may record it as an opaque
identifier.

The unrestricted type universe has no constructor embedding `Cap`. Capability
variables occur only in `Delta`:

```text
Delta ::= empty | Delta, k : Cap<Q,P,K>
```

Contexts are finite maps with distinct variables and distinct live capability
identities.

## 2. Structural discipline

Exchange is admissible:

```text
Gamma; Delta1, k:A, j:B, Delta2 |- c : C
------------------------------------------------ EXCHANGE
Gamma; Delta1, j:B, k:A, Delta2 |- c : C
```

There is no contraction rule:

```text
Gamma; Delta, k:A, k':A |- c : C
---------------------------------  FORBIDDEN
Gamma; Delta, k:A |- c[k/k',k] : C
```

There is no silent weakening rule. Affine disposal is a term:

```text
k : Cap<Q,P,K> in Delta
Gamma |- r : DiscardReason<Q,P>
Gamma; Delta - {k} |- c : C => Delta'
---------------------------------------------------------------- CAP-DISCARD
Gamma; Delta |- discard k reason r ; c : C => Delta'
```

Elaboration emits `CapabilityDiscarded(cap_id, Q, P, K, r, source_span)`.
This event says authority was abandoned; it says nothing about whether an
external system acted independently.

Because explicit discard makes every linear resource discardable, the
programmer-facing discipline is affine. Keeping the core context exact avoids
silently losing authority at branches and action boundaries.

## 3. Protocol declarations

A protocol declares phases and operations. An operation signature is one of:

```text
op : Cap<Q,P,K> * A -[g]-> B * Cap<Q,P',K'>
terminal op : Cap<Q,P,K> * A -[g]-> Outcome<B>
```

The first form consumes the input capability and creates exactly one child
capability in phase `P'`. Its child identity is fresh and its `parent_id` is
the consumed identity. The second form consumes authority and returns no
capability.

```text
k : Cap<Q,P,K> in Delta
op : Cap<Q,P,K> * A -[g]-> B * Cap<Q,P',K'>
Gamma |- e : A
Gamma,y:B; Delta - {k}, k':Cap<Q,P',K'> |- c : C => Delta'
---------------------------------------------------------------- CAP-INVOKE
Gamma; Delta |- invoke op with k(e) as (y,k') in c : C
  => Delta'

k : Cap<Q,P,K> in Delta
terminal op : Cap<Q,P,K> * A -[g]-> Outcome<B>
Gamma |- e : A
Gamma,y:Outcome<B>; Delta - {k} |- c : C => Delta'
---------------------------------------------------------------- CAP-FINISH
Gamma; Delta |- finish op with k(e) as y in c : C
  => Delta'
```

The operation grade is combined with the continuation grade by the declared
grade algebra. An untrusted adapter cannot mint the returned capability; the
trusted operation boundary creates and records it.

## 4. Provider-attempt instance

The v1 benchmark uses:

```text
protocol ProviderAttempt<AttemptId> {
  phase Ready;

  terminal dispatch :
    Cap<ProviderAttempt, Ready, AttemptId> * Request
      -[provider_calls(1)]-> DispatchOutcome;
}

DispatchOutcome =
    Committed(ProviderEvidence)
  | Rejected(ProviderRejection)
  | Indeterminate(AttemptReceipt)
```

All three outcomes consume `Ready` authority. `Indeterminate` is not a retry
capability. A retry policy may request a newly minted
`Cap<ProviderAttempt,Ready,NewAttemptId>` only through a separate transition
that proves its policy preconditions and binds a new attempt identity. This
prevents an ambiguous response from silently restoring the authority that was
just consumed.

Evidence of `Committed` must bind both the attempt identity and consumed
`cap_id`. Possession or consumption of the capability alone never establishes
that the provider committed the effect.

## 5. Branches, choice, and composition

Both branches of a conditional or finite nondeterministic choice must produce
the same residual context:

```text
Gamma; Delta |- c1 : C => Delta'
Gamma; Delta |- c2 : C => Delta'
--------------------------------- CAP-BRANCH
Gamma; Delta |- choose(c1,c2) : C => Delta'
```

If only one branch invokes a capability, the other must either invoke it,
return it through a common sum-typed protocol result, or explicitly discard
it. This rule prevents later availability from depending on an unrecorded
branch.

Parallel composition partitions authority:

```text
Delta = Delta1 disjoint_union Delta2
Gamma; Delta1 |- B1 : Behavior => Delta1'
Gamma; Delta2 |- B2 : Behavior => Delta2'
---------------------------------------------------------------- CAP-PAR
Gamma; Delta |- B1 || B2 : Behavior
  => Delta1' disjoint_union Delta2'
```

Transfer between components is a synchronized action that removes the token
from the sender context and introduces the same identity in the receiver
context atomically. Fan-out has no typing rule.

## 6. Dynamic invariant

Let `live(rho)` be the finite map from live capability identities to their
protocol metadata. A store realizes `Delta`, written `rho : Delta`, iff:

1. every variable in `Delta` resolves to one entry in `live(rho)`;
2. no two variables resolve to the same identity;
3. every entry has the protocol, phase, and binding declared by its type;
4. every non-root identity has one recorded parent;
5. a consumed or discarded identity is never live.

For one operational step define `desc(c)` as the live child identities whose
provenance chain includes `c`. Capability provenance uniqueness is:

```text
for every consumed input identity c,
cardinality(desc(c) intersect newly_live(rho')) <= 1
```

For parallel composition, the live identity sets of the component stores
remain disjoint except during one atomic transfer, where ownership moves and
never overlaps.

## 7. Required metatheorems

### No duplication

```text
rho : Delta
Gamma; Delta |- c : C => Delta'
<c,rho> ⇓ <result,rho'>
--------------------------------
rho' : Delta'
and every live provenance has at most one live affine descendant
```

### No authority fabrication

Every live non-root capability in `rho'` is the unique child of a capability
consumed by the same trusted operation step. Root minting is confined to named
trusted constructors whose policy obligations appear in evidence.

### Partition preservation

A well-typed parallel step preserves disjoint live identity sets. Transfer is
linearizable to one ownership move.

These are exact proof obligations. V1 does not claim “exactly once” execution:
explicit discard and external uncertainty mean the valid theorem is at-most-
once use of NMLT authority.

## Evidence consequences

- Type checking may establish only that source follows the affine discipline.
- Operation evidence names the consumed capability identity and binding key.
- Discard evidence is distinct from operation evidence.
- `Indeterminate` remains indeterminate even though authority was consumed.
- An adapter capable of minting, cloning, or relabeling tokens is inside the
  trusted computing base for capability claims.
- A no-duplication claim requires a checked proof of the type and operational
  rules, not merely Rust ownership in the implementation.

## Negative controls

The implementation and mechanization must reject or refute:

- using one capability variable in two sequential invocations;
- assigning one capability to both sides of `CAP-PAR`;
- copying a capability through a record, collection, closure, or serializer;
- reusing the parent after `CAP-INVOKE`;
- treating `Indeterminate` as an implicit retry capability;
- branches with unequal residual contexts and no explicit discard;
- minting a child without consuming a parent or invoking a root policy;
- two live children with the same consumed ancestor;
- reporting capability consumption as proof of external commitment.

## Compatibility

Capability protocol, phase, binding key, and operation identity enter typed IR
and evidence identity. Changing any of them is a semantic compatibility break.
Adding borrowing or shared permissions later requires a distinct capability
kind and must not reinterpret `Cap` values accepted under this RFC.

## Alternatives

- **Unrestricted opaque tokens:** nominal opacity does not prevent copying.
- **Implicit affine weakening:** sound for at-most-once use but hides
  abandonment at precisely the boundary evidence must explain.
- **Strict linear values:** makes cancellation and shutdown artificially
  difficult and still does not prove the external effect happened.
- **Rust borrowing model:** powerful but much larger than the provider slice;
  Oxide and RustBelt demonstrate the metatheoretic cost.
- **Fractional permissions:** useful for shared reads, unnecessary for v1
  external-effect authority.

## Risks and unresolved questions

- Protocol phase changes across nondeterministic responses may require an
  explicit sum of capability contexts rather than identical branch outputs.
- Opaque identity generation and evidence serialization must be deterministic
  enough for replay while preventing identity collision.
- Revocation is not yet modeled; a revoked but locally live capability needs a
  checked external policy response, not type-level time travel.
- Cross-process transfer will require an authenticated runtime envelope and a
  stronger threat model.

## Research basis

- [Oxide: The Essence of Rust](https://arxiv.org/abs/1903.00982) presents a
  source-level substructural ownership judgment with syntactic progress and
  preservation proofs.
- [RustBelt](https://doi.org/10.1145/3158154) gives a machine-checked account of
  an ownership-based core extended by privileged libraries, motivating an
  explicit trusted operation boundary.
- [A graded dependent type system with a usage-aware
  semantics](https://arxiv.org/abs/2011.04070) demonstrates that resource
  accounting needs a usage-aware semantics to justify single-pointer and
  non-interference results.

These sources establish relevant techniques, not the soundness or novelty of
NMLT's combination.

## Implementation plan

1. Mechanize exact capability contexts, explicit discard, and operation rules.
2. Prove no duplication and provenance uniqueness for the sequential core.
3. Prove disjoint partition preservation for parallel composition.
4. Implement the provider protocol and each duplication/retry negative control.
5. Bind operation/discard events into evidence manifests.
6. Postpone borrowing, revocation, and cross-process transfer until separate
   RFCs carry their own metatheory and threats.
