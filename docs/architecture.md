# Architecture

## One language, one behavioral meaning

NMLT has one active semantic path:

```text
exact .nmlt bytes
  │
  ├─ lossless CST and surface projection       nmlt-core
  ├─ resolved names and typed terms           nmlt-hir / nmlt-elaborate
  ├─ first-class ports, actions, resources    nmlt-ir
  ├─ deterministic behavior-core-v1           nmlt-compile
  │       │
  │       ├─ normative decode and semantics      Lean
  │       └─ non-verifying operational view      nmlt-eval
  │
  └─ exact SHA-256 source binding
```

The Rust artifact producer is intentionally outside the proof boundary. A
byte-for-byte snapshot and source digest make its output inspectable; they do
not prove compilation correctness. Lean decodes the artifact and owns semantic
acceptance.

## Active components

- `nmlt-core` preserves every source byte, recovers syntax, projects the
  behavioral forms, and produces stable syntax diagnostics.
- `nmlt-hir` resolves names and retains stable source identities.
- `nmlt-ir` contains the ordinary typed core plus the first-class behavioral
  objects: `CorePort`, `CoreResourceProfile`, `CoreComposition`, and
  `CoreRefinement`. Resource profiles live on actions and are never inferred by
  an evaluator backend.
- `nmlt-elaborate` and `nmlt-certificate` produce inspectable typed-elaboration
  derivations for the retained compiler-validation boundary.
- `nmlt-kernel` independently replays that typed-elaboration boundary. Despite
  its historical name, it is not NMLT's behavior prover and cannot authorize a
  theorem claim.
- `nmlt-compile` drives the exact source-to-core path and emits
  `behavior-core-v1`.
- `nmlt-eval` explores a canonical artifact for debugging and language design.
  Its results always carry an assurance level of `none`.
- `nmlt-cli` exposes `check`, `inspect`, `tokens`, `typecheck`, `elaborate`, and
  `explore`.
- `mechanization/lean/NMLT/Behavior/ResourceBehavior.lean` defines the
  normative behavior object, product transitions, resource-aware refinement,
  composition premises, and the conditional lifting theorem.
- `mechanization/lean/NMLT/Behavior/ResourceWorld.lean` is the next dynamic
  layer: a capability has one optional owner in a shared world; local steps may
  consume but not move boundary authority; synchronized product steps move
  authority between distinct owners and preserve everything unaffected. Its
  transfer theorem proves the sender cannot retain moved authority. Its
  dynamic lifting theorem weakly matches every product-step constructor through
  strengthened resource refinement: visible local, peer-local, and synchronized
  transitions remain transitions, while a hidden local step may stutter only
  when its mapped control state and complete authority world are unchanged.
- `mechanization/lean/NMLT/Artifact/BehaviorCore.lean` decodes and validates
  the finite artifact envelope into typed terms, systems, actions, profiles,
  wirings, and refinement maps.
- `mechanization/lean/NMLT/Artifact/SemanticClosure.lean` enumerates the typed
  finite states, evaluates artifact terms, constructs the normative `Behavior`
  objects and their real step relations, decides every premise of the
  refinement-lifting theorem, and returns a proof-carrying certificate. The
  executable checker also recomputes the exact source digest. It additionally
  derives the concrete product's initial authority world and exposes its
  dynamic product-step type. Accepted refinements include the reverse
  requirement implication needed to preserve world-step enabledness, and the
  dependent certificate exposes `Certificate.liftedStep` for every step of the
  decoded product. The original certificate still targets
  `ResourceBehavior.parallel`; the dynamic witness is a complete one-step
  simulation, not yet a reachability or trace theorem over authority worlds.

## Behavioral core v1

A behavior contains state, an initial predicate, a step relation, observations,
action direction and payload, hidden-action classification, owned capabilities,
and a complete resource profile for every action. A resource profile contains
required, consumed, transferred, and received authority; an additive grade;
and rely/guarantee facts.

Binary products are formed only when:

1. wiring is complete and preserved;
2. connected actions are visible, direction-compatible, and payload-compatible;
3. component capability ownership is disjoint;
4. transfer and receive sets match exactly in both directions; and
5. every synchronized reliance is guaranteed by its peer.

The product step relation has real left, right, and synchronized constructors.
Synchronized resources add grades, discharge peer reliances, combine
guarantees, and internalize the matched transfer. Hiding changes observability,
not resource meaning.

Resource-aware weak refinement carries explicit state and observation maps,
visible-step simulation, hidden-state equality, pointwise resource refinement,
and complete hidden-step compatibility with stutter. `liftParallel` proves that
this witness lifts through binary composition when both products are formed
and the whole wiring relation is preserved.

Permanent Lean controls show why the theorem cannot drop hidden-boundary
isolation, whole-wiring preservation, capability partition, transfer matching,
grade monotonicity, rely discharge, or hidden-step resource compatibility.
Source fixtures separately exercise the corresponding compiler boundaries.

## Assurance vocabulary

The active CLI deliberately does not emit `proved`, `model_checked`, evidence
manifests, or verification certificates.

- `check` and `inspect` are structural.
- `typecheck` means the Rust frontend accepted the finite surface slice.
- `elaborate` creates an auditable artifact.
- `nmlt-artifact-check` means Lean accepted the current source-bound artifact,
  constructed its finite behaviors, and instantiated the conditional
  refinement-through-composition theorem. It is not a compiler-correctness
  claim.
- `explore` is a bounded reference execution with no verification claim.
- Theorems are claims about the Lean definitions and their explicit premises.

## Deferred boundaries

The first version has no fairness fields and transports no liveness property.
Infinite traces, behavior-indexed fairness, probabilistic and hybrid behavior,
user-defined grade algebras, higher-order or partial state maps, general
composition, code generation, runtime attestation, and a verified compiler are
future work. Unsupported constructs fail with stable typed diagnostics rather
than being assigned an approximate meaning.
