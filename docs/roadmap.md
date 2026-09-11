# Roadmap

NMLT's roadmap is organized by semantic dependencies, not release dates.
Completion always means completion at the explicitly stated pre-alpha scope.

This document tracks the mathematical dependencies. The current
[execution plan](../Plan.md) adds the immediate checker baseline and R0–R5
product milestones for an executable language serving all three audiences;
it governs delivery priorities and status.

R2 completed at local pre-alpha scope on 2026-09-09; see its
[validation record](reviews/r2-completion-2026-09-09.md). The implementation
history begins with the [first local execution increment](r2-local-execution.md)
adds finite source-driven run/replay over shared evaluator steps. The
[bounded job runtime](../crates/nmlt-runtime/README.md) now adds durable control,
uncertain recovery, and a subprocess worker prototype. The [source job increment](r2-source-jobs.md)
connects typed worker effects with explicit limits, journal replay, and recovery
inspection. The [initial Lean and async host API](r2-lean-async.md) now checks
legacy proof templates and supports runtime start/poll/cancel/collect. The
[scoped source controls](r2-source-async.md) now expose worker and fixed-template
Lean jobs with affine checks and source/session replay. Affine handle transfer, closed Init proof terms, finite safety invariants,
project tooling and source resumption are implemented; the
[R2 audit](r2-completion-tracker.md) records the completed validation.
The [pure source workflow increment](r2-pure-workflows.md) adds named function
entries, structured inputs, records, bounded lists/folds, typed outcomes, and evaluation/replay. These
constructs currently have an executable-only interpretation.
The [source package increment](r2-source-packages.md) adds reusable libraries,
per-file resolution, and version 3 replay over the complete import closure.

R3 is in progress. Its [first local Lean task profile](r3-lean-tasks.md) binds
exact targets and saved source environments, checks separate candidates, and
exports independently checked proofs with fresh rechecking and readable patches.
The current profile also discovers local source imports across selected source
and vendor roots, and streams proof exports up to 16 MiB to files with exact
byte counts and digests. Accepted results also retain readable declaration
dependency graphs, reconstructed during fresh rechecking. Small-project and
proof-export bounds remain explicit.
Bound declaration lookup and native diagnostic ranges now support candidate
repair, with context kept separate from independent proof acceptance.

## Now: make the pivot publicly coherent

- align README, security, contributor, RFC, example, and Lean documentation
  with the language-and-mathematics architecture;
- remove active claims about deleted Build Week verifier systems;
- distinguish theorem premises from product-formation policy;
- distinguish source identification from verified translation; and
- reconstruct the large pivot as a reviewable PR stack.

## Implemented: one resource-bearing behavior

R1's first increment implements `ResourceDynamics.Behavior`, combining control
and authority in its state, initialization, observations, and completed steps.
The binary constructor preserves remaining open action metadata and leaf
effects. Constructed nested examples exercise a transfer and a later monitor
step. The v1 artifact path still uses the retained static and auxiliary dynamic
semantics. The opt-in v2 path checks decoded execution against the unified model.

Deliverables:

- one product state and step relation;
- a relation to or replacement for the current static product;
- exact ownership/consumption/transfer effects;
- explicit minimal theorem premises; and
- negative controls that demonstrate failure of weakened theorem statements,
  not only failure of language formation.

## Implemented: `behavior-core-v2` and dynamic witnesses

V2 encodes independently derived capability namespaces and initial authority.
Supplied path witnesses carry dynamic state and action data. Lean constructs the
primary fixture's initial synchronized step and its initialized abstract image.

Deliverables:

- deterministic v2 schema and fixture;
- source identification plus repository-level compiler reproduction;
- artifact-derived initial state and synchronization witness;
- exact one-time transfer theorem for that decoded step; and
- malformed-world and forged-step controls.

## Implemented: finite paths and received authority

Define reachability over dynamic worlds and make received affine capabilities
available to later receiver actions without allowing copying, fabrication, or
retention after transfer.

R1 defines finite paths, vacancy preservation, and ownership-origin results in
the unified model. V2 source continuation supports receive then consume or
retransfer. Rust/Lean comparisons cover a resource-free Bool/Unit/enum fixture
and the complete eight-state, twelve-transition resource fixture. General source
composition, runtime effects, and broader correspondence remain later work.

Deliverables:

- finite-path semantics;
- receive-then-consume and receive-then-transfer examples;
- path-level ownership invariants; and
- Rust/Lean operational cross-checks with Rust remaining non-authoritative.

## Later: fairness and liveness

Fairness returns only after the unified safety/resource semantics and
reachability layer are stable. This milestone will revisit hidden divergence,
define behavior-indexed fairness, and state exactly which liveness properties
survive hiding and composition.

## Longer horizon

- richer mathematical value and type layers;
- general and n-ary composition;
- user-defined grade algebras;
- probabilistic, hybrid, and quantitative behaviors;
- verified elaboration or translation validation;
- code generation and runtime observation; and
- additional languages and mathematical techniques under the wider NMLT
  research program.

See [`Plan.md`](../Plan.md) for gates and residual boundaries. Pre-pivot
milestone records are indexed as history in [the documentation map](README.md).
