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

## Language and implementation

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
- [Historical records](history.md)
- [Public pivot pre-PR review](reviews/public-pivot-pre-pr-2026-08-31.md)

Any dated handoff, reproduction report, completion audit, test report, or
research note is a historical record. It can explain how the project arrived
here, but it cannot override the current architecture, roadmap, security
inventory, or semantic trust boundary.
