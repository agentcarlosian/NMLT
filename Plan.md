# NMLT execution plan

- Status: active execution plan for pre-alpha language and mathematics research
- Current architecture: Rust frontend and reference evaluator; Lean behavioral semantics
- Current result: unified resource-aware semantics, decoded finite execution,
  and received-capability continuation; v1 retains its conditional witnesses
- Next milestone: R3 — existing Lean projects (native workflows implemented; final reproduction and exit audit in progress)
- Target: one executable language serving AI/Lean developers, mathematicians,
  and software engineers
- Updated: 2026-09-11

This file governs execution priorities and milestone completion. The
[practical language design](docs/practical-language-plan.md) supplies detailed
requirements and evaluation criteria; the
[research review](docs/research-notes/practical-language-strategy-2026-09-06.md)
records evidence and alternatives. Planned features are not current capabilities.
Language and trust-boundary changes still need explicit RFCs and implementation
evidence.

## Program direction

Build NMLT into an executable language for computations with explicit behavior,
resources, and checkable results. Keep mathematical definitions and proofs in
native Lean. Programs should accept inputs, compose tool calls, inspect typed
outcomes, branch or retry within limits, reuse outputs, and reproduce what was
checked.

The first alpha targets three complete workflows on one interpreter, project
format, adapter protocol, and result model:

| Audience | Runnable workflow | Useful result |
|---|---|---|
| AI/Lean developers | Compare bounded proof strategies and recover interrupted attempts | Checked Lean patches, diagnostics, and resource measurements |
| Mathematicians | Explore lemma families, conjectures, counterexamples, and proofs | Reusable lemmas, checked refutations or constructions, and explanations |
| Software engineers | Execute a permit-controlled worker with receive-then-use authority, cancellation, and failure | Working software, model obligations, and scoped trace comparisons |

New mathematics is an explicit research goal: investigate stronger composition
results, weaker sufficient assumptions, useful counterexamples, resource bounds,
and mathematical constructions. Proof validity, faithfulness to an intended
claim, novelty, and usefulness remain distinct judgments.

## Completed foundation

The current branch establishes:

- lossless syntax, name resolution, typed IR, elaboration, and an independent
  validator for the retained ordinary typed-core boundary;
- typed ports, action polarity, affine capability transfer, additive named
  grades, rely/guarantee atoms, observation, binary connections, hiding, and
  explicit finite refinement maps in the behavioral source slice;
- deterministic `behavior-core-v1` artifacts and exact canonical snapshots;
- a Lean decoder that constructs finite behaviors and decides the premises of
  the static conditional composition/refinement theorem;
- an auxiliary dynamic authority-world semantics with unique ownership,
  consumption, exact transfer, and conditional one-step lifting for local,
  peer, synchronized, and resource-compatible hidden steps; and
- a bounded Rust explorer that always reports `assurance: none`.

The P1, P2, and P3 audit fixes are committed. P3's shared-term graph depth fix
is preserved separately in `33e7240`.

## Current honesty boundary

The v1 artifact path still constructs static and dynamic conditional witnesses.
R1's `ResourceDynamics.Behavior` combines control and authority in one state,
initializer, observation, and completed-step relation. Its leaf-pair step is
equivalent to the old dynamic relation and projects to the control product.
It has finite-path ownership results and constructed nested examples; these
are supplemented by v2 decoded initial paths and source-level receive then
consume/retransfer. The primary initial synchronization also lifts to an
initialized abstract step. Product formation remains distinct from the minimal
premises of lifting. The source digest identifies bytes without verifying
elaboration, and compiled Lean decoding/decision remains a runtime trust boundary.

Public documentation must preserve those distinctions.

