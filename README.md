# NMLT

NMLT is a research project for a behavior-first, evidence-carrying programming
language inspired by TLA+ and contemporary mathematics.

> To truly progress, humanity needs new mathematics, new language, and new
> technique.

## Status

NMLT is **pre-alpha research software**. Phases 0–7 now have executable,
identity-bound prototypes at deliberately narrow scopes: a lossless recovering
frontend; a typed provider core and bounded explicit-state checker; finite
temporal, refinement, and runtime-journal checking; a finite Boolean VC with
two checked routes; an authority-bounded deterministic repair baseline; and
one independent graded-resource experiment.

These are separate assurance subjects, not a complete language pipeline. NMLT
does not yet provide full surface-language resolution and execution, verified
source-to-IR elaboration, general or infinite-state verification, production
runtime attestation, general AI-repair evidence, signed evidence, or code
generation.

The structural `evidence` command deliberately reports `unknown`. Semantic
claims require `typecheck` or `model-check`; bounded exploration is reported as
`model_checked`, never as proof. A parsed file is not a verified program.

## The three programs

- **New mathematics:** a compositional temporal type theory for behaviors,
  resources, trust, refinement, and evidence.
- **New language:** one semantic artifact that can support specification,
  execution, proof obligations, model-based tests, and runtime monitoring.
- **New technique:** evidence-directed development using semantic challenges,
  structured counterexamples, localized repair, negative controls, and explicit
  unknown or indeterminate results.

## Quick start

`make ci` requires Linux x86_64 for byte-identical persisted executable
evidence, the Rust toolchain pinned by `rust-toolchain.toml`, GNU Make,
Bash/coreutils, Python 3.11+, and Node/npm with registry access (or a populated npm
cache) for pinned Quint 0.32.0. The separate metatheory gate requires Elan and
the pinned Lean 4.30.0 toolchain. TLC is run when `TLA2TOOLS_JAR` is set; P/.NET
is optional and the current corrected P model remains explicitly unvalidated
when it is absent.

```bash
make ci
cargo run -p nmlt-cli -- check examples/technicus/provider_attempt.nmlt
cargo run -p nmlt-cli -- inspect examples/technicus/provider_attempt.nmlt
cargo run -p nmlt-cli -- tokens examples/technicus/provider_attempt.nmlt
cargo run -p nmlt-cli -- evidence examples/technicus/provider_attempt.nmlt
cargo run -p nmlt-cli -- typecheck benchmarks/seeded-defects/provider-attempt/reference.nmlt
cargo run -p nmlt-cli -- model-check --json benchmarks/seeded-defects/provider-attempt/reference.nmlt
```

The `evidence` command emits a structural scaffold with result `unknown`; it
does not claim verification. Persisted, source-bound results live under
`benchmarks/results/` and `benchmarks/grades/`. The corresponding reproduction
targets are `model-reports`, `temporal-evidence`, `multi-engine-evidence`,
`agentic-evidence`, and `graded-evidence`.

## Repository map

```text
crates/       Frontend, provider engine, temporal/VC checkers, repair and grade prototypes, CLI
comparisons/  Comparable frozen provider models in TLA+, Quint, and P
docs/         Manifesto, semantics, architecture, method, and roadmap
rfcs/         Proposed language and evidence decisions
examples/     Design fixtures drawn from the source corpora
benchmarks/   Seeded-defect benchmark definitions
mechanization/ Pinned Lean metatheory and semantic counterexamples
schemas/      Machine-readable evidence and benchmark contracts
tests/        Cross-crate fixtures and future integration tests
```

Start with the canonical [execution plan](Plan.md) and
[manifesto](docs/manifesto.md), then read the
[design principles](docs/design-principles.md), [core calculus](docs/core-calculus.md),
and [architecture](docs/architecture.md). Proposed changes enter through the
[RFC process](rfcs/README.md).

## Implemented research slices

The durable provider-attempt protocol anchors the current slices:

1. the v2 bounded engine accepts the reference within frozen bounds and
   refutes dispatch-before-authorize, corrected state-local blind replay,
   corrupt response binding, and selection of a failing result;
2. finite temporal evidence replays a canonical lasso/fairness fixture, one
   forward-simulation refinement, accepted/rejected synthetic journals, and
   provider `NoBlindReplay` over a manually projected nine-state observation
   graph with an identity-stutter mutant lasso;
3. two finite VC routes agree on the manual two-observable dispatch claim and
   fail closed on disagreement or assurance laundering;
4. a deterministic three-task repair baseline enforces edit authority and
   links a synthetic drift event into one artifact graph;
5. a separate annotated-plan experiment checks one product grade over cost,
   privacy, energy, and uncertainty, with a kernel-checked mathematical algebra
   but no annotation-soundness or Rust/Lean compiler-correspondence claim.

See [Plan.md](Plan.md) for the exact gates, evidence boundaries, and residual
gaps. Passing any one slice does not verify arbitrary NMLT source.

## Contribution and governance

See [CONTRIBUTING.md](CONTRIBUTING.md), [GOVERNANCE.md](GOVERNANCE.md), and
[SECURITY.md](SECURITY.md). NMLT is licensed under the
[Apache License 2.0](LICENSE). Contributions are accepted under the same terms.
