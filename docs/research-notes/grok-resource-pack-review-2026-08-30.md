# Grok resource pack quarantine review

This note records why the accompanying working-tree snapshot is preserved on a
quarantine branch rather than merged into NMLT's active development line.

- `WeakResourceCongruence.liftParallelResources` transforms resource profiles
  indexed by `ParallelLabel`, but is not tied to the open-system transition
  relation, connection formation, or enabled product steps. It therefore does
  not establish resource-aware behavioral congruence.
- `InfiniteHiddenPath` supplies a hidden transition stream but has neither an
  initial-state premise nor a fairness predicate. It is a divergence witness,
  not an `I-FAIR` counterexample.
- The Rust hidden consume, grade, and rely cases all stop at the same
  `HiddenActionHasResources` formation check, so they do not independently
  exercise the corresponding refinement obligations.
- The M11 evidence manifests and durable axiom inventory were not updated for
  the new theorem names.

The useful counterexample shapes are retained for later restatement over a
single resource-bearing behavior semantics. The detached lifting theorem,
fairness claim, and Rust test architecture are not approved for main.
