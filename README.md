# NMLT

**NMLT—New Mathematics, Languages, and Techniques—is a programming-language
research project developing a new language and its mathematics together.**

NMLT is pre-alpha. It is a research system, not a production verifier, and it
must not authorize safety-critical, financial, security-critical, or
irreversible actions.

## Why NMLT

Most languages make values and functions primary, then add concurrency,
authority, resource use, and proof obligations through separate tools. NMLT
starts from behavior: which states may change, what a component can observe,
which boundary actions it may exchange, what authority an action consumes or
transfers, and which assumptions make composition valid.

The project has two inseparable outputs:

- a human-facing language for describing open, resource-aware systems; and
- mechanized mathematics for explaining when those systems compose and refine.

Rust implements the lossless frontend, typed elaboration pipeline, canonical
artifact producer, and a reference explorer. Lean defines the current
behavioral semantics and checks the theorem premises. Rust is not the semantic
prover.

## A small NMLT program

This excerpt is copied from the checked capability-bearing fixture:

```nmlt
enum ContractFact { Authorized, Ready }

system ConcreteSender {
  state unit: Bool = false
  capability permit: Once<Unit>
  port output send: Once<Unit>

  action output send grade { work: 1 } {
    rely ContractFact.Ready
    guarantee ContractFact.Authorized
    emit permit
    consume permit
  }

  observe unit
}

system Receiver {
  state bit: Bool = false
  port input receive: Once<Unit>

  action input receive(permit: Once<Unit>) grade { work: 2 } {
    require bit == false
    rely ContractFact.Authorized
    guarantee ContractFact.Ready
    set bit = true
  }

  observe bit
}

compose ConcreteNetwork {
  connect ConcreteSender.send -> Receiver.receive
}
```

The complete checked example also includes an explicit refinement:
[`visible_resource_sync.nmlt`](examples/pivot/visible_resource_sync.nmlt).

## Current working slice

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

The source digest identifies the source bytes presented to the Lean checker. The
checker does not re-run the Rust compiler, so it does not prove that an
arbitrary artifact was produced from those bytes. The repository gate separately
reproduces and byte-compares the canonical primary fixture.

### Explicit non-claims

The current repository does **not** establish:

- a verified Rust-to-Lean compiler;
- existence or reachability of the artifact's dynamic transfer step;
- preservation of a general open interface by binary product formation;
- preservation of peer-side hiding, direction, or payload by the current static
  product;
- a dynamic behavior initializer, observation, or static/dynamic correspondence
  theorem;
- an adequacy theorem connecting `ResourceWeakRefinement` to finite or infinite
  trace observations;
- necessity of every product-formation gate for the lifting proof;
- fairness, divergence, infinite traces, or liveness transport;
- general composition, arbitrary grade algebras, or infinite state; or
- proof, model-check, evidence, or runtime authority for the Rust explorer.

## Reproduce the current result

```bash
cargo run -p nmlt-cli -- typecheck \
  examples/pivot/visible_resource_sync.nmlt

cargo run -p nmlt-cli -- elaborate \
  examples/pivot/visible_resource_sync.nmlt \
  --emit-core /tmp/visible-resource-sync.json

cd mechanization/lean
lake exe nmlt-artifact-check /tmp/visible-resource-sync.json \
  ../../examples/pivot/visible_resource_sync.nmlt
cd ../..

cargo run -p nmlt-cli -- explore \
  --behavior ConcreteNetwork --max-states 8 \
  /tmp/visible-resource-sync.json
```

Use `make ci` for the Rust language gate, `make metatheory` for the Lean gate,
or `make reproduce` for the full Rust, Lean, and independent NanoDA gate. These
targets require a POSIX shell, `python3`, GNU core utilities including
`sha256sum`, Rust 1.94, and the pinned Lean toolchain; Windows development uses
WSL. The Lean gate rejects unchecked proof placeholders, tests artifact
mutations, and audits focused theorem dependencies. The full gate additionally
downloads pinned NanoDA/exporter sources and independently checks every project
declaration.

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
- [Language sketch](docs/language-sketch.md)
- [Current calculus](docs/core-calculus.md)
- [Manifesto](docs/manifesto.md)
- [Design principles](docs/design-principles.md)
- [Paper 1 in plain English](docs/paper-1-in-plain-english.md)
- [Paper 1 claim ceiling](docs/paper-1-claim-ceiling.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)

## History

NMLT previously shipped a contest-oriented collection of bounded verifiers and
demonstrations. That work remains reproducible from the immutable
[`build-week-judge-demo-2026`](https://github.com/agentcarlosian/NMLT/tree/build-week-judge-demo-2026)
tag at commit `0417f6e`. It is a historical release, not the active
architecture.

NMLT is licensed under Apache-2.0. See [`LICENSE`](LICENSE).