The behavioral and retained ordinary compilation routes differ; the reference
explorer supports finite Bool/Unit/enum state. A separate executable-only profile
now runs functions with scalars, outcomes, records, bounded lists/folds, and
opt-in local jobs. Affine source controls expose workers and closed Init proof
terms, with exact dependency identities and durable source/project resumption.
User safety predicates have a separately checked finite-model route. The first
R3 profile now binds native Lean/Lake projects and fixed targets, with editor,
automation and asynchronous proof workflows under final verification.
Historical independent-checker results do not remove the need for current
toolchain maintenance.

## Execution milestones

| Milestone | Status | Dependency | Responsible role |
|---|---|---|---|
| R0 — Checker and workflow baselines | Complete | Audited baseline | Integration and Lean maintainers |
| R1 — Unified semantics and finite execution | Complete at finite scope | R0 checker baseline | Lean/semantics and Rust maintainers |
| R2 — Useful executable language | Completed at local pre-alpha scope on 2026-09-09; see completion audit | R1 for formal execution claims | Compiler/runtime and integration maintainers |
| R3 — Supported Lean workflows | Native project, editor, automation, revision and asynchronous workflows implemented; final exit checks running | R2; adapter prototype can begin in R0 | Lean integration maintainer |
| R4 — Discovery workflows | Planned | R3 for integration; domain preparation can start earlier | Mathematical reviewer and integration maintainer |
| R5 — Three-audience alpha validation | Planned | R2–R4 | Maintainers and independent pilot users |

Mark a milestone complete only when its exit gate has recorded evidence.

### R0 — Establish the checker and workflow baselines

Completed on 2026-09-06. The [R0 evidence record](docs/reviews/r0-checker-and-baselines-2026-09-06.md)
contains immutable checker pins, exported-input hashes, executed case results,
validation commands, and the specific friction for the next stages to address.
Committed as `f966758` before beginning R1.

- Completed: preserve the P3 audit fix as its own reviewable commit (`33e7240`),
  separate from toolchain and feature changes; all 18 `nmlt-ir` tests passed.
- Completed: upgrade Lean from 4.30.0 to 4.33.1 with compatible pinned exporter
  and NanoDA revisions. Clean metatheory and the unchanged axiom policy pass;
  a fresh export independently checks 7,224 declarations without errors.
- Completed: [three baseline programs](examples/baselines/README.md) and their
  [input/output contracts](docs/r0-baseline-contracts.md), using batch Lean and
  Python. All cases and real proof interruption/resume pass on the pinned checker.
- Completed: freeze the public calibration cases, deterministic strategies,
  budgets, success meanings, and stop criteria. Fourteen harness tests pass;
  no baseline model access or incremental adapter is required.
- Completed: record [Draft RFC 0014](rfcs/0014-executable-workflow-profile.md)
  covering source-route consolidation, runtime/effect boundaries, artifact
  evolution, and the proposed extension to ADR 0004. Draft status is not acceptance.
- Completed: [role-based pilot cards and forms](docs/r0-pilot-protocol.md).
  Identifying or recruiting specific participants is not required for R0;
  independent user evaluation remains later work.

Exit gate: a fresh checker compatibility record, three runnable baseline
examples, and specific user friction that NMLT will attempt to reduce.

### R1 — Complete the semantic basis for execution

Retain M1–M3 as explicit mathematical milestones within R1.

The first increment established the unified Lean model and finite values;
its [evidence record](docs/reviews/r1-unified-behavior-increment-2026-09-06.md)
remains historical. The next increment implements v2 capability/initial-world
maps, decoded paths, receive then consume/retransfer, ownership-origin results,
and initial synchronized refinement through the unified theorem. V1 remains
the default and keeps its contract and canonical fixtures.
[RFC 0015](rfcs/0015-unified-resource-bearing-behavior.md) and
[RFC 0016](rfcs/0016-decoded-finite-execution.md) remain Under review.
Completed on 2026-09-06 at finite binary-source scope. The
[R1 completion evidence](docs/reviews/r1-finite-execution-2026-09-06.md) records
202 passing Rust tests, 30 execution rejection controls, both complete finite
graph comparisons, and a fresh NanoDA check of 8,655 declarations.
RFC acceptance and the publication review gate are separate.

