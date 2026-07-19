# Graded-resource extension

RFC 0012 is an executable Phase 7 research track for keeping quantitative
claims compositional without pretending that unlike resources are
interchangeable. Its result is useful now, but deliberately narrower than a
language-integrated graded type system.

## What exists

The implementation lives in `crates/nmlt-grades` and contains:

- a checked product grade for total work ticks, privacy micro-epsilon, energy
  microjoules, and a typed uncertainty certificate (`declared`, `hoeffding`,
  or `conformal`) carrying a ppm upper bound;
- a small S-expression parser and typed plan AST;
- sequence, exclusive choice, conservative parallel, and finite-repeat rules;
- a three-valued analyzer (`exact`, `exceeded`, or `unknown` at the budget
  boundary);
- a finite law-profile checker, including a deliberately noncommutative
  control;
- an executable evidence producer.

The matching mathematical algebra is kernel-checked in
[`NMLT/Grades/Algebra.lean`](../mechanization/lean/NMLT/Grades/Algebra.lean).

The reference source is `benchmarks/grades/provider_pipeline.nmltg`. Its exact
computed usage is `(66, 500000, 155, 47000)` and its budget is
`(100, 550000, 180, 60000)`. The claim is only about these declared
annotations under the RFC's rules.

## Metatheory capsule

For each fixed uncertainty family, let exact mathematical grades be
`G = Nat³ × {0..M}` for `M = 1,000,000`.
Order and maximum are componentwise. Sequential and conservative parallel
composition add the first three coordinates and use `min(M,u+v)` for the
fourth.

### Lemma 1: exact composition is a commutative monoid

Zero is an identity coordinatewise. Natural addition is associative and
commutative. Saturated addition is associative because
`min(M, min(M,a+b)+c) = min(M,a+b+c)`, and is commutative by commutativity of
addition. Finite products preserve these laws.

### Lemma 2: choice is a finite join

Componentwise maximum is associative, commutative, and idempotent. Numeric
zero is least in every coordinate, so it is the empty finite join. The
componentwise relation is exactly the order induced by this join.

### Lemma 3: composition distributes over nonempty choice

For natural coordinates,
`a + max(b,c) = max(a+b,a+c)`. For uncertainty, applying monotone truncation
at `M` to both sides preserves the equality. The result lifts coordinatewise
to the product.

The Lean capsule proves the family-homogeneous binary statement and its extension to every
nonempty finite choice. It does not claim distribution over the empty join:
zero is the sequencing identity in this profile, not an annihilator.

### Lemma 4: exact analysis is compositional

For every finite plan whose annotations are valid mathematical grades,
structural induction on the plan yields one exact grade determined only by
the grades of its immediate subplans and the matching algebra operation.
Finite repetition follows by induction on its count.

### Lemma 5: budget acceptance is conservative relative to annotations

Assume every atom grade is an upper bound in each coordinate, choice executes
at most one branch, and the composition interpretations are valid. By Lemma 4
and monotonicity, the computed plan grade is an upper bound. If it is
componentwise below the budget, the annotated plan is below the budget.

Lemmas 1–3, product-order laws, monotonicity, and equivalence of the Lean
Boolean budget test with componentwise order are mechanized in Lean 4.30.0.
Lemmas 4–5 remain conditional paper arguments because the Lean capsule does
not encode the Rust plan AST, parser, or analyzer. The crucial atom-soundness
and interpretation premises remain trusted. In Rust, exact naturals are
represented by `u64`; overflow or an absent repeat bound is lifted to
`unknown`, so the backend never uses wraparound as evidence. No verified
extraction or compiler-correctness theorem connects those Rust operations to
the Lean definitions.

## Reading the four coordinates

| Coordinate | Prototype meaning | Composition caveat |
| --- | --- | --- |
| `cost_ticks` | total abstract work | not latency and not inferred from execution |
| `privacy_micro_epsilon` | declared basic privacy-loss upper bound | no DP mechanism or sensitivity proof; parallel remains additive |
| `energy_microjoules` | declared total energy upper bound | no hardware or measurement model |
| `uncertainty` | family-tagged ppm upper bound (`declared`, `hoeffding`, or `conformal`) | only same-family bounds compose; the tag names an obligation but does not itself prove it |

The product comparison is componentwise. NMLT never adds cost to energy or
trades privacy against uncertainty. Unlike uncertainty families are
incomparable and their attempted composition produces `unknown`.

## Evidence and controls

Run:

```sh
cargo test --locked -p nmlt-grades --all-targets
python3 tools/check_graded_evidence.py
```

The evidence harness compiles the backend independently twice, checks the two
executables are byte-identical, runs the reference twice, validates the
specialized schema, and recomputes:

- the reference and three control source IDs;
- the implementation source-set ID;
- the extension-local claim ID;
- the executable SHA-256 digest;
- the Lean grade entry/root/build-toolchain source set, checker, pinned
  toolchain, and exact theorem handles;
- the canonical evidence manifest ID.

Its self-tests corrupt source and Lean bindings, forge `proved` and verified
Rust-extraction results, remove a theorem handle, and remove a control. All
must be rejected. Source-level controls additionally require a privacy-budget
violation, unknown unbounded repetition, and invalid ppm rejection. The law
checker must find a commutativity counterexample for word concatenation while
finding no counterexample in four finite product samples.

Passing finite samples is still only regression evidence. Universal laws for
each fixed-family mathematical slice come from the separately checked Lean
theorems, while the analyzer and Rust-correspondence claims remain outside
their scope. The evidence preserves those distinctions.

## Research synthesis

The local archive search on 2026-07-18 surfaced no relevant graded-resource
work. Live collector calls were also partially rate-limited, so current
primary sources were consulted directly. The strongest design implications
were:

- [Granule](https://www.cs.kent.ac.uk/people/staff/dao7/publ/granule-icfp19.pdf)
  and [Graded Modal Dependent Type Theory](https://arxiv.org/abs/2010.13163)
  support parameterizing quantitative reasoning by explicit algebraic grade
  structures rather than baking in one resource.
- [Effect quantales](https://arxiv.org/abs/1705.02264) show why a language must
  declare whether sequential composition is order-sensitive. RFC 0012 selects
  a commutative profile only for its four upper bounds.
- [DFuzz](https://cs-people.bu.edu/gaboardi/publication/GaboardiEtAll13popl.pdf)
  shows that a real differential-privacy guarantee requires a sensitivity and
  probabilistic typing story. A numeric field is not a substitute.
- Privacy [sequential composition](https://arxiv.org/abs/1311.0776) and
  [parallel overlap analysis](https://arxiv.org/abs/2109.09078) motivate the
  conservative choice to add privacy without checked disjointness.
- [Automatic amortized resource analysis](https://arxiv.org/abs/2002.09519)
  and 2026 work on
  [dependent graded effects](https://arxiv.org/abs/2601.14846) identify
  credible paths from annotation checking to inference and value-dependent
  bounds, but neither is implemented here.
- A 2026
  [Agda formalization of graded modal dependent type theory](https://arxiv.org/abs/2603.29716)
  is a useful mechanization benchmark. RFC 0012 now checks its standalone
  product-algebra laws, but stays experimental until the connection to NMLT
  terms and the executable analyzer is machine checked.

## Promotion criteria

This track can inform the main language only after it gains a typed-core
elaboration, mechanized plan-analysis preservation/soundness, verified
correspondence to the executable checker, semantic provenance for each
accepted coordinate, and canonical graded-core identity. Until then, `.nmltg`
remains an independent experiment and cannot strengthen an alpha NMLT
verification result.
