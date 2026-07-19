# ADR 0002: In-repository immutable green tree

- Status: Accepted
- Date: 2026-07-18

## Context

NMLT needs byte-exact round trips, stable spans, error recovery, and later
incremental tooling. The current structural scanner discards trivia and cannot
support those properties.

## Decision

Use a rowan-style immutable green tree implemented in `nmlt-core`: interior
nodes contain only kind and ordered children; token leaves carry kind and byte
length; red views add parent and absolute-offset context. Trivia and error
tokens remain leaves. Parser events are folded into the green tree, allowing
recovery nodes without manufacturing semantic declarations.

Phase 1 begins with the same lossless token representation and parser-facing
spans. The green-node arena is added when the declaration grammar lands. No
external syntax-tree crate is placed in the trusted frontend until benchmarks
show that maintaining the small representation is the larger risk.

## Consequences

- Exact source is reconstructed from ordered token slices.
- Formatting may replace trivia but parsing never discards it.
- Syntax identity remains source-byte identity; a green-tree cache is not a
  canonical semantic identity.
- Incremental reuse is possible by hash-consing immutable nodes later.
- We own recovery and offset invariants and must test them exhaustively.

## Rejected alternatives

- An AST-only parser loses comments and malformed text.
- A concrete tree with parent pointers in every node complicates sharing and
  incremental reuse.
- Adopting a third-party green-tree library immediately would save code but add
  an unfrozen dependency before token and recovery requirements are measured.
