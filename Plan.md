# NMLT Execution Plan

- Status: Active
- Current phase: Phase 1 — syntax and semantic skeleton
- Planning baseline: `6bd302b` (`chore: establish NMLT research scaffold`)
- Updated: 2026-07-18

## 1. Objective

Build NMLT into a behavior-first, evidence-carrying programming language in
which specifications, executable systems, proof obligations, tests, runtime
monitors, and refinement claims share one semantic foundation.

The project has three inseparable outputs:

1. **New mathematics:** a compositional temporal type theory for behavior,
   resources, authority, refinement, and evidence.
2. **New language:** an executable language that makes those concepts usable.
3. **New technique:** evidence-directed development driven by semantic
   challenges, structured witnesses, localized repair, and runtime conformance.

The thesis and boundaries are defined in
[the manifesto](docs/manifesto.md) and
[the design principles](docs/design-principles.md).

## 2. Current baseline

The repository currently contains:

- the research charter, architecture, proposed core calculus, language sketch,
  evidence model, and RFC process;
- examples derived from the Hyperbook, agent trust, and Technicusverus;
- a five-case provider-attempt benchmark containing one reference model and
  four independent semantic mutants;
- JSON Schemas for evidence and benchmark manifests;
- a dependency-free Rust workspace with `nmlt-core` and `nmlt-cli`;
- an RFC-defined lossless lexer plus structural recognition of balanced
  `system Name { ... }` declarations;
- `check`, `inspect`, and `evidence` CLI commands;
- explicitly `unknown` structural evidence, with no false verification claim;
- formatting, Clippy, tests, example checks, and GitHub CI.

What does **not** exist yet:

- a complete parser grammar or lossless syntax tree beyond the token stream;
- name resolution or module semantics;
- type checking;
- action, behavior, temporal, composition, or refinement semantics;
- model checking, proof checking, symbolic verification, or trace monitoring;
- cryptographic artifact binding;
- code generation;
- a stable release.

Structural acceptance must continue to be described as structural acceptance,
not verification.

## 3. Success definition

The first research alpha succeeds when NMLT can:

1. parse and type-check the provider-attempt example without syntax-specific
   escape hatches;
2. give executable meaning to initialization, state, actions, guards, updates,
   nondeterminism, observations, and stuttering;
3. express the four initial safety obligations;
4. accept the reference protocol and refute all four seeded mutants;
5. return deterministic, structured counterexample traces;
6. record finite bounds and assumptions in a schema-valid evidence manifest;
7. reject stale or mismatched source/evidence bindings;
8. validate persisted concrete traces against the abstract behavior;
9. preserve `unknown` and `indeterminate` without promoting either to success;
10. reproduce the complete result from a clean checkout using one documented
    command.

The project does not claim a general-purpose verified programming language at
that point. It claims one validated vertical slice.

## 4. Non-goals for the first alpha

- Full homotopy or cubical type theory.
- Continuous dynamics or differential equation reasoning.
- Probabilistic model checking.
- General-purpose optimizing native-code generation.
- Unrestricted recursion or effects.
- Automatic natural-language-to-verified-program translation.
- A custom SMT solver or interactive theorem prover.
- A universal `verified` badge that erases method and scope distinctions.
- Production authorization of safety-critical or irreversible effects.

These exclusions protect the kernel from becoming a collection of
unmechanized research axioms.

## 5. Program structure

Work is divided into seven coordinated streams.

### W1 — Mathematics and metatheory

Own the formal objects that make NMLT distinct.

Deliverables:

- value, state, action, behavior, property, grade, and evidence judgments;
- static and operational semantics;
- finite and infinite trace semantics;
- observation, hiding, stuttering, and refinement;
- resource and authority algebra interfaces;
- open-system composition conditions;
- mechanized preservation, productivity, refinement, and evidence soundness
  results for the accepted kernel.

Primary design entrypoint: [core calculus](docs/core-calculus.md).

### W2 — Language frontend

Turn the calculus into an understandable language.

Deliverables:

- lexer and lossless concrete syntax tree;
- parser recovery and stable source spans;
- module and import system;
- name resolution;
- typed high-level representation;
- formatter;
- actionable diagnostics;
- language-server protocol support after syntax stabilizes.

Primary design entrypoint: [language sketch](docs/language-sketch.md).

### W3 — Execution and verification

Give behavioral programs executable and checkable interpretations.

Deliverables:

- deterministic interpreter for the executable core;
- transition-system intermediate representation;
- explicit-state exploration;
- invariant evaluation;
- structured and reproducible counterexample traces;
- trace minimization without meaning loss;
- backend protocol for SMT and proof tools;
- proof or result checking at the narrowest available trust boundary.

### W4 — Evidence and assurance

Make the boundary of every claim machine-readable.

Deliverables:

- canonical evidence serialization;
- source, configuration, engine, and result digests;
- explicit method, scope, bounds, assumptions, trusted components, negative
  controls, witnesses, and residual gaps;
- promotion vectors with required dimensions and vetoes;
- conflict detection and fail-closed readback;
- later signature and transparency-log support.

Primary design entrypoint: [evidence model](docs/evidence-model.md).

### W5 — Refinement and runtime connection

Prevent the model from becoming a disconnected artifact.

Deliverables:

- observation and trace-mapping declarations;
- implementation adapters for selected runtimes;
- model-based test generation;
- runtime safety monitors;
- finite-prefix handling for bounded liveness observations;
- typed drift detection;
- conformance reports that preserve unknown and unobserved cells.

### W6 — Agentic formalization and repair

Use AI as a search mechanism without moving it into the trusted kernel.

Deliverables:

- progressive formalization stages;
- compiler-guided, subgoal-local repair;
- specification-strength evaluation through semantic mutants;
- edit-authority rules separating trusted intent/specification from generated
  implementation;
- exact structured feedback rather than prose-only summaries;
- benchmarks that separate syntax, typing, bounded semantics, proof, and human
  intent agreement.

### W7 — Research operations and product quality

Keep results reproducible and the project usable.

Deliverables:

- source and license provenance;
- frozen benchmark manifests;
- clean-room reproduction commands;
- CI and release automation;
- documentation, tutorials, and visualization;
- accessibility and diagnostic-quality review;
- public-claim review before publication.

## 6. Dependency order

```text
intent and examples
        ↓
core calculus and RFCs
        ↓
lossless syntax → resolution → typed behavioral core
                                  ↓
                  transition IR + verification conditions
                       ↙          ↓           ↘
                 execution   model checking   proof backends
                       ↘          ↓           ↙
                     structured evidence and witnesses
                                  ↓
                 implementation traces and runtime monitors
                                  ↓
                    agentic search and localized repair
```

Agentic automation depends on trustworthy feedback. It must not lead the
implementation of semantics or evidence classification.

## 7. Phase plan

### Phase 0 — Foundation and research contract

Goal: make the thesis falsifiable and establish the rules under which the
language will evolve.

Completed:

- [x] Create the standalone NMLT repository.
- [x] Establish manifesto, design principles, architecture, roadmap, and
  research method.
- [x] Establish RFC and decision-record processes.
- [x] Define draft evidence and benchmark schemas.
- [x] Create the provider-attempt reference and four semantic mutants.
- [x] Create a buildable Rust workspace and honest structural CLI.
- [x] Add local and hosted CI gates.

Phase 0 closeout (completed 2026-07-18):

- [x] Decide the project license before accepting external contributions.
- [x] Freeze ten canonical language examples and their intended claims.
- [x] Complete comparative encodings for TLA+, Quint, and P.
- [x] Convert RFC 0001 behavior types from an idea into candidate formal rules.
- [x] Specify the trusted-computing-base threat model.
- [x] Define canonical source and evidence identity requirements.

Exit gate:

- ten examples have intent capsules and negative controls;
- the v1 negative space is explicit;
- behavior types have at least one mechanizable candidate formulation;
- evidence cannot be mistaken for a stronger result class;
- licensing and contribution terms are explicit.

### Phase 1 — Syntax and semantic skeleton

Goal: replace structural scanning with a real, lossless frontend while keeping
the language intentionally small.

