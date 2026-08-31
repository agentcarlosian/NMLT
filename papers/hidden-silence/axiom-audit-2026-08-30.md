# Axiom audit — Paper 1 post-pivot declarations

**Date:** 2026-08-30

**Toolchain:** Lean 4.30.0

**Gate:** tools/check_metatheory.sh from a clean worktree

| Declaration | Reported axioms |
|---|---|
| NMLT.Behavior.ResourceBehavior.liftParallel | propext |
| NMLT.Artifact.SemanticClosure.Certificate.lifted | propext |

The same gate:

- builds the complete active Lean target;
- accepts the canonical behavior artifact and confirms its asserted digest
  matches the separately supplied source bytes;
- rejects a stale source digest;
- rejects a malformed transfer/receive profile;
- rejects a structurally valid artifact whose visible steps fail refinement;
- scans for sorry, sorryAx, admit, native_decide, and project axiom
  declarations.

The retained M9 elaboration-validation metatheory reports propext and, for
quotient-based declarations, Quot.sound. Those declarations are not used to
claim verified compilation of the new behavior artifact.

Later dynamic declarations and the live allowlist are audited in
`mechanization/lean/AXIOMS.md`. This dated report does not supersede that
file.
