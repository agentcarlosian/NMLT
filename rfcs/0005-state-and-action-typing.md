# RFC 0005: State and action typing v1

- Status: Under review
- Authors: Carlosian <carlosian@agentmail.to>
- Created: 2026-07-18
- Mathematical-core backlog: `NMLT-P1-102`

## Summary

Define state as a finite record of unrestricted data and define an action as a
typed, finitely branching relation that reads one pre-state and constructs one
post-state by simultaneous, explicit updates. The action judgment records its
write set, capability-context transition, output type, and resource grade.
Fields outside the write set are copied by a generated frame equation; no
command has implicit write authority.

This is a candidate kernel contract. Its syntax and the stated metatheorems are
not yet accepted as proven merely because this RFC is precise.

## Motivation

RFC 0001 fixes the semantic shape of an action but leaves several choices open:

- whether an update reads a changing store or a frozen pre-state;
- whether an omitted field may change;
- how branches combine write sets and capability contexts;
- whether state may contain capabilities;
- which facts elaboration must make explicit.

Those choices determine whether the provider-attempt mutants can be rejected
for the intended reasons. They also determine the statement of preservation
and frame soundness.

## Goals

- Give syntax-directed state and action judgments.
- Make pre-state reads and simultaneous post-state construction explicit.
- Make every write target and frame equality checkable.
- Keep nondeterminism relational while retaining a finite executable fragment.
- Thread the selected capability discipline and grade algebra through actions.
- State exact preservation, frame, and blocked-state obligations.

## Non-goals

- General references, heap aliasing, or mutable closures.
- Capabilities stored inside ordinary state values in v1.
- Unrestricted recursion, exceptions, or partial pure functions.
- An assertion that the rules are sound before mechanization.

## 1. Static domains

Let `DataType` contain total, equality-decidable v1 data types: finite sums,
finite products, finite records, bounded integers, booleans, finite sets and
maps, and total functions over finite domains. `Cap(q, p, k)` is not a
`DataType`; it lives only in a substructural context.

A state signature is a finite map from distinct field names to data types:

```text
Sigma = x_1 : A_1, ..., x_n : A_n

State(Sigma) = { x_1 : [[A_1]], ..., x_n : [[A_n]] }
```

Contexts have separate purposes:

```text
Gamma   unrestricted values and total pure functions
Sigma   state fields, available only through `pre.field`
Delta   capability variables; exchange is allowed, contraction is not
```

The v1 state-formation rules are:

```text
dom(Sigma) has no duplicates
for every (x : A) in Sigma, Gamma |- A : DataType
--------------------------------------------------- STATE-FORM
Gamma |- record Sigma : StateType

Gamma |- record Sigma : StateType
for every (x : A) in Sigma, Gamma |- v_x : A
--------------------------------------------------- STATE-INTRO
Gamma |- { x = v_x }_(x in dom(Sigma)) : State(Sigma)
```

Capabilities, open file handles, solver sessions, implementation pointers, and
functions with effects fail `STATE-FORM`. A system may retain a capability
between actions in its separate runtime capability store; ordinary state
serialization never duplicates it.

## 2. Pure expressions

Pure expressions read a frozen pre-state and the current input:

```text
Gamma; Sigma; input : I |- e : A
```

Representative rules are conventional. The important state read is:

```text
(x : A) in Sigma
------------------------------ E-PRE
Gamma; Sigma; input : I |- pre.x : A
```

There is no expression form for `post.x`, a capability variable, an external
call, or a state write. Pure evaluation is total. A failed lookup or division
is represented by an explicit sum or option value rather than evaluator
failure.

## 3. Action bodies

The core action body has the following relevant forms:

```text
c ::= skip
    | require e
    | update x := e
    | emit e
    | let y = e in c
    | let y = choose e in c
    | c ; c
    | if e then c else c
    | invoke op with k(e) as (y, k') in c
    | finish op with k(e) as y in c
    | discard k reason r ; c
```

`choose e` ranges over a finite, nonempty collection. `invoke` and `finish`
are the only capability-consuming operation forms and are specified by RFC
0006. Updates in one action are simultaneous: every right-hand side reads
`pre`, never a partially constructed post-state.