#### M1 — Unified resource-bearing behavior

Completed: unified model, legacy equivalence, control projection, separate
formation, and constructed nested binary executions.

- Make dynamic authority part of the normative behavior state rather than a
  parallel semantic layer, including initialization and observation.
- Preserve observations and the remaining open interface through product
  construction, including peer-side hiding, direction, and payload.
- Prove a projection or replacement theorem relating the present static and
  dynamic step relations.
- Re-state product formation separately from the minimal logical premises of
  step lifting.
- Include nested binary composition examples before claiming general n-ary
  composition.

Exit gate: one behavior object and one product-step relation support the
resource theorem without duplicated semantic authority.

#### M2 — `behavior-core-v2` and artifact-derived execution witness

Completed: reproduced v2 artifacts, decoded initial synchronization, exact
authority movement, and an initialized abstract step through unified lifting.

- Encode initial authority, dynamic state, and the data needed to form enabled
  local and synchronized steps.
- Reproduce the positive artifact byte-for-byte.
- Construct in Lean an initial decoded state and an actual synchronized step
  for the primary source fixture.
- Prove the decoded step moves authority exactly once and is accepted by the
  unified lifting theorem.
- Add tampering controls for initial worlds, ownership, and claimed steps.
- Preserve v1 regression fixtures and specify v2 migration and
  unsupported-version behavior.

Exit gate: the artifact checker proves conditional lifting and exhibits the
fixture's initial dynamic step. This remains source identification until a
separate translation-validation or compiler-correctness result exists.

#### M3 — Finite reachability and affine continuation

Completed: decoded three-step consume and retransfer paths, ownership-origin
results, and complete Rust/Lean comparisons for the frozen value/resource corpus.

- Define reachable dynamic states from decoded initial states.
- Permit a received affine capability to enter the receiver's post-step
  authority context and be consumed or retransferred exactly once.
- Prove ownership uniqueness and explained authority changes over finite paths.
- Bring Rust exploration into Bool/Unit/enum parity and cross-check states,
  enabled steps, and bounded paths against Lean on a frozen corpus.

Exit gate: decoded receive-then-use and receive-then-transfer examples have
checked finite paths, with meaningful local, synchronized, and hidden cases
and independent negative controls for duplication and fabrication. Conditional
lifting alone cannot complete R1. Formation rejection alone does not establish
the necessity of a theorem premise.

### R2 — Deliver the smallest useful executable language

Completed on 2026-09-09 at local, finite, pre-alpha scope: typed projects,
dependency locks, finite user safety invariants, affine handle transfer,
contained local processes, variable Init proof terms and durable source/project
resumption. The [completion audit](docs/r2-completion-tracker.md) and
[validation record](docs/reviews/r2-completion-2026-09-09.md) cover the full
requirements. The increment descriptions below are implementation history.
RFC acceptance and publication review remain separate.

Started on 2026-09-06 from R1 commit `c1404f1`. The first increment adds
[direct finite execution and replay](docs/r2-local-execution.md), a shared
initializer/successor operation for execution and exploration, structured stop
outcomes, exact source/artifact/executable identities, and a finite retry/reuse
example. [RFC 0017](rfcs/0017-finite-local-run-and-replay.md) is Under review.
The [increment evidence](docs/reviews/r2-local-execution-increment-2026-09-06.md)
records 215 passing Rust tests, unchanged frozen Rust/Lean graphs, and the
separately checked five-step retry witness.
This is an executable-only scheduling/record layer over existing v2 operations,
not the general workflow runtime or completion of R2.

