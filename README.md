# NMLT

**NMLT — New Mathematics, Languages, and Techniques — is a research
repository for trustworthy computation.** It investigates candidate mathematical
foundations, develops formal languages, and tests evidence-directed
techniques. Its first flagship language is the **NMLT language**, a
behavior-first, evidence-carrying programming language inspired by TLA+ and
contemporary mathematics.

> To truly progress, humanity needs new mathematics, new languages, and new
> techniques.

**[Watch the ~2.5 minute demo](https://www.youtube.com/watch?v=-PbhZ9me46Y)** —
built for OpenAI Build Week with GPT-5.6 (Sol) running inside the Codex CLI.

## Five-minute judge path

No Lean, TLC, Quint, or P required for this — just the Rust toolchain.

```bash
# Accepted reference model: every property model-checked and holding
cargo run -p nmlt-cli -- model-check --json benchmarks/seeded-defects/provider-attempt/reference.nmlt
```

```json
{
  "system": "ProviderAttemptReference",
  "result": "model_checked",
  "explored_states": 9,
  "properties": [
    { "property": "DispatchRequiresArm", "result": "model_checked", "reason": "holds on every reachable state and transition" }
  ]
}
```

```bash
# Same protocol, one seeded defect: dispatch is reachable before authorization
cargo run -p nmlt-cli -- model-check --json benchmarks/seeded-defects/provider-attempt/dispatch-before-authorize.nmlt
```

```json
{
  "system": "DispatchBeforeAuthorizeMutant",
  "result": "refuted",
  "properties": [
    {
      "property": "DispatchRequiresArm",
      "result": "refuted",
      "reason": "property `DispatchRequiresArm` is false in reachable state 1",
      "witness": { "steps": [ { "action": "dispatch", "state": { "armed": false, "dispatched": true } } ] }
    }
  ]
}
```

The second run is refuted with a structured counterexample instead of a pass —
the same mechanism that accepts the reference model fails closed the moment a
property actually breaks. Full commands and abbreviated output are also in
[the Build Week guide](docs/openai-build-week.md).

## OpenAI Build Week

NMLT's multi-language verification chain — Rust, Lean 4, pinned Charon/Aeneas
translation, evidence manifests, and comparison models in TLA+/Quint/P — was
built during OpenAI Build Week working with **Sol**, GPT-5.6 running inside
the Codex CLI. Sol accelerated construction and recovery across languages;
architectural boundaries, trust assumptions, and the verification strategy
remained human-directed. Every claim NMLT reports still has to clear an
independent kernel bound to exact source identities — nothing here is trusted
because an agent said so. See [the full story](docs/openai-build-week.md).

## NMLT Today

NMLT already runs as an end-to-end laboratory for trustworthy computation. You
can write `.nmlt` models, preserve and inspect their exact syntax, type-check a
supported executable core, explore bounded state spaces, receive structured
counterexamples, validate runtime traces, and reproduce source-bound evidence.

The repository currently includes:

- a lossless parser, recovering syntax tree, formatter, resolver, typed HIR,
  explicit core, and command-line interface;
- deterministic finite-state model checking with reproducible witnesses;
- temporal, fairness, stuttering, hiding, refinement, and runtime-journal
  experiments;
- evidence artifacts bound to exact sources, tools, limits, and certificates;
- independent checking paths that reject stale, forged, or mismatched results;
- open-system composition with assumptions, guarantees, synchronous wiring,
  affine capabilities, resources, grades, and invariant transport;
- a bounded dependency-free Rust validation kernel translated to Lean with
  pinned Charon/Aeneas, plus shared positive and adversarial controls; and
- comparison models in TLA+, Quint, and P.

Start with the [provider-attempt model](examples/technicus/provider_attempt.nmlt),
run the [quick-start commands](#quick-start), or browse the
[execution plan](Plan.md) for the research program.

> **Research status:** NMLT is pre-alpha and intentionally honest about proof
> boundaries. The implemented slices are real and reproducible, but they do
> not yet constitute a general-purpose verified programming language.

<details>
<summary><strong>Current assurance boundary and active work</strong></summary>

M8 independently reproduced the complete bounded gate from a fresh clone. M9
connected supported source through resolved HIR and explicit typed core to an
independently replayed `CheckedProgram`. M10 added bounded behavior-indexed
mathematics and certificates. M11 adds finite open composition and
contract-sound, label-aware two-sided refinement.

The current M11-001c kernel checks exact wiring reflection, product contracts,
invariant transport, affine authority, resource transfer, grades, and
rely/guarantee discharge. Pinned Charon/Aeneas translate its executed Rust path
to Lean. Numeric certificates carry canonical atom dictionaries and undergo
field-by-field readback. The kernel also binds a reconstructed expected
certificate to the executed certificate through structural equality. There are
19 shared Rust/Lean correspondence controls.

Active work is proving, layer by layer, that Aeneas-generated structural
equality implies native Lean equality. The `Grade` layer is complete and
axiom-audited. Rich source-to-certificate encoding, expected-certificate
construction, and Rust readback remain outside verified extraction. Fairness
transport, general source correspondence, infinite-state verification,
production attestation, signatures, and code generation remain future work.

The structural `evidence` command therefore reports `unknown`. Semantic claims
require `typecheck` or `model-check`; bounded exploration is reported as
`model_checked`, never as an unbounded proof. A parsed file is not a verified
program. See the [reproduction record](docs/reproduction-2026-07-18.md) and
[current handoff](docs/reboot-handoff-2026-07-20.md) for exact scope.

</details>

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
N–M–L–T.

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
gaps.

## Contribution and governance

See [CONTRIBUTING.md](CONTRIBUTING.md), [GOVERNANCE.md](GOVERNANCE.md), and
[SECURITY.md](SECURITY.md). NMLT is licensed under the
[Apache License 2.0](LICENSE). Contributions are accepted under the same terms.
