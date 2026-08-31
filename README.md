# NMLT

![NMLT — New Mathematics, Languages, and Techniques](.github/assets/nmlt-social-preview.jpg)

[![CI](https://github.com/agentcarlosian/NMLT/actions/workflows/ci.yml/badge.svg)](https://github.com/agentcarlosian/NMLT/actions/workflows/ci.yml)

**NMLT — New Mathematics, Languages, and Techniques — is a research repository
for trustworthy computation.** It investigates candidate mathematical
foundations, develops formal languages, and tests evidence-directed techniques.
Its first flagship language is the **NMLT language**, a behavior-first,
evidence-carrying programming language inspired by TLA+ and contemporary
mathematics.

> To truly progress, humanity needs new mathematics, new languages, and new
> techniques.

NMLT is pre-alpha research software. It is not intended to authorize
safety-critical, financial, security-critical, or irreversible actions.

## NMLT today

The active work develops the language and its mathematics together. Programs
describe open behaviors, observations, typed boundary actions, affine authority,
resource grades, and the assumptions and guarantees that make composition
valid.

Rust implements the lossless frontend, typed elaboration pipeline, canonical
artifact producer, and a reference explorer. Lean defines the current
behavioral semantics, checks the theorem premises, and is the semantic authority
for the current behavioral core.

The [getting-started guide](docs/getting-started.md) walks through the checked
sender/receiver program, its refinement, canonical artifact, Lean validation,
and non-authoritative exploration.

## From source to semantics

```text
exact .nmlt bytes
  → lossless syntax, resolution, and typed elaboration       Rust
  → deterministic behavior-core-v1 artifact                 Rust
  → finite behavior construction and premise checking       Lean
  → conditional composition/refinement witnesses            Lean
  → bounded operational inspection, assurance: none         Rust
```

| Area | Current status |
|---|---|
| Language | Finite `Bool`, `Unit`, and enum state; observations; typed input/output ports; affine capabilities; named natural-number grades; nominal rely/guarantee facts; binary wiring; explicit state maps |
| Static semantics | A resource-bearing `Behavior`, binary product steps, resource-aware weak refinement, and a conditional lifting theorem in Lean |
| Dynamic authority | An additional Lean authority-world layer proves unique ownership changes and conditional one-step lifting; it is not yet integrated into the single `Behavior` object or a reachability theorem |
| Artifact | Canonical `behavior-core-v1` JSON with a source digest, typed terms, action profiles, wiring, and refinement data |
| Exploration | `nmlt-eval` explores finite artifacts for language design and debugging, always with `assurance: none` |

This milestone is finite, binary, and safety-oriented. The source digest
identifies the source bytes presented to Lean; the repository separately
reproduces and byte-compares the primary artifact. Detailed semantic boundaries
are recorded with the active definitions in the
[current calculus](docs/core-calculus.md).

## Trust boundary

| Component | Role | Claim ceiling |
|---|---|---|
| Rust frontend and compiler crates | Parse, resolve, type, and emit inspectable core data | Implementation acceptance and reproducibility; no compiler-correctness theorem |
| `nmlt-kernel` | Replay the retained ordinary typed-elaboration certificate | Formation/type acceptance only; never behavioral proof |
| `NMLT.Behavior.ResourceBehavior` | Define the current behavior and static product/refinement theorem | The checked Lean statements under their explicit premises |
| `NMLT.Behavior.ResourceWorld` | Model dynamic nominal authority and one-step product simulation | Ownership uniqueness, explained effects, and conditional one-step lifting; no reachability or liveness |
| Lean artifact modules | Decode finite artifacts, construct behaviors, and decide theorem premises | Acceptance of the decoded artifact semantics; no verified source translation |
| `nmlt-eval` | Reference operational exploration | No proof or verification claim |

The precise active inventory is in
[`security/trusted-components.toml`](security/trusted-components.toml), with
attacker stories in [`docs/threat-model.md`](docs/threat-model.md) and theorem
dependencies in [`mechanization/lean/AXIOMS.md`](mechanization/lean/AXIOMS.md).

## Research map

- [Project status and roadmap](docs/roadmap.md)
- [Architecture](docs/architecture.md)
- [Getting started](docs/getting-started.md)
- [Language sketch](docs/language-sketch.md)
- [Current calculus](docs/core-calculus.md)
- [Manifesto](docs/manifesto.md)
- [Design principles](docs/design-principles.md)
- [Project history](docs/history.md)
- [Citation metadata](CITATION.cff)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)

NMLT is licensed under Apache-2.0. See [`LICENSE`](LICENSE).
