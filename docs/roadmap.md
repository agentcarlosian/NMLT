# Roadmap

Dates are records or planning ranges, not release commitments. The detailed
gates and residual gaps live in [`Plan.md`](../Plan.md). “Complete” below means
complete only at the explicitly bounded research scope; it never means a
stable, general-purpose, verified language.

## Phase 0 — Foundation and research contract (complete 2026-07-18)

Apache-2.0 contribution terms, ten identity-frozen canonical examples,
NMLT/TLA+/Quint/P comparison fixtures, candidate behavior rules, a
claim-specific TCB threat model, and source/evidence identity requirements.

## Phase 1 — Syntax and semantic skeleton (complete 2026-07-18)

Lossless tokens and immutable CST, deterministic recovery, stable spans,
preservation formatting, declaration/action shells, and a partial untyped-core
projection. Raw expressions and surface-only declarations retain no inferred
semantic assurance.

## Phase 2 — Typed behavioral core (complete for the provider slice 2026-07-18)

Contextual provider elaboration, typed state/actions, explicit frames, affine
provider capability, and a Rust/Lean correspondence vector with checked kernel
theorems. Full surface-to-kernel compiler correctness is not claimed.

## Phase 3 — Behavior execution (bounded implementation complete 2026-07-18)

Deterministic bounded BFS accepts one frozen reference and refutes four
semantic mutants with source-bound structured traces. Suite v2 corrects
`NoBlindReplay` to current-state enabledness and freezes a distinguishing
one-shot regression. The Phase 3/release gate remains open until independent
clean-checkout reproduction is recorded.

## Phase 4 — Temporal properties and refinement (finite-fixture milestone complete 2026-07-18)

Finite fair-lasso checking, weak/strong fairness, stuttering, hiding,
one-step forward simulation, and accepted/rejected/unknown runtime-journal
conformance with independent evidence replay on a canonical finite fixture.
Provider `NoBlindReplay` is also checked with finite `always` semantics over a
nine-state reference observation graph; the blind mutant has a zero-step stem
and identity-stutter infinite lasso that the Python harness independently
replays. The source-to-observation-graph projection is manually audited, not
compiler-derived. No general temporal language, infinite-state, or
liveness-refinement proof is claimed.

## Phase 5 — Multiple verification engines (complete for one finite VC 2026-07-18)

One manual two-observable provider VC is checked by deterministic reachability
and separate finite inductiveness enumeration. Certificates are checked and
disagreement fails closed. The `proved` result applies only to that exact VC,
not to the full NMLT source translation.

## Phase 6 — Runtime and agentic workflow (complete as a deterministic baseline 2026-07-18)

Protected edit authority, structured feedback, localized repair, held-out
fixtures, negative controls, and a source-bound artifact graph linked to a
synthetic drift event. The three-task result is protocol-conformance evidence,
not an LLM capability or production-runtime claim.

## Phase 7 — Independent research extensions (one prototype track complete 2026-07-18)

The first track checks a product of declared cost, privacy, energy, and
uncertainty upper bounds in a separate annotated-plan language. Trusted
annotations, no privacy/physical model, and no verified correspondence between
the Rust analyzer and its kernel-checked Lean algebra keep its promotion gate
closed. Cubical, hybrid, probabilistic, alternative-grade, and open-system
tracks remain future work.

## Active milestone — M8 integration and release hardening

- reproduce the full gate from an independent clean checkout;
- review final TCB and evidence identities after implementation freeze;
- preserve the distinct assurance subject and ceiling of every research slice;
- decide whether the bounded provider work is ready for a `0.1.0` tag;
- pursue full frontend resolution/elaboration and verified source-to-IR
  connections without weakening existing evidence classes.
