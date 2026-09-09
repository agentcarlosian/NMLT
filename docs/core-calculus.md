# Current behavioral calculus

Status: explanatory guide to the active Lean definitions. The Lean source is
normative when this overview is incomplete.

## Behavior

The retained v1 control object is parameterized by action, capability,
contract-fact, grade-atom, and observation types. It contains:

```text
State
init       : State → Prop
step       : State → Action → State → Prop
observe    : State → Observation
hidden     : Action → Prop
direction  : Action → Direction
payload    : Action → String
owns       : Capability → Prop
resources  : Action → ResourceProfile
```

A resource profile records required, consumed, transferred, and received
capabilities; a pointwise natural-number grade; and relied-upon and guaranteed
contract atoms.

## Static binary product

`ResourceBehavior.parallel` uses pairs of component control states. Its
step relation has left-local, right-local, and synchronized constructors.
Connected boundary actions synchronize; isolated actions interleave.
Synchronized resource profiles add grades and erase outward transfer/receive
fields. `Composable` separately requires those fields to match before a product
is admitted; `parallel` itself does not validate that premise.

`Composable` is a product-formation judgment. It checks complementary
directions, equal payload identities, absence of hidden connected boundaries,
disjoint declared capability ownership, exact transfer/receive agreement, and
rely/guarantee discharge.

The present `liftParallel` proof does not use every field of
`Composable`. Those fields are language/product-formation gates, not all
demonstrated necessary hypotheses of the lifting lemma. The permanent controls
show that malformed products fail their respective formation rules; they do
not yet provide countermodels to every weakened theorem statement.

## Resource-aware weak refinement

The current refinement object contains:

- an explicit state map and observation map;
- initial-state and observation preservation;
- equality of hidden-action classification;
- mapped equality for hidden control-state steps;
- visible-step simulation;
- pointwise resource-profile refinement; and
- compatibility of a concrete hidden profile with empty stutter.

The static theorem lifts that witness through binary product when wiring is
preserved and the required visibility condition is available.

No finite-path, trace, or reachability adequacy theorem has yet been proved for
`ResourceWeakRefinement`. The current result establishes that the defined
one-step relation is closed under the current binary product, not that it
already entails a separate observational trace semantics.

## Dynamic authority worlds

`ResourceWorld` adds a shared function from each nominal capability to
an optional owner. Local steps may consume authority but cannot transfer it
without synchronization. Synchronized steps may consume or transfer authority
exactly once and preserve every unaffected capability.

The dynamic one-step theorem covers:

- visible left-local transitions;
- peer-local transitions;
- synchronizations; and
- hidden left-local stuttering when the complete concrete resource profile
  refines empty and every capability owner is pointwise unchanged.

These legacy `ProductState` and `ProductStep` definitions remain unchanged for
v1 artifact regression evidence.

## Unified R1 behavior

`ResourceDynamics.Behavior` contains the control presentation, a deferred effect
per action with an agreeing resource profile, and an initial-world predicate.
Its state is the control state plus one authority world. Its initializer checks
both, its observation retains control observations and full authority, and its
completed step requires the control transition and the one shared-world effect.

Atomic effects retain their leaf actor. Binary synchronization applies the
existing `SyncStep` rule to two atomic participants. An open action survives an
inner product without attempting an unmatched transfer; an inherited completed
sync executes once. A completed sync cannot rendezvous again with a third actor.
The constructor preserves isolated actions' direction, payload and hidden
classification on both sides.

`legacy_step_iff` is an exact replacement result for leaf pairs;
`step_projects` maps a dynamic product step to its control product. The new
`liftParallel` constructs a simulation preserving initialization, observation,
resource refinement, hidden classification and weak step matching. Its wiring,
connected-visibility and synchronized-effect premises are explicit. Hidden
matching requires unchanged authority; an empty aggregate transfer summary is
insufficient because an inner synchronization may still move authority.

`Path` and `Reachable` use this same completed-step relation. The path theorem
preserves a vacant capability, and the reachable-state theorem propagates that
fact from every admitted initial state. Ownership uniqueness follows from the
functional world representation. The main nested example has explicit formation,
initialization, and finite execution witnesses; the enclosed inner-sync example
proves a standalone step. These are not decoded source paths, general trace
equivalence, fairness, or an NMLT runtime.

## Artifact closure

The Lean decoder accepts the finite JSON envelope, constructs states, terms,
actions, profiles, wiring, and refinement maps, then decides the premises used
to build static and dynamic conditional witnesses.

The source digest proves only that the separately supplied source bytes match
the digest written in the artifact. The checker does not re-elaborate source.
The repository gate provides a narrower reproducibility check for the committed
fixture by regenerating and byte-comparing its artifact.

The retained v1 dynamic certificate maps a supplied concrete step to an abstract
match. The v2 execution certificate instead carries its actual initializer and
finite `Path`, with derived reachability, vacancy preservation, unique ownership,
and an ownership-origin result: final ownership comes from initial ownership or
a transfer to that actor on the checked path. The primary v2 synchronization
also produces an initialized abstract step via the unified lifting theorem.

V2 capability and initial-world maps are recomputed from declarations and input
bindings. Supplied paths name the selected binary composition and bind the core
bytes by digest. Sentinel control/action indices and unknown authority identities
are rejected. The compiled Lean decoding/decision procedure remains trusted for
runtime acceptance; the independent exporter checks package declarations, not a
separately exported proof file for each CLI invocation.

## Deferred calculus

The active calculus does not contain fairness, infinite traces, liveness
transport, probabilistic or hybrid behavior, arbitrary grade algebras, general
n-ary composition, or compiler correctness.
