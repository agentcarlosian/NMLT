# NMLT execution plan

- Status: active execution plan for pre-alpha language and mathematics research
- Current architecture: Rust frontend and reference evaluator; Lean behavioral semantics
- Current result: finite resource-aware composition plus auxiliary dynamic
  authority-world one-step lifting
- Immediate milestone: R1 — unified semantics and finite execution
- Target: one executable language serving AI/Lean developers, mathematicians,
  and software engineers
- Updated: 2026-09-06

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

The static `Behavior.parallel` semantics and the dynamic
`ResourceWorld.ProductStep` semantics are currently separate layers. The
dynamic layer is not yet a `Behavior`, has no artifact-derived initial-step
existence theorem, and has no reachability or trace theorem. Product formation
checks several language invariants that are not all logical dependencies of the
current lifting proof. The source digest identifies supplied source bytes but
does not verify Rust elaboration.

Public documentation must preserve those distinctions.

The behavioral and retained ordinary compilation routes differ; the reference
explorer currently supports Boolean state. General program execution, host
effects, and user-defined behavioral property checking remain planned work.
Historical independent-checker results do not remove the need for current
toolchain maintenance.

## Execution milestones

| Milestone | Status | Dependency | Responsible role |
|---|---|---|---|
| R0 — Checker and workflow baselines | Complete | Audited baseline | Integration and Lean maintainers |
| R1 — Unified semantics and finite execution | Planned | R0 checker baseline | Lean/semantics and Rust maintainers |
| R2 — Useful executable language | Planned | R1 for formal execution claims | Compiler/runtime and integration maintainers |
| R3 — Supported Lean workflows | Planned | R2; adapter prototype can begin in R0 | Lean integration maintainer |
| R4 — Discovery workflows | Planned | R3 for integration; domain preparation can start earlier | Mathematical reviewer and integration maintainer |
| R5 — Three-audience alpha validation | Planned | R2–R4 | Maintainers and independent pilot users |

Mark a milestone complete only when its exit gate has recorded evidence.

### R0 — Establish the checker and workflow baselines

Completed on 2026-09-06. The [R0 evidence record](docs/reviews/r0-checker-and-baselines-2026-09-06.md)
contains immutable checker pins, exported-input hashes, executed case results,
validation commands, and the specific friction for the next stages to address.

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

#### M1 — Unified resource-bearing behavior

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
