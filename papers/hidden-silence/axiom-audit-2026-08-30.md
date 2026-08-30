# Axiom audit — Paper 1 post-pivot declarations

**Date:** 2026-08-30

**Toolchain:** Lean 4.30.0

**Gate:** tools/check_metatheory.sh from a clean worktree

| Declaration | Reported axioms |
|---|---|
| NMLT.Behavior.ResourceBehavior.liftParallel | propext |
| NMLT.Examples.VisibleResourceSync.visibleResourceSync_lifts | propext |

The same gate:

- builds the complete active Lean target;
- accepts the source-bound canonical behavior artifact;
- rejects a stale source digest;
- rejects a malformed transfer/receive profile;
- scans for sorry, sorryAx, admit, native_decide, and project axiom
  declarations.

The retained M9 elaboration-validation metatheory reports propext and, for
quotient-based declarations, Quot.sound. Those declarations are not used to
claim verified compilation of the new behavior artifact.
