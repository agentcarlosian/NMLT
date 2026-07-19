# Phase 0 closeout and Phase 1 kickoff

- Closed: 2026-07-18
- Phase 1 started: 2026-07-18
- Exit decision: passed with explicitly recorded research obligations

## Phase 0 decisions and artifacts

| Exit item | Decision / artifact | Validation |
|---|---|---|
| License | Apache License 2.0; inbound contributions use the same terms; no CLA presently | Full `LICENSE`, Cargo metadata, contributing and governance text |
| Ten examples | `nmlt-canonical-v1` fixes C01–C10, intents, claim handles, negative controls, and exact source IDs | `python3 tools/canonical_examples.py` |
| Comparisons | One provider-attempt protocol and four claims encoded in TLA+, Quint, and P | TLC: 7 reachable states; Quint 0.32.0 typecheck; P 3.1.0 compile plus 100 schedules |
| Behavior types | RFC 0001 defines candidate formation, action/frame, temporal, composition, and weak-refinement rules | Under review; mechanization remains the acceptance gate |
| TCB threat model | Claim-specific components, boundaries, attack stories, security invariants, and severity calibration | `docs/threat-model.md` plus `security/trusted-components.toml` |
| Identity | Domain-separated SHA-256 source/source-set IDs and RFC 8785 evidence IDs | RFC 0004; corpus reference implementation; evidence JCS implementation intentionally pending |

Phase 0 does not claim the behavior calculus is sound or implemented. It
establishes a falsifiable candidate, exact fixtures, and rules that prevent
structural or bounded results from being presented as stronger evidence.

## Phase 1 work started

- Accepted RFC 0003 lexical grammar.
- Accepted ADR 0002 for an in-repository immutable green tree.
- Added a public lossless token stream covering every byte, including trivia
  and malformed suffixes.
- Migrated structural system recognition to that stream and extended delimiter
  checking to braces, parentheses, and brackets.
- Added `nmlt tokens <file>` and lexer round-trip/negative-control tests.

## Remaining M1 gate

- Implement parser events and immutable green nodes.
- Parse modules, types, systems, state, actions, guards, and updates.
- Define canonical diagnostic snapshots.
- Add recovery, formatter round-trip, and idempotence tests.
- Round-trip all ten canonical examples without converting parse success into
  a semantic assurance claim.

The authoritative task status remains `Plan.md`.