The second increment adds the executable-only `nmlt-runtime` job lifecycle,
typed attempt bindings, revisioned control, reservation/settlement rules, and a
locked journal with explicit uncertain restart recovery. A real local square
worker demonstrates failure, bounded fallback, and persistent result reuse.
[RFC 0018](rfcs/0018-bounded-local-job-lifecycle.md) is Under review. This remains
a Rust API and adapter prototype; `.nmlt` host-effect integration is outstanding.
The [second-increment evidence](docs/reviews/r2-job-lifecycle-increment-2026-09-06.md)
records cross-platform tests, two process-death boundaries, and fresh unchanged
Lean/NanoDA checks.
The first two increments were committed as `80e2a0c` before further development.

The third increment adds [pure source workflows](docs/r2-pure-workflows.md):
named entry points with scalar inputs, local modules, reusable acyclic functions,
typed `Outcome<T>` values, exhaustive matching, immutable bindings, and bounded
evaluation/replay. It reuses the lossless source projection and retains source
spans through its separate typed lowering. [RFC 0019](rfcs/0019-pure-workflow-source.md)
is Under review. The entire new profile is executable-only; its functions do
not yet execute the job runtime or inherit finite behavioral proofs.
The [third-increment evidence](docs/reviews/r2-pure-workflow-increment-2026-09-06.md)
records 247 Windows / 248 Linux tests, source and replay rejection controls,
and the complete fresh Rust/Lean/NanoDA reproduction gate.

The fourth increment extends that profile with nominal records, structured JSON
inputs, homogeneous lists capped at 256 items, safe lookup, and bounded folds.
Aggregate value and cumulative value-work limits bound nested data and reuse.
[RFC 0020](rfcs/0020-workflow-records-and-collections.md) is Under review; all
new constructs remain executable-only. The profile and pure record formats move
explicitly to version 2. The [batch example](examples/pivot/batch_summary.nmlt)
handles three input records and returns accepted 2, rejected 1, total 41.
The [fourth-increment evidence](docs/reviews/r2-collections-increment-2026-09-06.md)
records 257 Windows / 258 Linux tests and a complete fresh
Rust/Lean/NanoDA/baseline gate.

The fifth increment adds [cross-file packages](docs/r2-source-packages.md) using
the canonical `import Name` syntax, sibling module files, explicit per-file
scope, and global declaration/type checks. File-aware locations and a complete
source manifest bind imported code to version 3 pure replay. The
[four-file example](examples/pivot/package_batch/main.nmlt) passes records and
outcomes between reusable libraries. [RFC 0021](rfcs/0021-workflow-source-packages.md)
is Under review and the new facilities remain executable-only.
The [fifth-increment evidence](docs/reviews/r2-package-increment-2026-09-07.md)
records 269 Windows / 271 Linux tests and the complete fresh
Rust/Lean/NanoDA/baseline gate.

The sixth increment connects [source jobs](docs/r2-source-jobs.md) to the durable
runtime. `job_square(Int)` yields `Outcome<Int>` after validated settlement and
collection. Conservative transitive effect inference protects the pure route.
Explicit job/time limits, bounded subprocess pipes, source-bound context and
journal capture, replay without launching work, and `jobs-recover` inspection
complete this first native adapter. Host failures stop with uncertain charged
work; source domain failures can select a bounded fallback and reuse its value.
[RFC 0022](rfcs/0022-source-local-job-effects.md) is Under review. This remains
executable-only and does not complete the asynchronous job or Lean adapter scope.
The [sixth-increment evidence](docs/reviews/r2-source-job-increment-2026-09-07.md)
records 287 Windows / 289 Linux Rust tests, the source fallback demonstration,
and a complete fresh Rust/Lean/NanoDA/parity/baseline gate.

The source workflow, collection, package, and synchronous job increments were
committed as `0f4a9c8` before starting the seventh increment.

