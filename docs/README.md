# NMLT documentation

NMLT is a programming language and mechanized-mathematics research project.
These documents distinguish the active architecture from historical verifier
experiments.

## Start here

1. [Manifesto](manifesto.md) — why new languages and mathematics belong
   together
2. [Project roadmap](roadmap.md) — current result, next dependencies, and
   deferred work
3. [Language sketch](language-sketch.md) — implemented finite surface and
   future direction
4. [Core calculus](core-calculus.md) — the current Lean semantic objects and
   their limitations
5. [Architecture](architecture.md) — Rust/Lean boundary and artifact path
6. [Design principles](design-principles.md) — project constraints

## Semantics and trust

- [Getting started](getting-started.md)
- [Threat model](threat-model.md)
- [Artifact identity and translation boundary](artifact-identity.md)
- [Lean axiom policy](../mechanization/lean/AXIOMS.md)
- [Lean package guide](../mechanization/lean/README.md)

The current normative executable artifacts are:

- [`behavior-core-v1` schema](../schemas/behavior-core-v1.schema.json);
- [primary NMLT source](../examples/pivot/visible_resource_sync.nmlt);
- [canonical primary artifact](../examples/pivot/visible_resource_sync.behavior-core-v1.json);
- [Lean behavior semantics](../mechanization/lean/NMLT/Behavior/ResourceBehavior.lean);
- [dynamic authority-world layer](../mechanization/lean/NMLT/Behavior/ResourceWorld.lean); and
- [artifact semantic closure](../mechanization/lean/NMLT/Artifact/SemanticClosure.lean).

R1 additionally introduces a
[unified dynamic behavior model](../mechanization/lean/NMLT/Behavior/ResourceDynamics.lean)
and [nested execution examples](../mechanization/lean/NMLT/Examples/NestedResourceDynamics.lean).
The v1 contract is retained. The opt-in
[v2 schema](../schemas/behavior-core-v2.schema.json),
[execution witness schema](../schemas/behavior-execution-v1.schema.json), and
[path checker](../mechanization/lean/NMLT/Artifact/ExecutionWitness.lean) use the
unified model. See the [v2 execution guide](getting-started.md#finite-v2-execution).

## Language and implementation

- [R2 local execution](r2-local-execution.md) — finite source-driven run/replay,
  structured outcomes, and the remaining workflow-runtime boundary
- [R2 job runtime](../crates/nmlt-runtime/README.md) — bounded job control,
  journal recovery, and a real local subprocess adapter prototype
- [R2 pure workflows](r2-pure-workflows.md) — real inputs, named functions,
  records, bounded lists/folds, typed outcomes, matching, execution, and replay
- [R2 source packages](r2-source-packages.md) — reusable libraries, file scope,
  source manifests, imported diagnostics, and version 3 replay
- [R2 source jobs](r2-source-jobs.md) — typed subprocess outcomes, bounded fallback,
  durable context, replay without dispatch, and recovery inspection
- [Source corpus](source-corpus.md) — historical frontend corpus and provenance
- [Untyped projection](untyped-core-desugaring.md)
- [Typed executable core](typed-executable-core.md) — retained ordinary
  typed-core history and boundary
- [Semantics correspondence](semantics-correspondence.md) — retained M9
  correspondence scope
- [RFC registry](../rfcs/README.md)
- [Architecture decisions](decisions/README.md)

## Research method and history

- [Research method](research-method.md)
- [Practical language plan](practical-language-plan.md) — proposed shared
  language and workflows for AI/Lean development, mathematics, and software
- [Practical language research](research-notes/practical-language-strategy-2026-09-06.md)
  — evidence, alternatives, and limitations behind the proposal
- [R0 baseline contracts](r0-baseline-contracts.md) — frozen tasks, budgets, and
  evidence meanings for the three reference workflows
- [R0 pilot protocol](r0-pilot-protocol.md) — reusable role-based tasks and
  observation forms for later evaluation
- [R0 completion evidence](reviews/r0-checker-and-baselines-2026-09-06.md)
  — patched checker compatibility, executed workflows, and remaining friction
- [R1 first-increment evidence](reviews/r1-unified-behavior-increment-2026-09-06.md)
  — unified model, mixed finite values, validation results, and remaining gates
- [R1 completion evidence](reviews/r1-finite-execution-2026-09-06.md)
  — decoded finite paths, affine continuation, initialized refinement, and fresh checks
- [R2 first-increment evidence](reviews/r2-local-execution-increment-2026-09-06.md)
  — finite run/replay, shared steps, mutation controls, and remaining runtime work
- [R2 second-increment evidence](reviews/r2-job-lifecycle-increment-2026-09-06.md)
  — bounded job control, recovery, subprocess execution, and cross-platform checks
- [R2 third-increment evidence](reviews/r2-pure-workflow-increment-2026-09-06.md)
  — pure source functions, typed outcomes, input-sensitive execution, and replay
- [R2 fourth-increment evidence](reviews/r2-collections-increment-2026-09-06.md)
  — records, structured inputs, bounded lists/folds, value limits, and version 2 replay
- [R2 fifth-increment evidence](reviews/r2-package-increment-2026-09-07.md)
  — reusable source packages, imported locations, dependency manifests, and version 3 replay
- [R2 sixth-increment evidence](reviews/r2-source-job-increment-2026-09-07.md)
  — bounded source effects, subprocess supervision, durable context, replay, and recovery
- [Executable workflow profile](../rfcs/0014-executable-workflow-profile.md)
  — Draft design for the proposed shared language and runtime
- [Unified resource-bearing behavior](../rfcs/0015-unified-resource-bearing-behavior.md)
  — Under review design for R1's first implementation increment
- [Decoded finite execution](../rfcs/0016-decoded-finite-execution.md)
  — Under review contract for v2 artifacts, capability continuation, and paths
- [Historical records](history.md)
- [Public pivot pre-PR review](reviews/public-pivot-pre-pr-2026-08-31.md)

Any dated handoff, reproduction report, completion audit, test report, or
research note is a historical record. It can explain how the project arrived
here, but it cannot override the current architecture, roadmap, security
inventory, or semantic trust boundary.
