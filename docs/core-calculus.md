# Core calculus sketch

Status: research sketch, non-normative.

## Objective

The NMLT kernel should be small enough to mechanize while expressive enough to
represent typed transition systems, temporal claims, resource-sensitive
actions, composition, and refinement evidence.

## Candidate judgments

```text
Γ ⊢ e : A                         value typing
Γ ; Δ ⊢ a : Action S E ▷ g       action typing with linear context and grade
Γ ⊢ B : Behavior S O             behavior formation
Γ ⊢ φ : Property B               property formation
Γ ⊢ r : Refines B₁ B₂ via obs    refinement evidence
Γ ⊢ ev : Evidence φ scope        verification evidence
```

`Γ` contains unrestricted mathematical values. `Δ` contains affine or linear
capabilities. `g` is drawn from an explicit resource algebra and describes
quantitative effect information.

## Values

The initial value layer should contain total functions, sums, products,
records, finite sets, finite maps, relations, natural and integer arithmetic,
and propositions. Partiality must be represented explicitly rather than hidden
inside evaluation failure.

## State and actions

A state type `S` defines snapshots. An action relates a pre-state, input event,
post-state, output event, and resource grade. Nondeterminism is semantic rather
than an unspecified implementation accident.

One candidate interpretation is:

```text
Action S E O g ≅ S × E → Relation (S × O × g)
```

The exact representation remains an RFC question because it affects
composition, probability, symbolic checking, and executable elaboration.

## Behaviors and time

A behavior contains initial states and a stuttering-closed transition relation.
Infinite traces require guarded or otherwise productive coinduction. Temporal
operators are indexed by a behavior so properties cannot accidentally mix
incompatible state spaces or observations.

The v1 temporal layer should cover:

- invariant safety;
- eventuality and leads-to;
- weak and strong fairness;
- hiding and observation;
- stuttering-invariant refinement.

## Resources, effects, and authority

Grades should be parameterized by an algebra rather than hard-coded to natural
numbers. Candidate instances include call counts, latency bounds, monetary
cost, privacy exposure, retry rights, and trust provenance.

Linear capabilities represent authority that may not be duplicated. An
external effect can consume a hash-bound capability and yield either terminal
evidence or an indeterminate effect record.

## Composition

Open systems expose typed input and output ports plus assumptions and
guarantees. Composition must check port compatibility, grade composition,
authority flow, and assumption discharge.

Required metatheoretic target:

> Well-typed composition preserves well-typedness, and accepted local claims
> are preserved only when their declared compatibility conditions hold.

## Refinement

A refinement object contains:

- an observation or state mapping;
- hidden variables;
- permitted stuttering;
- environment assumptions;
- the property class preserved;
- evidence and its scope.

Refinement should be reflexive, transitive, and congruent under validated
composition. These are proof obligations, not assumed implementation facts.

## Metatheory gates

Before freezing the kernel, mechanize at least:

1. decidability of the selected static fragment;
2. preservation for executable steps;
3. progress or an explicit characterization of blocked states;
4. productivity of infinite behaviors;
5. grade soundness for the selected resource models;
6. refinement reflexivity and transitivity;
7. compositional preservation under stated conditions;
8. soundness of accepted evidence constructors.

Full cubical equality, differential dynamics, and probability are later
extensions. They must not be axiomatized into v1 merely to make examples type.
