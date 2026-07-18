# ADR 0001: Begin with a small Rust workspace

- Status: accepted
- Date: 2026-07-18

## Context

NMLT needs deterministic command-line tooling, strong internal types, portable
binaries, and a path to performance-sensitive parsing and model exploration.
The semantic kernel is not yet defined, so a large crate graph would create
false architectural certainty.

## Decision

Begin with two crates:

- `nmlt-core` for shared syntax, diagnostics, and evidence structures;
- `nmlt-cli` for the stable command-line entry point.

Use no third-party Rust dependencies in the initial scaffold. Add crates and
dependencies only when an implemented boundary or reviewed requirement exists.

## Consequences

The initial parser is intentionally structural and incomplete. JSON emission is
small and internal. Mature parsing, serialization, LSP, and solver libraries may
be adopted later through ordinary dependency review.

## Alternatives

- A large compiler workspace was rejected because most boundaries would be
  speculative.
- Python was rejected for the eventual kernel/frontend implementation, though
  it may remain useful for research and benchmark tooling.
- Implementing the semantic core directly in Lean remains a complementary
  research path rather than the initial CLI implementation language.
