# NMLT execution plan

- Status: pre-alpha language and mathematics research
- Active architecture: Rust frontend and evaluator; Lean behavioral semantics
- Current result: finite resource-aware composition plus auxiliary dynamic
  authority-world one-step lifting
- Next milestone: unify dynamic worlds with the normative behavior and artifact
  model
- Updated: 2026-08-31

## Program direction

NMLT is building a new programming language and new compositional mathematics
as one project. The implementation is not organized around a portfolio of
verifiers. One source language elaborates to an inspectable behavioral core;
Lean gives that core its current normative meaning; executable Rust tooling
supports language development without issuing proof claims.

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

## Current honesty boundary

The static `Behavior.parallel` semantics and the dynamic
`ResourceWorld.ProductStep` semantics are currently separate layers. The
dynamic layer is not yet a `Behavior`, has no artifact-derived initial-step
existence theorem, and has no reachability or trace theorem. Product formation
checks several language invariants that are not all logical dependencies of the
current lifting proof. The source digest identifies supplied source bytes but
does not verify Rust elaboration.

Public documentation must preserve those distinctions.

## Next milestones

### M1 — Unified resource-bearing behavior

- Make dynamic authority part of the normative behavior state rather than a
  parallel semantic layer.
- Preserve observations and the remaining open interface through product
  construction, including peer-side hiding, direction, and payload.
- Prove a projection or replacement theorem relating the present static and
  dynamic step relations.
- Re-state product formation separately from the minimal logical premises of
  step lifting.

Exit gate: one behavior object and one product-step relation support the
resource theorem without duplicated semantic authority.

### M2 — `behavior-core-v2` and artifact-derived execution witness

- Encode initial authority, dynamic state, and the data needed to form enabled
  local and synchronized steps.
- Reproduce the positive artifact byte-for-byte.
- Construct in Lean an initial decoded state and an actual synchronized step
  for the primary source fixture.
- Prove the decoded step moves authority exactly once and is accepted by the
  unified lifting theorem.
- Add tampering controls for initial worlds, ownership, and claimed steps.

Exit gate: the artifact checker proves conditional lifting and exhibits the
fixture's initial dynamic step. This remains source identification until a
separate translation-validation or compiler-correctness result exists.

### M3 — Finite reachability and affine continuation

- Define reachable dynamic states from decoded initial states.
- Permit a received affine capability to enter the receiver's post-step
  authority context and be consumed or retransferred exactly once.
- Prove ownership uniqueness and explained authority changes over finite paths.
- Cross-check the Lean relation against non-authoritative Rust exploration.

Exit gate: the primary fixture and a receive-then-use fixture have checked
finite paths, with independent negative controls for duplication and
fabrication.

### M4 — Behavior-indexed fairness

Only after M1–M3 are stable:

- reintroduce the quarantined hidden-divergence question;
- define fairness over the unified behavior rather than over labels alone;
- distinguish finite stuttering from infinite hidden divergence; and
- state the first liveness transport theorem with explicit fairness premises.

No current result transports liveness.

## Deferred research

Infinite state, probabilistic and hybrid behavior, general n-ary composition,
user-defined grade algebras, higher-order state maps, code generation, runtime
attestation, verified elaboration, and a production assurance system remain
outside the active milestones.

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
