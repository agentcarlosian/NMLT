# NMLT

NMLT is a research project for a behavior-first, evidence-carrying programming
language inspired by TLA+ and contemporary mathematics.

> To truly progress, humanity needs new mathematics, new language, and new
> technique.

## Status

NMLT is **pre-alpha research software**. The repository currently defines the
project charter, candidate behavior rules, RFC process, frozen example corpus,
comparative models, evidence identity rules, and the first lossless lexer. It
does not yet provide semantic verification, model checking, code generation,
or a complete stable language syntax.

The frontend deliberately reports `unknown` when asked for evidence. A parsed
file is not a verified program.

## The three programs

- **New mathematics:** a compositional temporal type theory for behaviors,
  resources, trust, refinement, and evidence.
- **New language:** one semantic artifact that can support specification,
  execution, proof obligations, model-based tests, and runtime monitoring.
- **New technique:** evidence-directed development using semantic challenges,
  structured counterexamples, localized repair, negative controls, and explicit
  unknown or indeterminate results.

## Quick start

Requirements: a current stable Rust toolchain and GNU Make.

```bash
make ci
cargo run -p nmlt-cli -- check examples/technicus/provider_attempt.nmlt
cargo run -p nmlt-cli -- inspect examples/technicus/provider_attempt.nmlt
cargo run -p nmlt-cli -- tokens examples/technicus/provider_attempt.nmlt
cargo run -p nmlt-cli -- evidence examples/technicus/provider_attempt.nmlt
```

The `evidence` command emits a scaffold manifest with result `unknown`; it does
not claim verification.

## Repository map

```text
crates/       Minimal Rust frontend and CLI
comparisons/  Equivalent frozen models in TLA+, Quint, and P
docs/         Manifesto, semantics, architecture, method, and roadmap
rfcs/         Proposed language and evidence decisions
examples/     Design fixtures drawn from the source corpora
benchmarks/   Seeded-defect benchmark definitions
schemas/      Machine-readable evidence and benchmark contracts
tests/        Cross-crate fixtures and future integration tests
```

Start with the canonical [execution plan](Plan.md) and
[manifesto](docs/manifesto.md), then read the
[design principles](docs/design-principles.md), [core calculus](docs/core-calculus.md),
and [architecture](docs/architecture.md). Proposed changes enter through the
[RFC process](rfcs/README.md).

## Initial vertical slice

The first semantic milestone is the durable provider-attempt protocol:

1. represent authorization, dispatch, response binding, evaluation, selection,
   and ambiguity;
2. detect dispatch-before-authorize, blind replay, corrupt response binding,
   and selection of a failing result;
3. return structured counterexample traces;
4. emit an evidence manifest containing scope, bounds, assumptions, negative
   controls, and residual gaps;
5. validate concrete runtime traces against the abstract behavior.

## Contribution and governance

See [CONTRIBUTING.md](CONTRIBUTING.md), [GOVERNANCE.md](GOVERNANCE.md), and
[SECURITY.md](SECURITY.md). NMLT is licensed under the
[Apache License 2.0](LICENSE). Contributions are accepted under the same terms.