The principal judgment is:

```text
Gamma; Sigma; input : I; Delta
  |- c : Body<O, G> => <W, g, Delta'>
```

It means that `c` may emit a value of type `O`, writes exactly the fields in
`W`, accumulates grade expression `g`, and transforms the capability context
from `Delta` to `Delta'`.

### Pure and state rules

```text
-------------------------------------------------------------- C-SKIP
Gamma; Sigma; input:I; Delta |- skip : Body<Unit,G>
  => <empty, epsilon, Delta>

Gamma; Sigma; input:I |- e : Bool
-------------------------------------------------------------- C-REQUIRE
Gamma; Sigma; input:I; Delta |- require e : Body<Unit,G>
  => <empty, epsilon, Delta>

(x : A) in Sigma    Gamma; Sigma; input:I |- e : A
-------------------------------------------------------------- C-UPDATE
Gamma; Sigma; input:I; Delta |- update x := e : Body<Unit,G>
  => <{x}, epsilon, Delta>

Gamma; Sigma; input:I |- e : O
-------------------------------------------------------------- C-EMIT
Gamma; Sigma; input:I; Delta |- emit e : Body<O,G>
  => <empty, epsilon, Delta>
```

An update to a missing field fails `C-UPDATE`. A field name may occur at most
once along an executed sequential path; the surface diagnostic identifies both
assignments. This makes the simultaneous-update meaning independent of source
order.

### Sequential and branch rules

```text
Gamma; Sigma; input:I; Delta  |- c1 : Body<O1,G> => <W1,g1,Delta1>
Gamma,y:O1; Sigma; input:I; Delta1 |- c2 : Body<O2,G> => <W2,g2,Delta2>
W1 intersect W2 = empty
-------------------------------------------------------------------------- C-SEQ
Gamma; Sigma; input:I; Delta |- c1 ; c2 : Body<O2,G>
  => <W1 union W2, g1 tensor g2, Delta2>

Gamma; Sigma; input:I |- e : Bool
Gamma; Sigma; input:I; Delta |- c1 : Body<O,G> => <W1,g1,Delta'>
Gamma; Sigma; input:I; Delta |- c2 : Body<O,G> => <W2,g2,Delta'>
-------------------------------------------------------------------------- C-IF
Gamma; Sigma; input:I; Delta |- if e then c1 else c2 : Body<O,G>
  => <W1 union W2, join_G(g1,g2), Delta'>
```

Both branches must return the same capability context. If one branch should
abandon a capability, source must contain an explicit `discard`; it is not
silently forgotten at the join. A field written only in one branch is framed
to its pre-state value in the other branch. `join_G` is supplied by the grade
algebra and is not assumed to equal `tensor`.

Finite nondeterministic choice uses the same residual-context and grade-join
condition as `C-IF`. This prevents the availability of a capability after the
choice from depending on a hidden explorer decision.

## 4. Action formation and elaboration

An action declaration fixes its input, output, and declared write set:

```text
Gamma |- Sigma : StateType
Gamma |- I : DataType       Gamma |- O : DataType
Gamma |- G : GradeAlg
Gamma; Sigma; input:I; Delta_in |- c : Body<O,G>
  => <W_actual,g,Delta_out>
W_actual = W_declared
-------------------------------------------------------------- ACTION-FORM
Gamma; Delta_in |- action c
  : Action<State(Sigma),I,O,G,W_declared,g> => Delta_out
```

Elaboration produces a relation over complete states:

```text
[[c]] subseteq
  State(Sigma) x I x CapStore(Delta_in)
  x State(Sigma) x O x |G| x CapStore(Delta_out)
```

For each related tuple `(s,i,rho,s',o,g,rho')`:

```text
x in W_actual      => s'.x is the value of the unique executed update to x
x notin W_actual   => s'.x = s.x
```

The second line is emitted into the untyped behavioral core as a frame
equality. It is not an optimizer convention or a verifier default.

