# Roadmap

NMLT's roadmap is organized by semantic dependencies, not release dates.
Completion always means completion at the explicitly stated pre-alpha scope.

## Now: make the pivot publicly coherent

- align README, security, contributor, RFC, example, and Lean documentation
  with the language-and-mathematics architecture;
- remove active claims about deleted Build Week verifier systems;
- distinguish theorem premises from product-formation policy;
- distinguish source identification from verified translation; and
- reconstruct the large pivot as a reviewable PR stack.

## Next: one resource-bearing behavior

The immediate mathematical task is to integrate the authority-world state into
the normative `Behavior` rather than maintain separate static and dynamic
product semantics. Product formation must preserve the intended remaining open
interface, including action visibility, direction, and payload.

Deliverables:

- one product state and step relation;
- a relation to or replacement for the current static product;
- exact ownership/consumption/transfer effects;
- explicit minimal theorem premises; and
- negative controls that demonstrate failure of weakened theorem statements,
  not only failure of language formation.

## Then: `behavior-core-v2` and dynamic witnesses

The next artifact version will encode initial authority and dynamic step data.
Lean should construct the primary fixture's initial synchronized step rather
than only prove that any supplied step can be lifted.

Deliverables:

- deterministic v2 schema and fixture;
- source identification plus repository-level compiler reproduction;
- artifact-derived initial state and synchronization witness;
- exact one-time transfer theorem for that decoded step; and
- malformed-world and forged-step controls.

## Then: finite paths and reusable received authority

Define reachability over dynamic worlds and make received affine capabilities
available to later receiver actions without allowing copying, fabrication, or
retention after transfer.

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
