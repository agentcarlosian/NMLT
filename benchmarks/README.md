# Benchmarks

NMLT benchmarks measure semantic discrimination, evidence accuracy, and
counterexample quality—not parser acceptance alone. The frozen
`provider-attempt-seeded-defects-v2` suite contains one reference, four
single-defect mutants, three benchmark-integrity controls, and one executable
semantic-regression control. It supersedes v1 because the original
`NoBlindReplay` formula checked successor-state enabledness and therefore
permitted one replay. The complete correction record is in
[`provider-attempt/V2-CORRECTION.md`](provider-attempt/V2-CORRECTION.md).

## Honest result boundary

The provider fragment now has deterministic explicit-state execution. The
reference is currently `model_checked`; each of the four semantic mutants is
currently `refuted` with a counterexample. Every result binds the exact source,
engine source set and executable, algorithm, platform, assumptions, and bounds.

`model_checked` is not `proved`. For the reference it means the reachable
frontier was exhausted with `max_states = 10000` and `max_depth = 100`, and all
four properties held on the explored states and transitions. It establishes
neither an unbounded theorem nor correctness beyond the executable fragment
and trusted components recorded in the result. The manifest explicitly
excludes promotion to `proved`.

The malformed fixture still yields a frontend diagnostic rather than semantic
evidence. The vacuity and weakened-property fixtures remain integrity controls,
not successful semantic cases. The one-shot replay fixture is executable under
the Phase 3 action-step profile: `next` ranges over declared action successors,
and identity stutter is inserted only when a state is terminal. In that profile
the superseded next-state formula is `model_checked`, while the corrected
current-state formula is `refuted` at the initial state. This control does not
apply to RFC 0007/Phase 4 universal identity-stutter closure, under which the
superseded formula is also refuted at the initial state.

Files in `provider-attempt/expected-counterexamples/` are human-reviewed
expected witness shapes frozen before execution. Their mandatory
`expected_not_observed` status describes their origin as oracles and keeps
them distinct from checker-produced traces; validation now compares each
observed refutation trace against its corresponding oracle.

## Frozen suite

| Case | Frozen oracle | Current bounded result | Witness |
|---|---|---|---|
| `provider-attempt-reference` | all four properties hold within the frozen bounds | `model_checked` | none |
| `dispatch-before-authorize` | `DispatchRequiresArm` refuted | `refuted` | one-transition unauthorized dispatch |
| `blind-replay` | `NoBlindReplay` refuted | `refuted` | zero-transition initial state with replay enabled |
| `response-binding` | `EvaluationRequiresIntactResponse` refuted | `refuted` | one-transition unbound evaluation |
| `passing-selection` | `SelectionRequiresPass` refuted | `refuted` | one-transition failing selection |

Each case has a JSON intent capsule that binds exact source bytes, property
contracts, provenance, positive and forbidden scenarios, anti-vacuity
obligations, result classes, and expected witnesses. Changing any frozen
element requires a new suite version; an observed result must not rewrite the
oracle in place. The historical oracle token
`holds_within_frozen_future_bounds` records that it was fixed before execution;
the source-bound current result records that it has now been observed.

## Integrity controls

- `malformed-unclosed-system.nmlt` proves the frontend still rejects an
  unclosed system rather than treating a fixture as valid evidence.
- `vacuous-dispatch-property.nmlt` retains the canonical property but makes
  dispatch unreachable. A future semantic pass alone is insufficient; the
  anti-vacuity obligation must fail.
- `weakened-dispatch-invariant.nmlt` retains the property name while replacing
  its formula with a tautology. Its property-identity mismatch must fail
  closed.
- `one-shot-replay-regression.nmlt` moves from `indeterminate` to
  `reconciled` in one dispatch. Under the explicitly scoped Phase 3
  action-step/terminal-stutter profile it distinguishes the superseded
  formula, which accepts that one replay, from the corrected property, which
  refutes enabledness at state 0. It is a semantic control rather than a fifth
  primary mutant; it is not a universal-stutter temporal claim.

## Identities

NMLT source and source-set IDs use the normative algorithm in
`docs/artifact-identity.md`. Benchmark property, intent, expected-witness, and
provenance and persisted result objects use a deliberately restricted
canonical JSON subset:

1. JSON must contain only objects, arrays, strings, integers, booleans, and
   null; duplicate names, non-finite values, and floating-point values fail.
2. Remove the object's identity member (`property_id`, `capsule_id`,
   `witness_id`, `provenance_id`, or `result_id`).
3. Serialize UTF-8 JSON with lexicographically sorted names and no optional
   whitespace.
4. Hash `domain || u64be(length) || bytes` with SHA-256.

The exact domains and prefixes live in
`tools/validate_benchmark_integrity.py`. These IDs bind benchmark contracts;
they are not substitutes for the future typed-core semantic claim identity.

## Provenance

`provider-attempt/provenance.json` records the exact Technicusverus Git
revision, source paths, SHA-256 blob digests, and upstream MIT license digest.
The NMLT fixtures are independent re-encodings and copy no upstream source
text. A clean NMLT checkout validates the frozen metadata without needing the
source repository; maintainers with the upstream checkout can additionally
verify every recorded blob.

## Validation

The integrity validator uses only the Python standard library. It validates
the five benchmark-integrity schemas plus the persisted-result and model-report
schemas, recomputes every source and artifact identity, checks all cross-file
bindings, compares observed traces with frozen witness oracles, enforces the
bounded result-class boundary, confirms that the three negative controls
remain observable, and freezes the exact one-shot semantic regression. The
model-report checker independently reruns each case twice and rejects
nondeterministic or stale persisted bytes.

```bash
python3 tools/validate_benchmark_integrity.py
python3 tools/validate_benchmark_integrity.py --self-test
python3 tools/check_model_reports.py
python3 tools/validate_benchmark_integrity.py \
  --upstream /path/to/technicusverus
```

The self-test and model-report commands form the CI readback pair.
`--self-test` proves that a `model_checked`-to-`proved` promotion, stale source
identity, stale result binding, and missing control are all rejected.

## M11 metatheory evidence

[`results/open-composition/m11-001a-evidence.json`](results/open-composition/m11-001a-evidence.json)
is a claim-specific readback artifact for the finite exact-action
open-composition slice. It is not a generic provider evidence manifest and it
does not claim Rust/Lean correspondence. The checker binds the exact Lean
source set, pinned toolchain, theorem and positive/negative control handles,
trusted-component inventory, schema, and checker identities, then compares the
declared axiom sets with actual `#print axioms` output from the pinned build.

```bash
python3 tools/check_open_composition_evidence.py
./tools/check_metatheory.sh
```
