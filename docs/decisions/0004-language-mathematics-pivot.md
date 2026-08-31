# 0004: Language-and-mathematics pivot

- Status: Accepted
- Date: 2026-08-31

## Context

NMLT's public repository grew around a Build Week verifier demonstration and
then accumulated several independent checking, evidence, runtime, agentic, and
grade experiments. The enduring research direction is different: create a new
programming language and develop the mathematics required to explain its
behavior, composition, authority, and refinement.

The previous architecture made Rust verifier implementations appear to be the
center of semantic authority. It also made independent research prototypes look
like one supported assurance product.

## Decision

NMLT will have one promoted language path:

```text
.nmlt source
  → Rust lossless frontend and typed elaboration
  → deterministic behavioral artifact
  → Lean-defined semantics and theorem premises
  → optional Rust exploration with no assurance claim
```

Lean owns the current behavioral definitions. Rust is the frontend, artifact
producer, retained ordinary typed-elaboration validator, and reference
evaluator. Exact artifacts support audit and reproduction but do not constitute
a verified compiler.

The active project prioritizes:

- first-class open boundaries, observations, resources, authority, grades, and
  rely/guarantee contracts;
- compositional refinement theorems over actual transition relations;
- explicit negative controls and narrow claim ceilings; and
- new mathematical and language techniques that grow from the same semantic
  core.

Fairness and liveness follow only after the safety/resource semantics,
authority worlds, artifact witnesses, and reachability are unified.

## Consequences

- Contest-only engines, schemas, corpora, scripts, and workflows leave the
  active branch.
- The immutable `build-week-judge-demo-2026` tag preserves the
  historical release.
- Historical RFCs and research notes remain records, not current architecture.
- Public documentation must distinguish current checked results from planned
  semantics.
- Security and trusted-component inventories name only active paths.
- The current static and dynamic product layers are disclosed as an integration
  gap until a unifying theorem or replacement lands.

## Alternatives rejected

- **Keep Rust verifiers as the product center.** This would optimize the
  repository around implementation breadth rather than language semantics.
- **Treat every research prototype as an active backend.** This creates an
  unreviewable trusted boundary and misleading public story.
- **Wait for a verified compiler before building the language.** Deterministic
  artifacts and explicit trust boundaries permit useful research without that
  overclaim.
