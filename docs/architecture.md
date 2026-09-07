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
  ├─ deterministic behavior-core-v1/v2         nmlt-compile
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
  default `behavior-core-v1` or opt-in v2 for the finite behavioral profile.
- `nmlt-eval` shares initialization and successors between bounded reference
  exploration and direct finite execution, with Bool/Unit/enum values and
  `assurance: none`.
- `nmlt-cli` exposes `check`, `inspect`, `tokens`,
  `typecheck`, `elaborate`, `explore`, v2 path generation with `trace`, and
  experimental finite `run`/`replay`. The [local execution contract](../rfcs/0017-finite-local-run-and-replay.md)
  binds source, artifact, and executable bytes; its scheduler and record are
  executable-only and do not add a Lean or host-runtime guarantee.
- `nmlt-runtime` supplies the experimental bounded job lifecycle, typed response
  bindings, revisioned control, and locked journal with uncertain recovery.
  Its subprocess worker example uses the Rust API; the CLI now connects the same
  protocol to source jobs. Lean job integration remains outstanding.
  [RFC 0018](../rfcs/0018-bounded-local-job-lifecycle.md)
  specifies its separate executable-only boundary.
- `nmlt-workflow` consumes the same lossless CST/projection and interprets retained
  function/record slices for pure entry points, acyclic calls, scalar values,
  nominal records, bounded lists/folds, typed outcomes and matching. Its private
  typed tree and resolved record table serve execution and replay. Value validation
  bounds structured inputs and repeated aggregate production.
  Its package loader interprets canonical imports, checks per-file scopes,
  prefixes dependency declarations, and retains local spans with source indices.
  Every imported source is identified in the private program and replay manifest.
  CLI `--entry` and `typecheck --profile workflow` explicitly select this route;
  `--behavior` retains finite execution. All new constructs are executable-only
  under [RFC 0019](../rfcs/0019-pure-workflow-source.md) and
  [RFC 0020](../rfcs/0020-workflow-records-and-collections.md); no host effects or Lean
  correspondence is implied.
  [RFC 0021](../rfcs/0021-workflow-source-packages.md) specifies the local package
  reader and version 3 record boundary.
  [RFC 0022](../rfcs/0022-source-local-job-effects.md) adds a dedicated typed
  `job_square` node and transitive effect summary. An explicit host boundary
  connects the CLI's source-bound context and journal to the fixed worker;
  limits and supervision precede validated settlement/collection. The pure
  executor rejects job effects. Job replay reconstructs source and journal
  consistency without launching work; recovery classifies unfinished state.

## Active Lean components

- `NMLT.Behavior.ResourceBehavior` defines the current `Behavior`,
  resource profiles, static binary product, product-formation judgment,
  resource-aware weak refinement, and conditional `liftParallel` theorem.
- `NMLT.Behavior.ResourceWorld` defines optional nominal ownership,
  enabled local/synchronized effects, a dynamic product state/step relation,
  exact transfer properties, and conditional one-step dynamic lifting.
- `NMLT.Behavior.ResourceDynamics` combines a control presentation and deferred
  effects in one initialized behavior with one shared authority world. It
  preserves open metadata and leaf actors through binary nesting and supplies
  simulation and finite-path ownership results.
- `NMLT.Artifact.BehaviorCore` decodes and validates the finite JSON
  envelope.
- `NMLT.Artifact.SemanticClosure` enumerates finite states, interprets
  decoded terms, constructs behaviors, decides theorem premises, and returns
  static and dynamic conditional witnesses.
- `NMLT.Artifact.ExecutionClosure` and `ExecutionWitness` check decoded v2
  formation, initial states, and finite paths against the unified relation.
  `ExecutionLift` applies unified refinement to an initial synchronization.
- `NMLT.Counterexamples` contains product-formation and resource-world
  controls.

