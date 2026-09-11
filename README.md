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
artifact producer, reference explorer, and bounded finite interpreter. Lean
defines the current behavioral semantics, checks the theorem premises, and is
the semantic authority for the current behavioral core.

The [getting-started guide](docs/getting-started.md) walks through the checked
sender/receiver program, its refinement, canonical artifact, Lean validation,
and non-authoritative exploration.

## From source to semantics

```text
exact .nmlt bytes
  → lossless syntax, resolution, and typed elaboration       Rust
  → deterministic behavior-core-v1/v2 artifact              Rust
  → finite behavior construction and premise checking       Lean
  → conditional refinement or supplied finite path checks   Lean
  → bounded operational inspection, assurance: none         Rust
```

| Area | Current status |
|---|---|
| Language | Finite `Bool`, `Unit`, and enum state; observations; typed input/output ports; affine capabilities; named natural-number grades; nominal rely/guarantee facts; binary wiring; explicit state maps |
| Static semantics | A resource-bearing `Behavior`, binary product steps, resource-aware weak refinement, and a conditional lifting theorem in Lean |
| Dynamic authority | R1 combines control and authority in an initialized behavior with conditional simulation and finite-path ownership results; v1 artifacts still construct the earlier conditional witnesses |
| Artifact | Default v1 plus opt-in v2 with explicit known capabilities, initial authority, and separately checked finite path witnesses |
| Exploration | `nmlt-eval` explores Bool/Unit/enum artifacts and emits v2 paths; frozen value/resource graphs are compared with Lean, always with `assurance: none` |
| Local execution | R2's first increment adds source-driven finite `run` and exact-executable `replay`; shared v2 steps, structured stop outcomes, no host jobs, `assurance: none` |
| Local source jobs | Opt-in `job_square` uses bounded attempts, a durable journal, validated subprocess results, timeout/output limits, replay without redispatch, and recovery inspection; executable-only |
| Source jobs | Typed worker and pinned Init proof-term jobs; affine transfer through functions/folds; poll/cancel/collect, durable decisions, explicit uncertainty and source/project resumption; executable-only |
| Projects | `init`, `lock`, `run`, `test`, `replay`, `resume`, `fmt`, and `check-project`; complete source/tool identities and structured diagnostics |
| Local Lean tasks | `lean-task bind/prove/recheck`; native import discovery across selected source/vendor roots, fixed targets, saved sources, closed proof terms, actual dependencies and independent NanoDA checking; bounded R3 profile |
| User safety properties | Exact source predicates with Lean-checked initialization/preservation or an initialized finite counterexample; decoded finite scope |
| Pure source workflows | Named entries, structured inputs, records, bounded lists/folds, local modules, acyclic functions, typed outcomes, matching, execution/replay; executable-only, no host jobs |

Start with the [project guide](docs/r2-projects.md) to change real inputs,
handle a failed worker, reuse its output, test the result and replay retained
sources. [Source packages](docs/r2-source-packages.md) share typed components;
[affine handles](docs/r2-job-transfer.md) move job authority through them.
[Lean proof terms](docs/r2-lean-terms.md) accept variable statements/candidates
under the pinned Init and empty-axiom policy. [Recovery](docs/r2-recovery.md)
continues a saved invocation while retaining prior spend and explicit uncertainty.

The [finite safety guide](docs/r2-safety-invariants.md) checks user predicates
against the decoded model. Workflow functions and host effects remain
executable-only; they do not automatically inherit finite model proofs.
R2 is complete at local pre-alpha scope. The
[R2 audit](docs/r2-completion-tracker.md) and
[validation record](docs/reviews/r2-completion-2026-09-09.md) record its full scope
and reproduction evidence. The current [R3 profile](docs/r3-lean-tasks.md)
discovers local source imports, binds Lean targets and independently checks
saved proof artifacts, with bounded file exports up to 16 MiB and readable
graphs of actual declaration dependencies.
R3 remains in progress; larger libraries, editor integration and asynchronous
project-proof jobs are later work.

The behavioral milestone is finite, binary, and safety-oriented. The source digest
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
| `NMLT.Behavior.ResourceDynamics` | Unify control, shared authority, initialization, observation and deferred effects | Scoped binary simulation and finite-path results |
| Lean invariant checker | Checks finite enumeration, exact predicate, initialization/preservation or a reachable violation | Supplied decoded finite binary model; no verified source translation or host correspondence |
| Lean v2 execution checker | Check supplied paths from decoded initial states, including receive then consume/retransfer | Finite binary execution and initial synchronized refinement; no verified compilation or host runtime |
| Lean artifact modules | Decode finite artifacts, construct behaviors, and decide theorem premises | Acceptance of the decoded artifact semantics; no verified source translation |
| `nmlt-eval` | Reference exploration and bounded finite execution | No proof or verification claim |
| `nmlt-runtime` | Bounded job control, journal replay/recovery, and local adapter protocol | Executable-only; no host-correctness theorem, complete host isolation, or exactly-once external execution claim |
| `nmlt-workflow` | Workflow typing, affine source control, lowering, bounded evaluation and replay | Executable-only; no behavioral artifact, Lean proof of workflow semantics, or verified host behavior |

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
