# Graded-resource benchmark

This directory freezes one reference and three source-level negative controls
for RFC 0012. `provider_pipeline.nmltg` claims only that, under its declared
annotations and the composition rules in the RFC, its product bound is:

```text
cost_ticks             66 <= 100
privacy_micro_epsilon  500000 <= 550000
energy_microjoules     155 <= 180
uncertainty            declared:47000 <= declared:60000
```

It does **not** claim that the annotations match a deployed implementation, that
`500000 micro-epsilon` is a differential-privacy proof, or that uncertainty is
a calibrated probability. Those links require separate evidence.

The controls require these outcomes:

- `privacy_budget_violation.nmltg`: `exceeded`, specifically the privacy
  coordinate (`500001 > 400000`);
- `unknown_iteration.nmltg`: `unknown`, never `within_budget`;
- `invalid_uncertainty.nmltg`: parse rejection;
- an in-memory noncommutative word algebra: rejected by the extension's
  declared *commutative product* law profile. This does not reject
  noncommutative effect quantales in other extensions.

Run `python3 tools/check_graded_evidence.py --update` to regenerate the
evidence, then rerun without `--update` for readback and determinism checks.