Deliverables:

- lexical grammar and tokens;
- rowan-style or equivalent lossless syntax representation selected by ADR;
- parser recovery and diagnostic snapshots;
- modules, declarations, algebraic data, records, and total pure functions;
- systems, state declarations, action blocks, guards, and explicit updates;
- desugaring from surface syntax into a documented untyped core;
- formatter idempotence and round-trip tests.

Required negative controls:

- unclosed and mismatched delimiters;
- duplicate declarations;
- invalid update targets;
- undeclared state;
- hidden implicit state modification;
- ambiguous or recovery-dependent parses.

Exit gate:

- all canonical examples round-trip through parse and format;
- every parser error retains a stable source span;
- malformed fixtures fail deterministically;
- parsing still makes no semantic assurance claim.

### Phase 2 — Typed behavioral core

Goal: implement the smallest useful version of the new mathematics.

Deliverables:

- resolved names and modules;
- typed values and state spaces;
- action input/output and update typing;
- explicit frame conditions;
- affine or linear capabilities for the provider-effect slice;
- behavior-indexed safety propositions;
- observation declarations;
- typed elaboration into the behavioral core;
- initial mechanization of typing and step semantics.

Required theorems or mechanically checked properties:

- preservation for executable steps;
- a precise characterization of progress and blocked states;
- frame soundness for action updates;
- no duplication of linear provider capability;
- property/system indexing prevents cross-system misuse.

Exit gate:

- the provider-attempt reference type-checks;
- type-level versions of the four mutants fail where appropriate;
- remaining semantic mutants survive typing and proceed to model checking;
- no unproved axiom is hidden in the implementation.

### Phase 3 — Behavior execution and explicit-state checking

Goal: validate the first complete semantic slice.

Deliverables:

- initialization and next-state evaluation;
- finite model configuration;
- deterministic state canonicalization;
- reachable-state exploration;
- safety invariant checking;
- structured trace schema and renderer;
- exact benchmark result manifests;
- independent seeded-defect readback.

Exit gate:

- the reference model passes within frozen bounds;
- all four semantic mutants are refuted by the intended property;
- every refutation includes a deterministic structured trace;
- bounds, assumptions, and engine identity are recorded;
- rerunning from a clean checkout reproduces the same classifications.

### Phase 4 — Temporal properties and refinement

Goal: restore the full behavioral ideas that distinguish TLA+-inspired work
from ordinary contract checking.

Deliverables:

- infinite or lasso trace representation;
- eventuality and leads-to;
- weak and strong fairness;
- stuttering invariance;
- hiding and observation mapping;
- refinement declarations and checking;
- runtime journal-to-model trace adapter.

Exit gate:

- blind replay is checked as a temporal property, not only a state invariant;
- at least one refinement mapping hides internal implementation state;
- concrete persisted traces are accepted or rejected with localized reasons;
- fairness assumptions are visible in evidence.

### Phase 5 — Multiple verification engines

Goal: route claims to the cheapest soundly applicable oracle without changing
their meaning.

Deliverables:

- verification-condition IR;
- SMT backend protocol for finite symbolic claims;
- proof-assistant export and checked return path;
- property-based and model-based test integration;
- backend result normalization;
- evidence composition rules.

Exit gate:

- at least one property is checked through two independent engines;
- disagreements fail closed and preserve both raw results;
- bounded evidence cannot be serialized as `proved`;
- backend versions and trusted components are exact.

### Phase 6 — Runtime and agentic workflow

Goal: demonstrate the complete new technique from intent through deployment
evidence.

Deliverables:

- formal-query or intent-capsule review;
- semantic mutation generation;
- compiler-guided localized repair;
- runtime monitors and drift events;
- authority controls preventing automatic weakening of trusted claims;
- evaluation against held-out formalization tasks.

Exit gate:

- agent assistance measurably improves semantic completion;
- no benchmark pass depends on silently modifying the trusted property;
- runtime drift is classified and linked to its model and implementation;
- human reviewers can inspect intent, evidence, witness, and residual gaps from
  one artifact graph.

### Phase 7 — Independent research extensions

