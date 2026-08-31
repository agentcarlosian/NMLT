# Architecture

## Direction

NMLT is organized around one language pipeline and Lean-owned behavioral
semantics:

```text
exact .nmlt bytes
  │
  ├─ lossless CST and surface projection       nmlt-core
  ├─ resolved names and typed terms            nmlt-hir / nmlt-elaborate
  ├─ first-class behavioral IR                 nmlt-ir
  ├─ deterministic behavior-core-v1            nmlt-compile
  │       │
  │       ├─ finite decode and semantics         Lean
  │       └─ non-verifying operational view      nmlt-eval
  │
  └─ SHA-256 identification of exact source bytes
```

The Rust producer is outside the Lean proof boundary. A digest identifies the
source bytes supplied alongside an artifact; it does not show that Rust
translated those bytes into that artifact. The repository's primary fixture has
an additional reproducibility check: Rust regenerates it and CI requires
byte-for-byte equality.

## Active Rust components

- `nmlt-core` preserves every source byte, constructs the CST, recovers
  syntax, and projects recognized declarations with stable diagnostics.
- `nmlt-hir` resolves names and retains source-derived identities.
- `nmlt-ir` contains the ordinary typed core and first-class
  `CorePort`, `CoreResourceProfile`, `CoreComposition`, and
  `CoreRefinement` objects. Resource profiles live on actions.
- `nmlt-elaborate` and `nmlt-certificate` produce inspectable
  derivations for the retained ordinary typed-core boundary.
- `nmlt-kernel` independently replays that boundary. Its historical
  name does not make it the behavior prover.
- `nmlt-compile` drives the supported source routes and emits
  `behavior-core-v1` for the finite behavioral profile.
- `nmlt-eval` performs bounded reference exploration with
  `assurance: none`.
- `nmlt-cli` exposes `check`, `inspect`, `tokens`,
  `typecheck`, `elaborate`, and `explore`.

## Active Lean components

- `NMLT.Behavior.ResourceBehavior` defines the current `Behavior`,
  resource profiles, static binary product, product-formation judgment,
  resource-aware weak refinement, and conditional `liftParallel` theorem.
- `NMLT.Behavior.ResourceWorld` defines optional nominal ownership,
  enabled local/synchronized effects, a dynamic product state/step relation,
  exact transfer properties, and conditional one-step dynamic lifting.
- `NMLT.Artifact.BehaviorCore` decodes and validates the finite JSON
  envelope.
- `NMLT.Artifact.SemanticClosure` enumerates finite states, interprets
  decoded terms, constructs behaviors, decides theorem premises, and returns
  static and dynamic conditional witnesses.
- `NMLT.Counterexamples` contains product-formation and resource-world
  controls.

Lean checks the definitions and theorems it is given. It does not prove that
the Rust compiler produced a faithful translation, that a source model matches
an unstated human intention, or that an artifact step exists unless such a
witness is constructed.

## Two current state layers

The repository does not yet have one fully unified resource-bearing behavior:

1. `ResourceBehavior.parallel` uses a pair of component control states
   and attaches resource profiles to actions.
2. `ResourceWorld.ProductStep` adds a shared dynamic authority world.

The artifact certificate carries related refinements for both layers, but
there is no projection/correspondence theorem between their product steps.
The dynamic layer also lacks a behavior-level initializer and observation.

This is the immediate architectural gap. Until it is closed, “one semantic
path” means one promoted source/artifact route into Lean—not that the two
product-state definitions have already been proved identical.

## Product formation and theorem premises

Static binary products are admitted only when:

1. the complete wiring relation is preserved;
2. connected actions are visible, direction-compatible, and payload-compatible;
3. declared component capability ownership is disjoint;
4. transfer and receive profiles match in both directions; and
5. every synchronized reliance is discharged by its peer guarantee.

These are current language formation rules. The existing lifting theorem uses
only a subset of the bundled formation evidence. The controls demonstrate that
each malformed product violates its named rule; they do not yet establish that
each rule is logically necessary for every possible congruence theorem.

## Product interface limitation

The current static product is suitable for the closed two-component fixture,
but it is not a general open-system constructor. It marks product actions
internal, assigns a unit payload to the product action, and does not preserve
peer-side hidden classification for isolated right actions. General
open-interface preservation is therefore a future result, not a current claim.

## Dynamic authority limitation

The dynamic layer represents one owner or vacancy per capability. It proves
unique ownership, prevents isolated boundary transfer, and explains
synchronized ownership changes. Consumption intentionally vacates a
capability, so the accurate property is uniqueness/no fabrication plus
explained changes—not conservation.

The current source checker also treats received authority as an action-local
receive profile. It does not yet make that capability available to a later
receiver action. Reusable affine continuation belongs to the reachability
milestone.

## Assurance vocabulary

- `check` and `inspect` report structural frontend results.
- `typecheck` reports Rust frontend acceptance; Lean is not invoked.
- `elaborate` emits an inspectable artifact that still requires
  separate Lean checking.
- `nmlt-artifact-check` validates and interprets the supplied artifact,
  compares its asserted digest to supplied source bytes, and constructs
  conditional theorem witnesses. It does not recompile source.
- `explore` is bounded reference execution with no assurance claim.
- Theorems are claims about exact Lean statements and explicit premises.

## Deferred boundaries

Fairness, divergence, infinite traces, liveness transport, arbitrary
composition, probabilistic and hybrid behavior, user-defined grade algebras,
higher-order state maps, verified compilation, code generation, runtime
attestation, and production assurance remain deferred.
