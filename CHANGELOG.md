# Changelog

NMLT is pre-alpha and does not yet promise compatibility.

## Unreleased

### Language and semantics

- Recentered the repository on the `.nmlt` programming language with normative
  behavior semantics in Lean.
- Added typed ports, action polarity and payload binding, affine capability
  transfer, additive grades, contract facts, binary composition, explicit
  connections, observations, action hiding, and total refinement state maps to
  the source-to-core path.
- Added deterministic `behavior-core-v1` with exact source binding, typed term
  ASTs, action resource profiles, compositions, and refinement witnesses.
- Added the resource-bearing Lean `Behavior`, real product-step relation,
  resource-aware weak refinement, conditional `liftParallel` theorem, primary
  fixture instantiation, and seven permanent premise controls.
- Added a Lean artifact decoder with source-digest checking and fail-closed
  resource/contract validation.
- Added `nmlt-eval` for explicitly non-verifying artifact exploration.

### CLI and gates

- Added `elaborate --emit-core` and artifact-based `explore`.
- Removed `model-check` and `evidence` from the active CLI.
- Replaced the default gate with Rust formatting, Clippy, compilation, tests,
  fixture/artifact reproduction, Lean build, no-`sorry` policy, decoder controls,
  and theorem axiom audits.
- Removed unused Mathlib and Aeneas dependencies from the active Lean package.

### Removed from the active branch

- Contest verifier engines, temporal/OpenSystem checkers, standalone grade
  analyzer, open Rust kernel, Paper 1 adapter, agent evaluator, certificate
  tools, benchmark/evidence corpora, comparison harnesses, release scripts, and
  contest schemas.

The former release remains immutable at tag `build-week-judge-demo-2026`
(`0417f6e`). The reviewed follow-up resource patch is preserved on
`codex/quarantine-grok-resource-pack` and is not merged here.
