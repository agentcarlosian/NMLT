# Current behavioral calculus

Status: explanatory guide to the active Lean definitions. The Lean source is
normative when this overview is incomplete.

## Behavior

The current finite behavioral object is parameterized by action, capability,
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

This dynamic layer currently defines a second `ProductState` and
`ProductStep`. It is not itself a `Behavior`, and no theorem yet
identifies it with the static product. Unifying those layers is the next
semantic milestone.

## Artifact closure

The Lean decoder accepts the finite JSON envelope, constructs states, terms,
actions, profiles, wiring, and refinement maps, then decides the premises used
to build static and dynamic conditional witnesses.

The source digest proves only that the separately supplied source bytes match
the digest written in the artifact. The checker does not re-elaborate source.
The repository gate provides a narrower reproducibility check for the committed
fixture by regenerating and byte-comparing its artifact.

The dynamic certificate maps any supplied concrete dynamic step to an abstract
match. It does not yet construct a step from the decoded initial state or prove
reachability.

## Deferred calculus

The active calculus does not contain fairness, infinite traces, liveness
transport, probabilistic or hybrid behavior, arbitrary grade algebras, general
n-ary composition, or compiler correctness.
