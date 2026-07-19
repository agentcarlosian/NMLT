# Changelog

All notable project changes will be recorded here.

The format follows Keep a Changelog principles. NMLT does not yet make semantic
versioning compatibility promises.

## Unreleased

### Added

- Apache-2.0 project governance, research charter, RFC/decision process,
  architecture, calculus, language, threat-model, and evidence contracts.
- Ten source-identity-frozen canonical examples and comparative NMLT, TLA+,
  Quint, and P provider fixtures with explicit validation scopes.
- Lossless tokens and immutable CST, deterministic recovery, stable diagnostic
  spans, preservation formatter, declaration/action shells, partial untyped
  projection, negative controls, and structural CLI views.
- Typed executable provider slice with explicit frames, affine capability
  tracking, deterministic bounded BFS, and structured counterexamples.
- Pinned Lean provider-kernel mechanization for preservation, progress/blocked
  states, frames, capability use, and property indexing; no full compiler
  correctness claim.
- Source-bound provider result reproduction, benchmark integrity validation,
  canonical evidence readback, and adversarial stale/forged-evidence controls.
- Finite `always`/eventuality/lasso checking, explicit weak/strong fairness,
  stuttering/hiding, finite forward-simulation refinement, and three-valued
  runtime-journal checking with independent evidence replay, including a
  manually projected provider `NoBlindReplay` observation graph.
- Finite Boolean VC IR with independent reachability and inductiveness routes,
  checked witnesses/certificates, SMT-LIB and Lean export protocols,
  model-test hooks, and fail-closed raw-result composition.
- Authority-bounded deterministic repair-protocol baseline over three
  hand-authored held-out fixtures and a source-bound graph linked to a
  synthetic runtime-drift event; this is not LLM capability evidence.
- Independent graded-resource experiment for declared cost, privacy, energy,
  and uncertainty bounds, including unknown-preserving arithmetic, algebra
  controls, schema-valid evidence, and a Lean-checked mathematical product
  algebra; the plan analyzer and Rust correspondence remain unverified.

### Changed

- Corrected provider suite v2 `NoBlindReplay` from successor-state to
  current-state enabledness and added a one-shot replay regression with a
  zero-transition counterexample. The historical-formula side of that control
  is explicitly limited to the Phase 3 action-step/terminal-stutter profile;
  RFC 0007 and Phase 4 use universal identity-stutter closure.
- Updated the execution plan and roadmap to record Phases 0–7 at their exact
  bounded scopes. The complete gate reproduced from a fresh clone of
  `fcf2317b9b92a59d1937d08ced4e9c476b30bebd`; a `0.1.0` tag is deliberately
  deferred while the documented promotion gaps remain.