Possible tracks:

- guarded cubical equality and proof-relevant equivalence;
- hybrid dynamics and differential invariants;
- probabilistic temporal properties;
- richer graded modalities for privacy, cost, energy, and uncertainty;
- categorical libraries for reusable open-system composition.

Each track requires its own RFC, metatheory, implementation, backend, negative
controls, and benchmark. None blocks the first alpha.

## 8. Immediate execution backlog

Work should proceed in this order.

### P0 — Decisions that unblock implementation

- [x] `NMLT-P0-001`: Decide and record license.
- [x] `NMLT-P0-002`: Write the lexical grammar RFC.
- [x] `NMLT-P0-003`: Select the lossless syntax-tree representation by ADR.
- [ ] `NMLT-P0-004`: Define canonical diagnostics and snapshot format.
- [x] `NMLT-P0-005`: Define canonical source identity and hashing requirements.
- [x] `NMLT-P0-006`: Create the trusted-computing-base threat model.

### P1 — Real frontend

- [x] `NMLT-P1-001`: Replace the structural scanner with a token stream.
- [ ] `NMLT-P1-002`: Parse modules, data declarations, systems, and state.
- [ ] `NMLT-P1-003`: Parse action blocks and explicit updates.
- [ ] `NMLT-P1-004`: Preserve comments and whitespace in a lossless tree.
- [ ] `NMLT-P1-005`: Implement parser recovery without accepting ambiguous
  semantics.
- [ ] `NMLT-P1-006`: Add formatter round-trip and idempotence tests.

### P1 — Mathematical core

- [x] `NMLT-P1-101`: Formalize candidate behavior-type formation rules.
- [ ] `NMLT-P1-102`: Formalize state and action typing judgments.
- [ ] `NMLT-P1-103`: Select the v1 capability discipline.
- [ ] `NMLT-P1-104`: Define observation and stuttering semantics.
- [ ] `NMLT-P1-105`: Choose a mechanization environment and repository layout.
- [ ] `NMLT-P1-106`: Prove or refute refinement congruence for the candidate
  composition rule.

### P1 — Benchmark integrity

- [ ] `NMLT-P1-201`: Add intent capsules for all five provider cases.
- [ ] `NMLT-P1-202`: Freeze property identities and expected witnesses.
- [ ] `NMLT-P1-203`: Record exact source-corpus provenance without copying
  protected material.
- [ ] `NMLT-P1-204`: Add malformed, vacuous-property, and weakened-invariant
  controls.
- [ ] `NMLT-P1-205`: Add schema validation to `make ci` without depending on a
  globally installed Python package.

## 9. Canonical example ladder

Examples should be added in increasing semantic difficulty.

| Level | Example | Concepts |
|---|---|---|
| 0 | Boolean toggle | State, action, invariant |
| 1 | One-bit clock | Infinite behavior, fairness, observation |
| 2 | Euclid | Pure functions, algorithm, termination |
| 3 | Provider attempt | Capability, external effect, ambiguity, evidence |
| 4 | Mutual exclusion | Concurrency, inductive invariant, liveness |
| 5 | Bounded channel/buffer | Refinement, hiding, stuttering |
| 6 | Agent trust chain | Authority provenance, composition, information flow |
| 7 | Durable controller trace | Concrete refinement and runtime conformance |
| 8 | Probabilistic or hybrid pilot | Research extension, not alpha scope |

Each level must include a reference, at least one meaningful semantic mutant,
an intended evidence class, and a plain-language counterexample scenario.

## 10. Verification and CI gates

The default local gate remains:

```bash
make ci
```

Before merging a change, require as applicable:

- formatting and compiler checks;
- Clippy with warnings denied;
- unit, integration, round-trip, and negative-control tests;
- schema validation;
- internal documentation-link validation;
- example and benchmark manifest validation;
- deterministic evidence readback;
- an RFC or decision record for semantic and trusted-core changes;
- updated assumptions and residual gaps.

No test count is itself a quality claim. A checker must demonstrate that it can
observe the defect it claims to prevent.

## 11. Metrics