The seventh increment starts [Lean adapters and asynchronous host control](docs/r2-lean-async.md).
An initial Rust session API starts, polls, cancels, and collects worker or fixed
Lean-template jobs through the durable journal. Opaque session-specific handles,
independent child deadlines, uncertain cleanup states, and captured-observation
verification are implemented under [RFC 0023](rfcs/0023-asynchronous-host-and-lean-adapter.md).
The actual pinned Lean process rejects wrong/admitted proofs and checks the
existing-lemma and induction templates. These host APIs do not yet expose
asynchronous `.nmlt` handles or general Lean source input.
The [seventh-increment evidence](docs/reviews/r2-lean-async-increment-2026-09-07.md)
records the current tests and real subprocess validation.

The eighth increment exposes [scoped asynchronous source controls](docs/r2-source-async.md)
under [RFC 0024](rfcs/0024-scoped-source-job-controls.md). Let-bound `Job<Int>`
and `Job<Text>` handles start worker and fixed-template Lean jobs. Poll/cancel
borrow authority; collection consumes it. Branches agree on consumed handles,
folds preserve outer authority, and iteration-local jobs must be collected.
The separate async source record validates source operations against journal
boundaries and captured observations before replay without redispatch.
The [eighth-increment evidence](docs/reviews/r2-source-async-increment-2026-09-07.md)
records the complete native Windows Rust/Lean/NanoDA gate and remaining R2 gaps.

The next increment adds the [local project loop](docs/r2-projects.md) under
[RFC 0025](rfcs/0025-local-projects-and-dependency-locks.md): manifest inputs,
dependency/tool locks, initialization, tests, formatting, structured diagnostics,
and replay from retained source snapshots and executables. The
[completion tracker](docs/r2-completion-tracker.md) audits the complete milestone;
project tooling alone does not complete R2.

The [finite safety route](docs/r2-safety-invariants.md) under
[RFC 0026](rfcs/0026-finite-source-safety-invariants.md) now preserves user
predicates and constructs Lean-checked initialization/preservation evidence or
an initialized counterexample. Its reached-set certificate is checked against
complete semantic state/action enumerations; Rust exploration completeness is
not assumed. Later completed increments add affine job transfer (RFC 0027),
contained process lifecycle (RFC 0028), pinned Init proof terms (RFC 0029),
and durable source/project resumption with explicit uncertainty (RFC 0030).
The complete reproduction and final follow-up CI are recorded in the audit.

The full R2 requirements and exit gate audited above are:

- Share parsing, names, types, source spans, and lowering facilities where
  appropriate; document the source route for each supported construct.
- Implement a local interpreter and reuse its common step operations for
  exploration and execution.
- Add entry points, modules, reusable components, records, tagged outcomes,
  pattern matching, total functions, and bounded collections/iteration as the
  three workflows require them.
- Provide typed local effects for starting, observing, cancelling, and
  collecting Lean jobs and one simple software worker.
- Specify bounded job slots and attempt generations, ownership, settlement,
  stale-result rejection, and recovery. Test late completion after cancellation,
  slot reuse, and restart with uncertain in-flight work.
- Separate reusable proofs/artifacts from affine job/write permissions, and
  modelled resource grades from observed expenditure and enforced reservations.
- Add one complete user-defined safety-invariant feature: preserve the predicate
  in the artifact, generate initialization/preservation obligations, and return
  checked evidence or an explicitly scoped counterexample.
- Supply project/toolchain locks, source-located diagnostics, structured results,
  a formatter, and the planned `init`, `run`, `test`, and `replay` development loop.

Every new construct needs specified lowering and Lean interpretation/validation,
or an executable-only designation. Applying finite model properties to concrete
execution requires a stated abstraction/simulation obligation or a disclosed
trusted assumption. Typed adapters do not prove all host behavior; local
ownership does not imply exactly-once external execution.

Exit gate: a user modifies real input, runs a complete program, handles failure,
reuses a successful output, and replays the result without manual JSON editing.
Document the semantic scope of every exercised construct.

### R3 — Support existing Lean projects