`require false` yields no related successor. This is a blocked action, not a
runtime type error. A behavior is blocked in state `s` for input `i` when no
declared action has a successor and the environment does not select the
distinguished semantic stutter.

## 5. Dynamic consistency

A runtime capability store `rho` realizes `Delta`, written `rho : Delta`, when
every variable in `Delta` names exactly one live capability identity of the
declared protocol and phase, and no identity appears twice.

The executable big-step relation is:

```text
<c, s, i, rho> ⇓ <s', o, g, rho'>
```

It is nondeterministic only at explicit finite `choose` sites and operation
responses. Expression evaluation, update construction, and grade calculation
are deterministic for fixed choices and responses.

## 6. Required metatheorems

Acceptance requires machine-checked versions of the following statements.

### Action preservation

```text
Gamma; Sigma; input:I; Delta |- c : Body<O,G> => <W,g,Delta'>
Gamma |- s : State(Sigma)    Gamma |- i : I    rho : Delta
<c,s,i,rho> ⇓ <s',o,g_actual,rho'>
----------------------------------------------------------------
Gamma |- s' : State(Sigma)   Gamma |- o : O    rho' : Delta'
and g_actual is admitted by g
```

### Frame soundness

```text
well_typed(c,W) and <c,s,i,rho> ⇓ <s',o,g,rho'>
implies for every x notin W, s'.x = s.x
```

### Capability uniqueness

If `rho : Delta` and a well-typed body steps to `rho' : Delta'`, then no live
capability identity occurs more than once in `rho'`. RFC 0006 strengthens this
to provenance uniqueness for transformed capabilities.

### Progress characterization

A well-typed body is either a value-producing terminal form, can take an
internal evaluation step, is waiting at a declared external operation, or is
blocked at a false guard or empty semantic relation. “Blocked” remains a
first-class outcome; v1 does not assert deadlock freedom from typing.

## Evidence consequences

A successful derivation can support only a type-safety claim bound to the
exact typing rules and elaborator version. Frame soundness and capability
uniqueness require checked metatheory before they may be reported as proved.
Exploring the resulting finite relation is still bounded model-checking
evidence, not proof of unbounded temporal properties.

## Negative controls

The frontend, elaborator, or mechanization must reject or refute:

- a duplicate state field;
- a capability nested in a state record;
- an update of an undeclared field;
- two updates to one field along one sequential path;
- an update RHS that observes a preceding update rather than `pre`;
- a branch that exposes different residual capability contexts;
- a nondeterministic empty `choose` domain accepted as progress;
- a post-state change outside the inferred write set;
- an effectful or partial expression admitted as pure.

## Compatibility

Separating event output from state observation is normative for these rules;
RFC 0007 supplies the corresponding behavior signature. A future heap or
borrow system will require a new RFC and cannot reinterpret a v1 state field as
an implicit reference.

## Alternatives

- **Sequential mutable updates:** familiar, but makes source order semantic and
  complicates correspondence with next-state relations.
- **Implicit frames:** concise, but makes a missing update indistinguishable
  from permission to havoc a field at the core boundary.
- **Capabilities in state:** convenient persistence, but ordinary copying,
  serialization, and refinement mappings would need a full ownership model.
- **One monadic effect judgment:** expressive, but hides the exact write and
  capability facts needed by the first benchmark.

## Risks and unresolved questions

- `join_G` needs a concrete interface for partial orders without finite joins.
- Requiring identical residual contexts at every branch may need scoped sum
  types for protocols whose branches intentionally return different phases.
- Finite collections and bounded integers make execution decidable but require
  explicit abstraction when modeling unbounded mathematics.
- The relationship between blocked inputs and open-system input receptiveness
  belongs to the composition contract, not this local judgment.

## Implementation plan

1. Mechanize state formation, pure expressions, and command contexts.
2. Prove action preservation and frame soundness.
3. Add explicit affine capability rules from RFC 0006.
4. Elaborate typed actions into the transition IR without implicit writes.
5. Run the provider reference and all invalid-update/frame negative controls.
6. Accept this RFC only when no `sorry`, unrecorded axiom, or unchecked
   translation is needed for the claimed fragment.