Track metrics by claim class and benchmark rather than as one project score.

### Semantic effectiveness

- seeded defects detected;
- non-equivalent mutants rejected;
- counterexample length before and after minimization;
- invariant and refinement failures localized;
- false acceptance and false rejection rates.

### Evidence integrity

- manifests with complete source, engine, scope, and assumption binding;
- stale or conflicting evidence rejected;
- required negative controls observed;
- unknown assurance cells;
- independent readback success.

### Language usability

- time to first meaningful model;
- diagnostic location and repair accuracy;
- formatter and parser round-trip stability;
- abstraction and refinement-mapping burden versus comparison languages;
- human ability to explain a returned witness.

### Agentic assistance

- syntax, type, and semantic success reported separately;
- repair success per structured feedback round;
- whole-program regeneration avoided;
- specification weakening attempts blocked;
- human intent agreement on held-out tasks.

## 12. Risk register

| Risk | Trigger | Response |
|---|---|---|
| Mathematical kitchen sink | A feature requires unproved axioms merely to enter v1 | Move it to a gated extension |
| Undecidable or opaque typing | Routine programs require general proof search during type checking | Restrict the stable static fragment and expose explicit obligations |
| State explosion | Canonical bounded examples exceed practical exploration budgets | Improve symmetry, symbolic backends, abstraction, and declared bounds without weakening claims |
| Intent/specification gap | Strong verification passes a vacuous or wrong property | Require intent capsules, examples, mutants, and human review |
| Backend trust confusion | One engine's result is presented as stronger than justified | Enforce typed evidence and fail-closed normalization |
| Model/implementation drift | Production traces no longer map to the accepted behavior | Emit typed drift events and block affected promotion claims |
| AI reward hacking | An agent weakens a property or benchmark to pass | Separate edit authority and bind trusted artifacts before repair |
| Research prototype sprawl | Many empty crates or unfinished feature branches appear | Add components only at implemented, reviewed boundaries |
| Unclear licensing | External work begins without explicit terms | Complete license decision during Phase 0 |
| Novelty overclaim | A combination of known ideas is described as established new math | Label hypotheses and require mechanized and comparative evidence |

## 13. Release plan

### `0.0.x` — Research scaffold

Unstable syntax and no semantic verification. Structural tooling and design
artifacts only.

### `0.1.0` — Provider-attempt research alpha

Requires the Phase 3 exit gate: typed behavioral core, executable finite model,
four detected semantic mutants, structured traces, and bound evidence.

### `0.2.0` — Temporal and refinement alpha

Requires fairness, stuttering, hiding, one checked refinement, and one concrete
trace adapter.

### `0.3.0` — Multi-engine alpha

Requires two independent verification routes and fail-closed disagreement
handling.

### `1.0.0`

Not scheduled. It requires a stable kernel, published compatibility policy,
explicit trusted computing base, independent review, mature tooling, and
evidence across more than the initial protocol domain.

## 14. Stop conditions

Stop and reassess a phase when:

- the formal semantics and implementation disagree;
- a required negative control is not observed;
- evidence cannot be bound to exact inputs and tools;
- a result class would need to be weakened or renamed to claim success;
- unexplained nondeterminism prevents reproduction;
- benchmark expectations change after results are observed without a recorded
  correction and new benchmark version;
- the language's claimed negative space collapses into an existing system with
  only superficial syntax differences.

Stopping under these conditions is research progress: it prevents an invalid
result from becoming architecture.

## 15. Next milestone

The active milestone is **M1: Lossless Frontend Contract**. Phase 1 started on
2026-07-18 with RFC 0003, ADR 0002, and the lossless token stream; the green
tree and declaration parser remain in progress.

M1 is complete when:

1. a lexical grammar RFC is accepted;
2. a lossless syntax-tree ADR is accepted;
3. the three existing examples and five provider benchmark files parse into a
   lossless tree;
4. formatting is idempotent and preserves comments;
5. malformed fixtures return stable diagnostics;
6. the CLI continues to emit only `unknown` evidence because semantic checking
   has not yet run.

M1 should be completed before adding model-checker or AI-generation code.
