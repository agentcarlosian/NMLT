# Phase 0 closeout and Phase 1 completion record

- Closed: 2026-07-18
- Phase 1 started: 2026-07-18
- Phase 1 bounded frontend gate completed: 2026-07-18
- Exit decision: passed with explicitly recorded research obligations

## Phase 0 decisions and artifacts

| Exit item | Decision / artifact | Validation |
|---|---|---|
| License | Apache License 2.0; inbound contributions use the same terms; no CLA presently | Full `LICENSE`, Cargo metadata, contributing and governance text |
| Ten examples | `nmlt-canonical-v1` fixes C01–C10, intents, claim handles, negative controls, and exact source IDs | `python3 tools/canonical_examples.py` |
| Comparisons | One provider-attempt protocol and its claims encoded in NMLT, TLA+, Quint, and P | Current validation record: bounded NMLT model check, TLC 7-state exploration, and Quint 0.32.0 typecheck; corrected P sources are byte-bound but unvalidated because P/.NET is unavailable and the earlier run predates the correction |
| Behavior types | RFC 0001 defines candidate formation, action/frame, temporal, composition, and weak-refinement rules | Candidate rules plus the later provider-effect Rust/Lean slice; no full-calculus soundness claim |
| TCB threat model | Claim-specific components, boundaries, attack stories, security invariants, and severity calibration | `docs/threat-model.md` plus `security/trusted-components.toml` |
| Identity | Domain-separated SHA-256 source/source-set IDs and canonical artifact-specific evidence IDs | RFC 0004 plus executable corpus, benchmark, result, and evidence readback tools; no signature or transparency log |

Phase 0 established a falsifiable candidate, exact fixtures, and rules that
prevent structural or bounded results from being presented as stronger
evidence. Later phases implemented selected slices; they do not retroactively
turn the Phase 0 calculus into a general soundness theorem. The comparative
encodings also have different validation scopes and are not a proved
cross-language equivalence result.

## Phase 1 result

- Accepted RFC 0003 lexical grammar.
- Accepted ADR 0002 for an in-repository immutable green tree.
- Added a public lossless token stream covering every byte, including trivia
  and malformed suffixes.
- Added an immutable CST and deterministic declaration recovery for modules,
  surface declarations, systems, state, actions, requirements, explicit
  updates, capabilities, properties, observations, and hiding shells.
- Added a documented partial CST-to-untyped-core projection. Expressions stay
  source-spanned raw terms; malformed and surface-only constructs remain
  explicit and receive no semantic interpretation.
- Added stable diagnostics, negative controls for duplicate declarations,
  malformed update targets, undeclared-state deferral, implicit-mutation
  recovery, delimiter errors, and recovery grouping.
- Added preservation formatting, idempotence, and corpus-wide parse/format
  round trips, plus `check`, `inspect`, and `tokens` CLI views.

## M1 gate result

- All ten canonical examples round-trip through the lossless frontend.
- Every frontend diagnostic has a UTF-8-valid half-open byte span, including
  stable zero-width insertion and file-level spans.
- Malformed controls fail deterministically and recovery nodes never become a
  semantic acceptance path.
- `is_structurally_complete`, `check`, and the structural `evidence` scaffold
  remain syntax/structure results, not type checking or verification.

## Remaining integration boundary

Module/data/record/function shells and raw expressions still need complete name
resolution, typing, totality, and executable semantics in the general surface
language. The Phase 2 provider checker is a separate narrow contextual
elaborator; no full surface-to-provider compiler-correctness result is claimed.

The authoritative task status remains `Plan.md`.
