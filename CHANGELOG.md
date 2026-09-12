# Changelog

NMLT is pre-alpha and does not yet promise compatibility.

## Unreleased

- Completed R3 at local pre-alpha scope with pinned native Lake/Mathlib
  environments, ordinary editor workspaces, structured proof automation,
  explicit task revisions and durable source/project proof jobs (RFC 0036).
  Tasks/results now use v3/v5. The full reproduction gate and all 13 final
  accepted proof/fresh-recheck pairs passed. See the
  [completion audit](docs/reviews/r3-completion-audit-2026-09-11.md).
  Earlier R3 entries below describe the incremental implementation history.

- Added bounded `lean-task inspect` declaration lookup, saved context bundles,
  and native Lean diagnostics tied to captured source bytes (RFC 0035). Includes
  a checked repair workflow and a local CLI/REPL/LeanInteract comparison.
  Inspection has `assurance: none`; this increment used task/result v2/v4.

- Added R3 proof dependency graphs with separate type/value constants, recursor
  reductions, projection types, literal support and export groups (RFC 0034).
  Results now use version 4 and retain a graph plus linked readable report;
  fresh rechecks reconstruct and compare the complete graph. A real mixed
  Lean fixture also compares every node's references with Lean's collectors.

- Added bounded R3 proof exports up to 16 MiB, streamed to files with exact
  SHA-256/byte-count receipts and checked again around independent NanoDA
  validation (RFC 0033). This introduced result version 3; task format remains
  version 2. Ordinary process/R2 limits remain unchanged. Includes a real
  136,729-byte proof closure, fresh rechecking and capture rejection controls.

- Added R3 source import discovery with the pinned Lean/Lake header parser:
  source directories and explicitly selected vendored roots, dependency-ordered
  snapshots, retained import reports and fresh header/closure comparison
  (RFC 0032). This introduced task/result version 2; old records require their
  retained original executable. The explicit-module v1 project manifest remains
  supported, and broader R3 integration remains in progress.

- Started R3 with bound tasks in trusted local Lean projects: explicit source
  snapshots, a separately selected task hash, closed proof candidates, final
  proof exports checked by NanoDA, additive Lean patches, and fresh rechecking
  using retained sources and the exact CLI executable (RFC 0031). This bounded
  executable-only increment leaves broader project, editor and asynchronous
  integration in progress. See [the guide](docs/r3-lean-tasks.md).

- Completed R2 at local pre-alpha scope: projects and exact dependency locks,
  affine source job transfer, variable Init proof terms, process/resource
  containment, durable source/project resumption and finite user safety
  invariants (RFCs 0024–0030). The integrated Rust/Lean/NanoDA reproduction
  passed; final follow-up CI passed 345 Rust tests and 14 Python tests.
  See [the validation record](docs/reviews/r2-completion-2026-09-09.md).
  Earlier entries below describe the incremental implementation history.

- Started the Lean/asynchronous host increment with bounded background process
  supervision, private session handles, polling/cancellation/collection, and
  snapshot verification. The first Lean adapter checks fixed zero-addition
  proof templates against the repository pin and an exact empty-axiom policy
  (RFC 0023). Source controls were added by subsequent R2 increments.

- Connected source `job_square(Int)` effects to the durable local runtime with
  typed results, conservative effect inference, explicit attempt/time limits,
  bounded subprocess pipes, captured-journal replay without redispatch, and
  `jobs-recover` inspection (RFC 0022). The local job profile is executable-only;
  subsequent increments add Lean adapters and asynchronous source control.

### Language and semantics

- Added bounded cross-file workflow packages using canonical `import Name`
  declarations, per-file scope, imported diagnostics, and complete source
  manifests (RFC 0021). Pure formats move to version 3; version 1/2 records
  require their original executable. Finite formats remain unchanged.

- Added nominal records, structured JSON inputs, lists capped at 256 items,
  safe lookup, bounded folds, and aggregate value/work limits (RFC 0020).
  Pure workflow output moves to version 2; old records require their original
  executable. Existing finite artifact and replay formats are unchanged.

- Added executable-only pure source functions with named entries and real scalar
  inputs, local modules, acyclic calls, typed outcomes, exhaustive matching,
  immutable bindings, bounded evaluation, and exact-executable replay (RFC 0019).

- Recentered the repository on the `.nmlt` programming language with normative
  behavior semantics in Lean.
- Added typed ports, action polarity and payload binding, affine capability
  transfer, additive grades, contract facts, binary composition, explicit
  connections, observations, action hiding, and total refinement state maps to
  the source-to-core path.
- Added deterministic `behavior-core-v1` with exact source-digest identification, typed term
  ASTs, action resource profiles, compositions, and refinement witnesses.
- Added the resource-bearing Lean `Behavior`, real product-step relation,
  resource-aware weak refinement, conditional `liftParallel` theorem, primary
  fixture instantiation, and seven permanent boundary controls.
- Added a Lean artifact decoder with source-digest checking and fail-closed
  resource/contract validation.
- Added `nmlt-eval` for explicitly non-verifying artifact exploration.
- Added a dynamic affine authority-world layer with local consumption,
  synchronized transfer, ownership uniqueness, and explained world changes.
- Added conditional one-step dynamic lifting for visible local, peer,
  synchronized, and resource-compatible hidden steps.
- Attached the conditional dynamic witness to decoded artifact certificates;
  step existence and reachability remain unclaimed.

### Public project surface

- Reframed the repository around a new programming language and its
  mechanized mathematics.
- Rebuilt the active security policy, threat model, trusted-component
  inventory, roadmap, Lean guide, contribution guidance, and claim ceiling.
- Distinguished source identification from verified translation,
  product-formation policy from theorem necessity, and conditional step lifting
  from reachability.
- Focused the active documentation on the language, its mathematics, and
  reproducible implementation results.

### CLI and gates

- Added R2's bounded job lifecycle and locked journal, exact attempt/context
  bindings, revisioned control, cancellation/settlement, uncertain recovery,
  and `make r2-jobs` with a real subprocess worker. Source effects remain planned.
- Started R2 with direct finite `run` and exact-source/executable `replay`,
  shared evaluator steps, structured stop outcomes, modeled-grade accounting,
  and a retry/reuse simulation. Host adapters and the full R2 gate remain open.
- Added `elaborate --emit-core` and artifact-based `explore`.
- Removed `model-check` and `evidence` from the active CLI.
- Replaced the default gate with Rust formatting, Clippy, compilation, tests,
  fixture/artifact reproduction, Lean build, no-`sorry` policy, decoder controls,
  and theorem axiom audits.
- Removed unused Mathlib and Aeneas dependencies from the active Lean package.

### Removed from the active branch

- Contest verifier engines, temporal/OpenSystem checkers, standalone grade
  analyzer, open Rust kernel, research sketch adapter, agent evaluator, certificate
  tools, benchmark/evidence corpora, comparison harnesses, release scripts, and
  contest schemas.

The former release remains immutable at tag `build-week-judge-demo-2026`
(`0417f6e`). A reviewed follow-up resource experiment was kept in a local
quarantine snapshot and is not part of the active project.