The first increment implements [bound local Lean tasks](docs/r3-lean-tasks.md)
under [RFC 0031](rfcs/0031-bound-lean-project-tasks.md): exact declaration types,
source/module and tool identities, a separate closed proof-term candidate,
Lean target checks, transitive axiom policy, independent NanoDA artifacts,
actual dependencies, readable additive patches and fresh source-based rechecking.
The profile is bounded to trusted local sources and bounded exported closures.
The second increment adds automatic source import discovery with Lean's native
header parser under [RFC 0032](rfcs/0032-lean-source-import-discovery.md). It
supports explicitly selected source directories and vendored roots, records
the import closure, and reparses saved headers before proof reconstruction.
The [second-increment validation record](docs/reviews/r3-import-discovery-2026-09-11.md)
records the complete Windows and Linux gates and the discovery controls.
The third increment adds byte-exact file exports up to 16 MiB under
[RFC 0033](rfcs/0033-bounded-lean-proof-exports.md), with captured byte counts,
digests and independent rechecking. Its
[local validation record](docs/reviews/r3-bounded-exports-2026-09-11.md) records
the larger proof fixture and failure controls.
The fourth increment adds actual declaration dependency graphs under
[RFC 0034](rfcs/0034-lean-proof-dependency-graphs.md), with explicit constants,
projection names, reductions, literal support and export groups kept distinct.
The [local validation record](docs/reviews/r3-proof-dependencies-2026-09-11.md)
records complete graph reconstruction and reference comparison with Lean.
The fifth increment adds bound declaration lookup and native Lean compilation
diagnostics under [RFC 0035](rfcs/0035-bound-lean-inspection-and-diagnostics.md).
The [local validation record](docs/reviews/r3-lean-inspection-2026-09-11.md) covers
inspection, located candidate repair and an initial CLI/REPL/LeanInteract
comparison. Retrieval context cannot become proof acceptance.
The final integration implements native pinned Lake package reconstruction,
ordinary editor workspaces and actual LSP interaction, structured native proof
automation, explicit revision lineage and asynchronous source/project proof
jobs under [RFC 0036](rfcs/0036-native-lean-project-workflows.md). Focused checks
include a pinned Mathlib environment and fresh independent checking of every
accepted native-development and source-job proof. The
[completion audit](docs/reviews/r3-completion-audit-2026-09-11.md) tracks the
remaining full reproduction and final frozen-set checks. The complete exit gate
below remains authoritative.

- Bind tasks to exact targets, assumptions, definitions, and pinned dependency
  environments; check formal target identity in Lean.
- Separate candidate statements, proof patches, and checked results. Explicit
  statement revisions invalidate dependent acceptance records.
- Reuse compatible Lean diagnostics, retrieval, REPL/LSP interfaces, and proof
  automation; add advanced subgoal search only when needed.
- Reconstruct ordinary Lean files, check them in a clean pinned environment,
  enforce the transitive axiom policy, compare targets, and independently
  recheck final proof artifacts.
- Separate draft proof plans from actual formal dependencies. Scheduler
  bookkeeping cannot turn unresolved lemmas into a completed root theorem.
- Export readable patches, dependencies, and explanations attached to exact
  declarations, retaining familiar Lean editor workflows.

Exit gate: every reported success independently rechecks without the generating
model session. Worker success or an empty tactic-goal list is insufficient.
Replay checks recorded artifacts; it need not regenerate model responses.

### R4 — Make discovery a first-class workflow

- Distinguish conjectures, bounded computational evidence, checked
  counterexamples, Lean theorems, and novelty/usefulness review.
- Calibrate by reproducing a known result before claiming discovery ability.
- Investigate the sufficient premises of an NMLT composition law against
  deterministic analysis and ordinary Lean automation. Necessity countermodels
  must satisfy retained assumptions and falsify the fixed conclusion; require
  admitted initial states and enabled behavior to check non-vacuity.
- Search for worker-protocol improvements against frozen inputs, environment
  traces, and required outcomes. Check every case within declared bounds before
  scoring; reject added deadlocks or cheap failures replacing required success.
