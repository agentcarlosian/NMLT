# Architecture

## Pipeline

```text
source
  → lossless syntax tree
  → name and module resolution
  → typed behavioral core
  → verification-condition and transition-system IRs
  → backend adapters
  → checked evidence or structured witnesses
  → optional execution, tests, monitors, and trace refinement
```

No downstream stage may upgrade the assurance class produced by an upstream
stage. A backend adapter translates results; it does not reinterpret bounded
evidence as proof.

## Planned components

- `nmlt-core`: shared source, diagnostic, syntax, and evidence structures.
- `nmlt-cli`: stable command-line entry point.
- Future `nmlt-hir`: resolved and typed high-level representation.
- Future `nmlt-kernel`: small checker for type and proof evidence.
- Future `nmlt-transition`: explicit transition-system representation.
- Future `nmlt-model`: explicit-state exploration and trace minimization.
- Future `nmlt-smt`: symbolic backend protocol.
- Future `nmlt-monitor`: runtime monitor and trace refinement generation.
- Future `nmlt-lsp`: editor protocol implementation.

Crates are added only when a real boundary exists; empty architectural crates
are deliberately avoided.

## Trusted boundaries

The intended trusted core includes the formal kernel, evidence decoder, and the
minimal code needed to bind a checked artifact to its exact source and engine
identity. Solvers, tactics, parsers, generators, and AI systems may sit outside
the core when their output is independently checkable.

Backends without independently checkable certificates require an explicit
external-trust entry in their evidence manifests.

## Intermediate representations

NMLT should not force every backend to consume the same low-level encoding.
Two initial IR families are expected:

- a transition-system IR for initialization, actions, observations, fairness,
  and traces;
- a verification-condition IR for logical obligations, assumptions, scopes,
  and proof dependencies.

Both derive from the typed behavioral core and retain stable source spans.

## Current implementation

The current Rust workspace performs only structural scanning of top-level
system declarations, basic diagnostics, and generation of explicitly unknown
evidence scaffolds. It exists to validate repository boundaries and CLI
discipline before semantic implementation begins.
