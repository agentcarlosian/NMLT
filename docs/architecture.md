# Architecture

## Pipeline

```text
source
  → lossless syntax tree
  → name and module resolution
  → typed behavioral core
  → verification-condition and transition-system IRs
  → backend adapters
  → checked evidence or structured witnesses
  → optional execution, tests, monitors, and trace refinement
```

No downstream stage may upgrade the assurance class produced by an upstream
stage. A backend adapter translates results; it does not reinterpret bounded
evidence as proof.

This is the intended integrated pipeline. The current research slices do not
yet form a verified end-to-end compiler: finite temporal graphs and the Phase 5
VC are constructed through explicit fixture code or a documented manual
projection, and the graded experiment uses its own `.nmltg` input.

M9 makes the source-to-typed-core prefix the active integration boundary:
complete surface projection, deterministic resolution into `nmlt-hir`,
bidirectional elaboration with an inspectable derivation, and independent
validation in `nmlt-kernel` before an engine receives `CheckedProgram`. See
[RFC 0013](../rfcs/0013-source-to-typed-core.md).

## Implemented components

- `nmlt-core`: lossless tokens/CST, recovering declaration parser, stable
  diagnostics, preservation formatter, ordered and origin-censused complete
  surface projection, explicit M9 surface-feature diagnostics, and structural
  evidence types.
- `nmlt-hir`: the canonical `nmlt-core`-to-resolver adapter, closed acyclic
  imports, typed declaration namespaces, direct-import lookup, stable semantic
  identities and locals, source-derived expression/type HIR, a complete
  `ResolutionMap`, exact-spelling candidate replay, opaque source-bound
  inputs/results, and bounded fail-closed resolution.
- `nmlt-ir`: span-free explicit typed terms, system-indexed state/temporal
  propositions, exact action update/frame partitions, canonical integer and
  core identities, and structural graph/type/resource validation. It does not
  establish correspondence to HIR and cannot construct `CheckedProgram`.
- `nmlt-elaborate`: bidirectional resolved-HIR elaboration, exact action/frame
  reconstruction, system-indexed formula formation, explicit insertions, and
  a canonical identity-bound derivation DAG with complete root/origin coverage.
  It is an untrusted producer for the kernel-accepted profile and cannot
  construct `CheckedProgram`.
- `nmlt-certificate`: neutral certificate syntax and producer-side canonical
  identity construction. Its producer utilities do not decide acceptance.
- `nmlt-kernel`: independent identity, graph, resource, rule, and aggregate
  core replay. Its private fields make successful `check` the only route to
  `CheckedProgram`.
- `nmlt-engine`: typed provider fragment and deterministic explicit-state
  exploration with structured counterexamples.
- `nmlt-temporal`: canonical finite graphs, `always`/eventuality/lasso checking,
  weak and strong action fairness, stuttering/observation utilities, finite
  forward-simulation refinement, and three-valued journal conformance. Its
  provider graph is a manually audited observation projection, not compiler
  output.
- `nmlt-verify`: finite Boolean VC IR, independent reachability and
  inductiveness evaluators, checked witnesses/certificates, SMT-LIB and Lean
  export protocols, model-test hooks, and fail-closed evidence composition.
- `nmlt-agent`: protected-artifact/edit-authority rules and a deterministic
  localized-repair protocol baseline. It is not an LLM evaluation.
- `nmlt-grades`: an independent annotated-plan experiment for product grades
  over cost, privacy, energy, and uncertainty. It is not integrated into the
  NMLT typed core; its matching standalone mathematical algebra is checked in
  Lean without a verified Rust extraction or analyzer correspondence.
- `nmlt-cli`: workspace command-line entry point.

## Active and planned boundaries

- Completed M9 elaboration (M9-005): exact resolved-HIR obligations connect to
  `nmlt-ir` nodes through a canonical bidirectional derivation artifact.
- Completed M9 `nmlt-kernel` (M9-006): independent typed-core elaboration
  checker with fail-closed diagnostics and a sealed checked-program boundary.
- Active M9 engine migration (M9-007): replace the provider parser/core route
  with exclusive consumption of `CheckedProgram`.
- Future separation of `nmlt-engine` into stable transition and model-checking
  boundaries once the first executable slice has more than one consumer.
- Future solver integration behind the existing SMT-LIB request protocol, with
  an accepted proof-certificate return format.
- Future verified elaboration into the finite graph and VC IRs.
- Future deployed-runtime adapters, journal attestation, monitor generation,
  and trace-refinement integration.
- Future `nmlt-lsp`: editor protocol implementation.

Crates are added only when a real boundary exists; empty architectural crates
are deliberately avoided.

## Trusted boundaries

The intended trusted core includes the formal kernel, evidence decoder, and the
minimal code needed to bind a checked artifact to its exact source and engine
identity. The current claim-specific boundary also includes each Rust checker,
its independent Python readback harness where used, the pinned toolchain, and
the explicitly listed platform/runtime dependencies. See the
[threat model](threat-model.md) for the inventory and attack stories.

Solvers, tactics, parsers, generators, and AI systems may sit outside a future
smaller core when their output is independently checkable. That architecture
is a target, not a claim that every current producer has already been removed
from the TCB.

Backends without independently checkable certificates require an explicit
external-trust entry in their evidence manifests.

## Intermediate representations

NMLT should not force every backend to consume the same low-level encoding.
Two initial IR families are expected:

- a transition-system IR for initialization, actions, observations, fairness,
  and traces;
- a verification-condition IR for logical obligations, assumptions, scopes,
  and proof dependencies.

The intended design derives both from the typed behavioral core and retains
stable source spans. Today, the provider engine has a contextual typed slice,
while the Phase 4 finite graph and Phase 5 finite Boolean VC evidence explicitly
record their manual/fixture construction. Source-to-IR correctness is not
claimed.

## Current implementation

The workspace keeps assurance classes separate:

| Boundary | Positive result at the implemented scope | Explicit ceiling |
|---|---|---|
| Lossless frontend | parse/format round trip or `syntax_accepted` | no semantic assurance |
| Typed provider core | provider-slice type acceptance and checked Rust/Lean correspondence vector | no full compiler-correctness theorem |
| Provider BFS | `model_checked` after frontier exhaustion within frozen bounds, or replayable `refuted` | never `proved` |
| Finite temporal graph | finite `always`/fair-lasso acceptance or refutation, simulation acceptance, or finite-prefix journal verdict | no compiler-derived general temporal language, infinite-state, or liveness-refinement proof |
| Finite VC | `model_checked` reachability and `proved` for an accepted finite-invariant certificate | proof applies to the exact VC only |
| Repair protocol | fixture-stage completion after authority checks and isolated recheck | deterministic baseline, not general agent evidence |
| Graded plan | `within_budget`, `exceeded`, or `unknown` under trusted annotations; proved laws for the standalone Lean algebra | no typed-core, Rust-correspondence, or physical/privacy soundness claim |

Persisted artifacts bind their exact subjects, configurations, implementation
source sets, executables/toolchains, controls, and residual gaps as defined by
their schemas. The standalone `evidence` command remains a structural
`unknown` scaffold; it is not silently upgraded by any semantic engine.