- Run an outward-facing finite-construction experiment with an exact predicate
  and mathematical reviewer. Hold out sizes and distinguish individual objects
  from proved parameterized families.
- Record prior-art searches and human assessment separately from proof
  acceptance. Failed search remains unknown; new formalization is not
  automatically new mathematics.

Exit gate: reproduce a known result and obtain at least one useful checked
generalization, counterexample, or construction that a reviewer can explain
and reuse. No deadline guarantees a new theorem. Bounded completion checks
do not establish general liveness.

### R5 — Demonstrate usefulness for all three audiences

- R0 freezes the small public calibration corpus. Before comparative testing,
  finalize and freeze a separate evaluation corpus: proposed scope is 30 Lean
  tasks, 12 finite behavior scenarios plus invalid controls, and three
  calibrated discovery exercises. These remain targets, not existing benchmarks.
- Separate tuning from evaluation projects; report finite bounds, rejected
  attempts, unknown outcomes, and contamination limitations.
- Require every reported proof/trace success to recheck and every specified
  invalid control to be rejected, with actual denominators reported.
- Compare with a library-only baseline using identical adapters, orchestration,
  prompts, retrieval, caching, models, and checking policy. Separate language
  gains from improvements to the proving system.
- Use recorded-response replay and paired repeated live runs; measure setup,
  hands-on time, resource use, diagnosis, proof maintenance, and reuse.
- Have at least one independent user per audience install, modify, run, and
  explain its workflow; treat this as qualitative pilot evidence.
- Pre-register a useful improvement target. The detailed design suggests 20%
  lower median hands-on time without worse acceptance quality, with sample
  size and uncertainty explicitly reported.

Exit gate: all three workflows are independently usable, their results reproduce
within the declared scope, and comparison evidence supports the value of NMLT.
If the language adds ceremony without benefit, retain useful libraries,
simplify the surface, and repeat the focused comparison.

## Sequencing and parallel work

The integration chain is R0 → R1 → R2 → R3 → R4 → R5. The R0 checker gate
precedes new theorem claims. Baseline/adapter prototypes, source/UX design,
and discovery-domain preparation can proceed alongside semantic unification;
they cannot inherit guarantees from unfinished semantics.

Start with the separate P3 commit and checker compatibility change, then the
executable-profile RFC and three baseline contracts. Develop M1 alongside the
batch Lean adapter prototype. Exhibit M2/M3 executions before promoting runtime
resource guarantees. Update estimates from completed gates rather than promise
a calendar date for unresolved mathematics.

## Later mathematical milestones

### M4 — Behavior-indexed fairness

Only after M1–M3 are stable and a concrete workflow needs it:

- reintroduce the quarantined hidden-divergence question;
- define fairness over the unified behavior rather than over labels alone;
- distinguish finite stuttering from infinite hidden divergence; and
- state the first liveness transport theorem with explicit fairness premises.

No current result transports liveness.

## Deferred research

Richer refinement maps, infinite state, probabilistic and hybrid behavior,
general n-ary composition, user-defined grade algebras, and verified translation
remain later research.

Defer own foundation-model training, a replacement for Lean/mathlib, another
proof kernel, optimizing native code generation, a public package registry,
distributed deployment, runtime attestation, and separate bespoke UIs until
the pilots justify them. A general production assurance system is not an alpha
deliverable.

## Publication gate

Before any public PR:

- reconstruct a reviewable stack from `origin/main`, preserving the
  current branch as a safety reference;
- run Rust and Lean gates at every PR tip and from a clean worktree;
- reproduce and byte-compare the canonical artifact;
- reject tracked generated outputs and broken local links;
- verify every path in the trusted-component inventory;
- resolve or explicitly waive independent critical-review findings; and
- complete one authenticated cross-family review.

## Historical boundary

The contest-oriented release remains available from
`build-week-judge-demo-2026` at `0417f6e`. Historical plans and
experiments are research records, not current architecture.
