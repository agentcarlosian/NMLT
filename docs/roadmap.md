# Roadmap

NMLT's roadmap is organized by semantic dependencies, not release dates.
Completion always means completion at the explicitly stated pre-alpha scope.

This document tracks the mathematical dependencies. The current
[execution plan](../Plan.md) adds the immediate checker baseline and R0–R5
product milestones for an executable language serving all three audiences;
it governs delivery priorities and status.

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
