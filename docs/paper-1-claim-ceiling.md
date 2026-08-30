# Paper 1 claim ceiling

**Working theme:** Resource-aware contextual refinement for an open
programming language

**Status:** active post-pivot ceiling

**Normative source fixture:**
`examples/pivot/visible_resource_sync.nmlt`

**Normative theorem module:**
`NMLT.Behavior.ResourceBehavior`

This document controls what Paper 1 may claim. Earlier prose about the detached
OpenSystem checker, the standalone label-to-resource lift, or a fairness
counterexample is historical and must not be submitted under this ceiling.

## Contribution that may be claimed

NMLT defines one resource-bearing open behavior semantics in Lean and proves a
conditional binary-composition theorem over its actual product transition
relation. A concrete resource-aware weak refinement lifts through composition
when:

- the complete wiring relation is preserved;
- connected actions are not hidden;
- directions and payloads are compatible;
- component ownership is an affine capability partition;
- synchronized transfer and receive profiles match exactly;
- concrete grades pointwise refine abstract grades and product grades add;
- synchronized reliance is discharged by the peer guarantee; and
- every hidden concrete step maps to equal abstract state and its complete
  resource profile refines stutter.

The theorem is
`NMLT.Behavior.ResourceBehavior.liftParallel`. It constructs a lifted witness
for `parallel concrete peer concreteConnection` and
`parallel abstract peer abstractConnection`; it does not reason through a
detached label-resource function.

The primary executable instance is
`NMLT.Examples.VisibleResourceSync.visibleResourceSync_lifts`. It mirrors
`examples/pivot/visible_resource_sync.nmlt`: a sender and receiver synchronize,
move one nominal affine `permit`, discharge `Authorized`/`Ready` contracts, and
compose grades `work=1` and `work=2` into `work=3`.

The exact source elaborates to
`examples/pivot/visible_resource_sync.behavior-core-v1.json`. Lean's artifact
decoder accepts that source-bound artifact. Rust exploration is illustrative
only and makes no theorem or model-check claim.

## Negative controls that may be claimed

`NMLT.Counterexamples.ResourceBehaviorControls` contains permanent theorems
showing concrete failures when the following premises are independently
omitted:

1. hidden-boundary isolation;
2. whole-wiring preservation;
3. capability partition;
4. exact transfer/receive matching;
5. pointwise grade preservation;
6. rely discharge; and
7. hidden-step resource compatibility.

The corresponding `.nmlt` fixtures test distinct compiler diagnostics. These
controls establish necessity for the encoded lifting construction and examples;
they do not claim model-universal minimality.

## Required non-claims

Paper 1 must state all of the following:

- No fairness field or behavior-indexed fairness theorem exists in this core
  version.
- No liveness, divergence, or infinite-trace property is transported.
- `HiddenDivergence` is quarantined and is not yet a fairness counterexample.
- The Rust frontend is not verified against the Lean semantics. Exact snapshots
  and source digests provide auditability, not compiler correctness.
- `nmlt-eval` is a bounded reference evaluator, not a prover or model checker.
- The theorem is finite, binary, and safety/resource scoped.
- No general composition, user-defined grade algebra, higher-order state map,
  probabilistic behavior, hybrid behavior, code generation, or runtime
  attestation result is claimed.
- The project is a pre-alpha language research system, not a complete verified
  programming language.

## Trusted boundary and axiom report

The claimed proof artifacts use Lean 4.30.0 and the hand-written modules in the
active `NMLT` library. The clean gate forbids `sorry`, `sorryAx`, `admit`,
`native_decide`, and project-declared axioms.

Both `liftParallel` and `visibleResourceSync_lifts` report exactly `[propext]`.
The approved foundations are documented in
`mechanization/lean/AXIOMS.md`. No Mathlib or Aeneas dependency is required by
the active Lean package.

## Publication gate

The current TeX draft predates this pivot and is not submission-ready. Before a
paper tag is cut, its theorem names, figures, examples, and reproducibility
appendix must cite only the unified theorem, the primary source/artifact pair,
and the active controls above. The quarantined branch and historical release
may be discussed as provenance, not as current proof evidence.
