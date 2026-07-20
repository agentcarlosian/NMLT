# NMLT

**NMLT — New Mathematics, Languages, and Techniques — is an umbrella research
program for trustworthy computation.** It investigates candidate mathematical
foundations, develops formal languages, and tests evidence-directed
techniques. Its first flagship language is the **NMLT language**, a
behavior-first, evidence-carrying programming language inspired by TLA+ and
contemporary mathematics.

> To truly progress, humanity needs new mathematics, new languages, and new
> techniques.

Author and maintainer: [Carlosian](AUTHORS.md)
([carlosian@agentmail.to](mailto:carlosian@agentmail.to)).

## Status

NMLT is **pre-alpha research software**. Phases 0–7 now have executable,
identity-bound prototypes at deliberately narrow scopes: a lossless recovering
frontend; a typed provider core and bounded explicit-state checker; finite
temporal, refinement, and runtime-journal checking; a finite Boolean VC with
two checked routes; an authority-bounded deterministic repair baseline; and
one independent graded-resource experiment.

The complete bounded gate was reproduced from a fresh clone of commit
`e3f7ec6ae2d14ade78183ff78d58f7198cb76858`; see the
[reproduction record](docs/reproduction-2026-07-18.md). This closes the M8
reproduction gate, not the remaining research or release-promotion gaps.

These remain separate assurance subjects, not a complete flagship-language
pipeline. M9 now has a completed narrow integration contract, a complete
origin-censused surface projection, a source-fed resolved HIR with replayable
all-reference coverage, a structurally validated explicit typed core, and an
identity-bound bidirectional elaborator that emits a complete derivation DAG.
An independent kernel replays that DAG against exact HIR and core and is the
sole constructor of `CheckedProgram`; the bounded provider engine now consumes
that checked value without a second parser. M10 has closed its bounded
behavior-indexed mathematics and certificate seed. M11 now includes finite
safety-only open composition and contract-sound label-aware refinement:
global input receptiveness, explicit synchronous connections, canonical finite
nominal payload contracts, executable A/G variance checks, an axiom-free
exact-action Lean congruence theorem, and separate axiom-free open-refinement
identity/composition laws. M11-001c additionally has a finite two-sided product
checker, exact wiring-edge reflection, product-contract checking, invariant
transport, and an axiom-free label-mapped Lean product theorem with contract
variance and distinct concrete/abstract port types. The checker now also
enforces affine capability partition/transfer, componentwise grade improvement,
and rely/guarantee discharge, with matching Lean resource rules and a checked
finite-table/typed-map boundary. Lean now bundles operational, contract, and
resource refinement across all eight structural product-action constructors.
Rust revalidates the isolated canonical certificate through a bounded,
dependency-free kernel. Pinned Charon/Aeneas translate the executed kernel to
Lean, where acceptance is proved to imply its refinement and wiring contract.
The numeric certificate carries its canonical atom dictionary and is read back
field-by-field before execution; Lean proves unique decoding and referenced-ID
coverage for that boundary. The kernel now also executes structural equality
between a reconstructed expected certificate and the actual certificate before
validation. The translated Lean theorem exposes that equality decision and the
complete execution contract, with 19 shared Rust/Lean correspondence controls.
A bottom-up proof connecting generated equality to native Lean equality is in
progress; its scalar-only `Grade` layer is complete and axiom-audited. The full
gate remains open because the remaining equality layers, rich
system-to-certificate encoder, expected-certificate reconstruction, and Rust
readback implementations are not verified extraction; fairness and broader
source correspondence also remain open.
NMLT does not
yet provide full surface-language execution, verified source-to-IR
elaboration, general or infinite-state verification, production runtime
attestation, general AI-repair evidence, signed evidence, or code generation.

The structural `evidence` command deliberately reports `unknown`. Semantic
claims require `typecheck` or `model-check`; bounded exploration is reported as
`model_checked`, never as proof. A parsed file is not a verified program.

## The three research tracks

- **New mathematics:** a compositional temporal type theory for behaviors,
  resources, trust, refinement, and evidence.
- **New languages:** the flagship NMLT language plus explicit core, evidence,
  observation, and extension languages that share a semantic foundation rather
  than becoming disconnected notations.
- **New techniques:** evidence-directed development using semantic challenges,
  structured counterexamples, localized repair, negative controls, and
  explicit unknown or indeterminate results.

“NMLT” names the research program and remains the name of its flagship
language. Technical identifiers such as `.nmlt`, crate names, evidence domains,
and Lean namespaces are unchanged. The accepted naming decision is recorded in
[ADR 0003](docs/decisions/0003-project-identity.md).

Latin companion form: ***Nova Mathematica · Linguae · Technicae***. The
separators are deliberate: this is a four-part research title preserving
N–M–L–T, not a claim that the words form one classical prose sentence.

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
make reproduce
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
`agentic-evidence`, `graded-evidence`, `open-composition-evidence`,
`open-refinement-evidence`, and `open-congruence-evidence`.

## Repository map

```text
crates/       Frontend, HIR, typed core/elaborator, verification prototypes, and CLI
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
   but no annotation-soundness or Rust/Lean compiler-correspondence claim;
6. the first M11 open-system seed checks finite input/output/internal
   interfaces, strict noncircular symbolic contract discharge, global
   receptiveness, injective boundary mappings, whole-connection reflection,
   and product construction with state, transition, and work-item limits. Lean
   separately proves structural exact-action congruence plus composability and
   product-receptiveness results;
   the claim-specific evidence explicitly denies Rust/Lean correspondence.
7. M11-001b and the current M11-001c core add nominal finite payload contracts,
   label-aware two-sided refinement, contract variance, exact wiring-edge
   reflection, invariant transport, affine authority, grades, rely/guarantee
   discharge, canonical certificates, and a translated bounded execution
   kernel. Dictionary, numeric-field, rich-source, and kernel-bound
   substitution controls fail closed. The remaining correspondence boundary
   is recorded explicitly rather than promoted to a completed proof.

See [Plan.md](Plan.md) for the exact gates, evidence boundaries, and residual
gaps. The current reboot-safe continuation is in the
[2026-07-20 M11 handoff](docs/reboot-handoff-2026-07-20.md). Passing any one
slice does not verify arbitrary NMLT source.

## Contribution and governance

See [CONTRIBUTING.md](CONTRIBUTING.md), [GOVERNANCE.md](GOVERNANCE.md), and
[SECURITY.md](SECURITY.md). NMLT is licensed under the
[Apache License 2.0](LICENSE). Contributions are accepted under the same terms.
