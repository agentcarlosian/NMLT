# Public pivot pre-PR review

Date: 2026-08-31
Scope: complete active pivot from `origin/main`, including the public-surface
patch prepared for the final stack layer
Status: model-assisted pre-PR review; not a substitute for maintainer or external
human review

## Independent passes

Three independent read-only review agents examined different boundaries:

- public GitHub, trust, security, governance, and documentation surfaces;
- semantic claims against the Lean and Rust implementation; and
- commit ancestry and possible PR decomposition.

An authenticated second model-family pass then reviewed the same pivot
independently, with special attention to static versus dynamic semantics,
theorem-premise use, source/artifact identity, axiom policy, and PR dependencies.
No reviewer was allowed to modify the worktree.

## Corrections made from review

- Distinguished the static pair-state product from the dynamic authority-world
  step relation and stated that no correspondence theorem currently joins them.
- Stated that the source digest identifies supplied bytes but does not verify
  Rust elaboration.
- Stated that conditional dynamic step lifting proves neither step existence nor
  reachability.
- Stated that `ResourceWeakRefinement` has no separate path/trace adequacy theorem.
- Replaced `_isNecessary` Lean control names with rejection claims and rewrote
  the associated claim table so formation failures are not presented as
  minimality results.
- Documented the current closed-product interface, peer-hiding, capability-reuse,
  and surface-to-resource-lowering limitations.
- Rebuilt the active trust inventory and added a gate for links, trusted paths,
  removed component names, and tracked PDFs.
- Tightened and cross-checked the axiom policy. The focused behavior theorem audit
  excludes `Classical.choice`; the package-wide NanoDA allowlist documents its use
  by executable artifact decoding and finite premise decisions.
- Fixed the NanoDA constant-enumeration script for Lean 4.30 by importing
  `Lean.Meta.Basic`.

## Remaining research boundaries

These are explicit follow-up work, not claims made by the current pivot:

1. unify static behavior and dynamic authority worlds;
2. prove path/trace adequacy for the refinement relation;
3. derive an initial dynamic step and reachability from the artifact;
4. preserve remaining open ports, visibility, direction, and payload in product;
5. make received affine authority available to later receiver actions; and
6. revisit fairness and hidden divergence only after those safety foundations.

## Verification completed

- Rust formatting, compilation, Clippy with warnings denied, and all tests;
- byte-for-byte primary artifact reproduction and non-authoritative exploration;
- public link, trust-path, removed-component, and tracked-PDF hygiene;
- Lean build, no-`sorry` scan, four fail-closed artifact mutations, and focused
  theorem axiom audit; and
- pinned exporter plus independent NanoDA validation of the full `NMLT` module.

No branch or pull request was pushed as part of this review.