Lean checks the definitions and theorems it is given. It does not prove that
the Rust compiler produced a faithful translation, that a source model matches
an unstated human intention, or that an artifact step exists unless such a
witness is constructed.

## Unified R1 model and retained v1 artifacts

`ResourceDynamics.Behavior` reuses the control presentation and the existing
local/synchronized authority-effect rules. A completed step jointly changes
the control state and one shared world. Initialization constrains both, and
observation retains the selected control observation and full authority.

Composition retains open actions as control transitions with deferred leaf
effects; it does not prematurely execute each child in an independent world.
A completed inner synchronization can execute inside an outer product once.
A new rendezvous accepts atomic participants, not another completed sync.

For leaf pairs, `legacy_step_iff` gives exact equivalence to
`ResourceWorld.ProductStep`, while `step_projects` gives the control-product
projection. `liftParallel` states its dynamic effect and wiring premises
explicitly. Finite paths use that same completed relation and preserve vacancy;
functional worlds supply unique ownership.

The v1 artifact certificate retains its existing conditional witnesses.
The opt-in v2 route independently derives known capability types and initial
worlds, then checks supplied finite paths against `ResourceDynamics.Behavior`.
`ExecutionLift` applies the unified refinement theorem to the primary fixture's
actual initial synchronization, including initialization of its abstract image.
[RFC 0016](../rfcs/0016-decoded-finite-execution.md) states the binary scope,
explicit component/peer owner correspondence, and compatibility limits.

## Product formation and theorem premises

Static binary products are admitted only when:

1. connected actions are visible, direction-compatible, and payload-compatible;
2. declared component capability ownership is disjoint;
3. transfer and receive profiles match in both directions; and
4. every synchronized reliance is discharged by its peer guarantee.

These are current language formation rules. The existing lifting theorem uses
only a subset of the bundled formation evidence. The controls demonstrate that
each malformed product violates its named rule; they do not yet establish that
each rule is logically necessary for every possible congruence theorem.
Preservation of the complete wiring relation is a separate refinement/lifting
premise comparing the concrete and abstract products.

## Product interface limitation

The retained v1 static product is suitable for the closed two-component fixture,
but it is not a general open-system constructor. It marks product actions
internal, assigns a unit payload to the product action, and does not preserve
peer-side hidden classification for isolated right actions. General
open-interface preservation is not a claim of that artifact path. The R1
constructor preserves isolated left/right metadata and leaf effect identities.
The main nested execution has explicit formation and initialization witnesses;
the enclosed inner-synchronization example proves a standalone step.

## Dynamic authority limitation

The dynamic layer represents one owner or vacancy per capability. It proves
unique ownership, prevents isolated boundary transfer, and explains
synchronized ownership changes. Consumption intentionally vacates a
capability, so the accurate property is uniqueness/no fabrication plus
explained changes—not conservation.

The default v1 source route retains action-local receive bindings. The opt-in
v2 route knows the types of all input capability slots without granting them
ownership. A later consume or retransfer is enabled only after acquisition;
reception while already owning the capability is disabled. V2 source composition
still supports two distinct named systems; nested source instances remain later
work despite the broader constructed Lean examples.

## Assurance vocabulary

- `check` and `inspect` report structural frontend results.
- `typecheck` reports Rust frontend acceptance; Lean is not invoked.
- `elaborate` emits an inspectable artifact that still requires
  separate Lean checking.
- `nmlt-artifact-check` validates and interprets the supplied artifact,
  compares its asserted digest to supplied source bytes, and constructs
  conditional theorem witnesses or, with a v2 path, actual finite execution
  witnesses. It does not recompile source.
- `explore` is bounded reference execution with no assurance claim.
- Theorems are claims about exact Lean statements and explicit premises.

## Deferred boundaries

Fairness, divergence, infinite traces, liveness transport, arbitrary
composition, probabilistic and hybrid behavior, user-defined grade algebras,
higher-order state maps, verified compilation, code generation, runtime
attestation, and production assurance remain deferred.
