# NMLT Execution Plan

- Status: Active pre-alpha research prototype
- Implementation status: bounded milestones across Phases 0–7 and the M8
  independent-reproduction gate are complete; broader promotion gates remain
- Active focus: M9 source-to-typed-core integration, followed by deeper
  mathematical and verification work
- Initial planning baseline: `cf4f006` (`chore: establish NMLT research scaffold`)
- Updated: 2026-07-19

## 1. Objective

Develop **NMLT — New Mathematics, Languages, and Techniques** as an umbrella
research program for trustworthy computation. Its first flagship language is
the NMLT language: a behavior-first, evidence-carrying programming language in
which specifications, executable systems, proof obligations, tests, runtime
monitors, and refinement claims share one semantic foundation.

The project has three inseparable outputs:

1. **New mathematics:** candidate compositional temporal type theories for behavior,
   resources, authority, refinement, and evidence.
2. **New languages:** the flagship surface language and explicit core,
   evidence, observation, and extension languages that make those concepts
   usable without hiding their semantic boundaries.
3. **New techniques:** evidence-directed development driven by semantic
   challenges, structured witnesses, localized repair, independent checking,
   and runtime conformance.

The thesis and boundaries are defined in
[the manifesto](docs/manifesto.md) and
[the design principles](docs/design-principles.md).

## 2. Current baseline

The repository currently contains:

- the research charter, architecture, proposed core calculus, language sketch,
  evidence model, and RFC process;
- examples derived from the Hyperbook, agent trust, and Technicusverus;
- a v2 provider-attempt benchmark containing one reference model, four
  independent semantic mutants, three integrity controls, and a one-shot
  replay regression that distinguishes the corrected `NoBlindReplay` claim;
- JSON Schemas and independent readback harnesses for benchmark, model-check,
  temporal, multi-engine, agentic, graded, and general evidence artifacts;
- a Rust workspace with `nmlt-core`, `nmlt-hir`, `nmlt-engine`,
  `nmlt-temporal`, `nmlt-verify`, `nmlt-agent`, `nmlt-grades`, and
  `nmlt-cli`;
- an RFC-defined lossless lexer, immutable CST, recovering declaration parser,
  diagnostic snapshots, and preservation formatter;
- `check`, `inspect`, `tokens`, `typecheck`, `model-check`, and `evidence` CLI
  commands;
- a typed executable provider fragment with explicit frames, affine
  capabilities, deterministic BFS, and structured counterexamples;
- source-bound persisted results that accept the provider reference within
  the frozen 10,000-state/100-step configuration and refute all four seeded
  semantic mutants with structured witnesses;
- finite lasso, fairness, stuttering, hiding, forward-simulation refinement,
  and three-valued runtime-journal checking with independently replayed
  Phase 4 evidence, including a manually projected provider observation graph
  for temporal `NoBlindReplay`;
- a finite Boolean VC boundary with deterministic reachability, independent
  inductiveness enumeration, a checked finite-invariant certificate, and
  raw-preserving fail-closed composition;
- an authority-bounded deterministic repair-protocol baseline over three
  hand-authored held-out tasks, linked to a synthetic runtime-drift artifact
  graph;
- one independent graded-resource experiment over cost, privacy, energy, and
  uncertainty annotations, with checked arithmetic and negative controls;
- explicitly `unknown` structural evidence, with no false verification claim;
- formatting, Clippy, tests, example checks, and GitHub CI.

What does **not** exist yet:

- complete name resolution and executable module semantics for the full
  lossless surface language;
- general action inputs, language-integrated grades, ports, open composition,
  or closed algebraic-constructor resolution in the executable core;
- an infinite-state or general source-language temporal/refinement checker,
  liveness-refinement proof, or authenticated deployed-runtime monitor;
- verified source-to-transition-graph or source-to-VC elaboration, general SMT
  solving, or a native proof-assistant certificate return path;
- a mechanized program-level metatheory for temporal refinement, evidence
  composition, agentic repair, or grade-preserving source/Rust elaboration;
- cryptographic signatures, a transparency log, or journal attestation;
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

Items 1–7 and 9–10 now have local, bounded implementations and checked-in
evidence for the provider slice. Item 10 was reproduced from a fresh clone of
`e3f7ec6ae2d14ade78183ff78d58f7198cb76858`; the exact environment, command,
and outcomes are in
[`docs/reproduction-2026-07-18.md`](docs/reproduction-2026-07-18.md). Item 8 is
exercised only by synthetic persisted journal fixtures, not observations from
a deployed runtime.

The project does not claim a general-purpose verified programming language at
that point. It claims several identity-bound experimental slices whose
assurance subjects and residual gaps remain separate.

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

Completed 2026-07-18. The frontend gate is syntax-only: recovery constructs an
error-bearing tree and never promotes malformed input to semantic acceptance.

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

Completed 2026-07-18 for the provider-effect slice. The Rust/Lean
correspondence is guarded by a shared provider vector; full compiler
correctness remains an explicitly recorded residual gap rather than an axiom.

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

Bounded implementation milestone completed 2026-07-18 for provider-attempt
suite v2. The deterministic BFS
engine exhausted the reference graph within `max_states = 10000` and
`max_depth = 100`, and all four primary mutants were refuted. The v2
correction replaced the off-by-one successor-state `NoBlindReplay` formula
with the state-local rule
`always(phase == indeterminate implies not enabled(dispatch))`. A permanent
one-shot control demonstrates the distinction under the Phase 3 execution
profile: `next` ranges over declared action successors, with an
identity-stutter successor added only at terminal states. Under that explicitly
stutter-sensitive profile, the old formula accepts one replay that the
corrected formula refutes at state zero. This is not a claim about RFC 0007 or
Phase 4 universal identity-stutter closure; under universal stuttering the old
`next` formula is also refuted at the initial state.

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

Gate result: passed for provider suite v2, including exact source, engine
source-set, executable, toolchain, configuration, and result identities. A
fresh clone of `e3f7ec6ae2d14ade78183ff78d58f7198cb76858` reproduced the
reference classification and all four deterministic refutations; see the
[independent reproduction record](docs/reproduction-2026-07-18.md).

### Phase 4 — Temporal properties and refinement

Goal: restore the full behavioral ideas that distinguish TLA+-inspired work
from ordinary contract checking.

Bounded implementation milestone completed 2026-07-18 for canonical finite
graphs. `nmlt-temporal` checks eventuality and leads-to with deterministic
lasso witnesses, explicit weak and strong action fairness, universal identity
stutter, observation projection, action hiding, finite one-step forward
simulation, and three-valued runtime journals. `TemporalChecker::always`
additionally checks the corrected provider
`NoBlindReplay` property over a nine-state property-relevant reference
observation graph and a one-state blind-replay mutant graph. The mutant returns
a zero-transition stem and an infinite identity-stutter lasso at the violating
initial state. Phase 4 evidence binds the exact property, sources, Phase 3
results, graphs, projection, implementation, and replay identities; Python
independently checks projected enabledness and replays the provider lasso.

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

Gate result: the generic finite temporal/refinement/runtime fixture and the
provider-specific temporal clause pass at their exact finite scopes. The
provider source-to-observation-graph construction is manually audited rather
than compiler-derived. This is not a general temporal-language implementation,
infinite-state checking, liveness-refinement proof, verified source-to-graph
translation, or runtime-journal authenticity.

### Phase 5 — Multiple verification engines

Goal: route claims to the cheapest soundly applicable oracle without changing
their meaning.

Completed 2026-07-18 for a hand-constructed finite Boolean safety VC.
`nmlt-verify` checks the provider dispatch property by deterministic reachable
state exploration and by separate finite inductiveness enumeration. The
second route returns a locally checked finite-invariant certificate;
composition retains both raw results and maps disagreement, stale identity,
or bounded-proof laundering to `unknown`.

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

Gate result: passed for the exact two-observable VC. Its `proved` result is
scoped to `finite_vc_only`; the manual source projection omits other provider
state, both engines run in one Rust process, and no verified NMLT-to-VC
compiler is claimed.

### Phase 6 — Runtime and agentic workflow

Goal: demonstrate the new technique from intent through runtime evidence while
keeping repair outside the claim authority.

Completed 2026-07-18 as a deterministic protocol-conformance baseline, not an
LLM evaluation. `nmlt-agent` moved three hand-authored held-out fixtures from
0/3 baseline completion to 3/3 after one structured-feedback repair each,
rejected all 21 protected-artifact modification probes, retained and killed
all three negative controls, and promoted no `unknown` or conflict. The
combined evidence graph links exact intent, property, oracle, candidate,
feedback, proposal, evaluation, finite model, implementation, runtime witness,
drift event, and residual-gap artifacts.

Deliverables:

- formal-query or intent-capsule review;
- semantic mutation generation;
- compiler-guided localized repair;
- runtime-journal checking and drift events;
- authority controls preventing automatic weakening of trusted claims;
- evaluation against held-out formalization tasks.

Exit gate:

- the deterministic assistant improves completion on the frozen three-task
  baseline;
- no benchmark pass depends on silently modifying the trusted property;
- runtime drift is classified and linked to its model and implementation;
- human reviewers can inspect intent, evidence, witness, and residual gaps from
  one artifact graph.

Gate result: passed as protocol evidence for these three fixtures. It does not
measure generalization, model-based agent performance, or production repair;
the runtime journal is synthetic and has no authenticity or completeness
attestation.

### Phase 7 — Independent research extensions

Completed 2026-07-18 for one experimental track: conservative graded-resource
modalities. `nmlt-grades` computes a checked product of declared cost,
privacy-loss, energy, and uncertainty upper bounds; supports sequential,
choice, conservative-parallel, and finite-repeat composition; preserves
overflow and unbounded repetition as `unknown`; and reproduces a
schema-validated provider-pipeline result with negative controls.

Completed track deliverables:

- RFC 0012 and a separate `.nmltg` prototype language;
- exact, exceeded, and unknown budget outcomes;
- finite law sampling plus a noncommutative-algebra rejection control;
- pinned Lean 4.30 proofs for the exact mathematical product algebra,
  nonempty-finite choice distribution, product order, and Boolean budget
  predicate;
- source-, implementation-, executable-, toolchain-, schema-, and
  validator-bound evidence;
- explicit trusted-annotation assumptions and residual gaps.

Prototype gate result: passed only for this independent modality experiment.
The standalone mathematical algebra is kernel-checked, but the promotion gate
remains closed: the Rust `u64` implementation is only manually aligned with
Lean's `Nat` model; the parser, analyzer, and grade-preservation connection are
unverified; atom annotations are trusted; privacy sensitivity and mechanisms
are not checked; energy has no operational measurement model; and uncertainty
is an abstract saturated upper bound. The experiment does not strengthen a
main-language NMLT claim.

Other possible tracks remain future work:

- guarded cubical equality and proof-relevant equivalence;
- hybrid dynamics and differential invariants;
- probabilistic temporal properties;
- alternative grade algebras and language-integrated quantitative modalities;
- categorical libraries for reusable open-system composition.

Each additional track requires its own RFC, metatheory, implementation,
backend, negative controls, and benchmark. None blocks the first alpha.

## 8. Initial execution backlog

This historical backlog is complete. Integration and release-hardening work
was tracked and closed at its bounded scope under M8 rather than being
retroactively folded into these Phase 0–2 tasks; broader integration now lives
in the post-M8 focus.

### P0 — Decisions that unblock implementation

- [x] `NMLT-P0-001`: Decide and record license.
- [x] `NMLT-P0-002`: Write the lexical grammar RFC.
- [x] `NMLT-P0-003`: Select the lossless syntax-tree representation by ADR.
- [x] `NMLT-P0-004`: Define canonical diagnostics and snapshot format.
- [x] `NMLT-P0-005`: Define canonical source identity and hashing requirements.
- [x] `NMLT-P0-006`: Create the trusted-computing-base threat model.

### P1 — Real frontend

- [x] `NMLT-P1-001`: Replace the structural scanner with a token stream.
- [x] `NMLT-P1-002`: Parse modules, data declarations, systems, and state.
- [x] `NMLT-P1-003`: Parse action blocks and explicit updates.
- [x] `NMLT-P1-004`: Preserve comments and whitespace in a lossless tree.
- [x] `NMLT-P1-005`: Implement parser recovery without accepting ambiguous
  semantics.
- [x] `NMLT-P1-006`: Add formatter round-trip and idempotence tests.

### P1 — Mathematical core

- [x] `NMLT-P1-101`: Formalize candidate behavior-type formation rules.
- [x] `NMLT-P1-102`: Formalize state and action typing judgments.
- [x] `NMLT-P1-103`: Select the v1 capability discipline.
- [x] `NMLT-P1-104`: Define observation and stuttering semantics.
- [x] `NMLT-P1-105`: Choose a mechanization environment and repository layout.
- [x] `NMLT-P1-106`: Prove or refute refinement congruence for the candidate
  composition rule.

### P1 — Benchmark integrity

- [x] `NMLT-P1-201`: Add intent capsules for all five provider cases.
- [x] `NMLT-P1-202`: Freeze property identities and expected witnesses.
- [x] `NMLT-P1-203`: Record exact source-corpus provenance without copying
  protected material.
- [x] `NMLT-P1-204`: Add malformed, vacuous-property, and weakened-invariant
  controls.
- [x] `NMLT-P1-205`: Add schema validation to `make ci` without depending on a
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
- human intent agreement on held-out tasks (unmeasured in the v1 deterministic
  baseline).

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

Historical scaffold scope: unstable syntax, structural tooling, and design
artifacts only. The working tree has advanced beyond this scope, but no new
release tag is asserted here.

### `0.1.0` — Provider-attempt research alpha

Requires the Phase 3 exit gate: typed behavioral core, executable finite model,
four detected semantic mutants, structured traces, and bounded evidence. The
bounded implementation is present; independent clean-checkout reproduction is
still required before cutting the release.

### `0.2.0` — Temporal and refinement alpha

Requires fairness, stuttering, hiding, one checked refinement, and one concrete
trace adapter. These exist for finite fixtures only; no tag or general temporal
language claim follows automatically.

### `0.3.0` — Multi-engine alpha

Requires two independent verification routes and fail-closed disagreement
handling. The implemented routes share one finite Boolean VC and one Rust
process, and the source-to-VC translation remains unverified.

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

**M1: Lossless Frontend Contract** completed on 2026-07-18 with RFC 0003, ADR
0002, the immutable lossless tree, deterministic recovery, stable diagnostic
snapshots, preservation formatting, and corpus-wide round-trip tests.

The completed M1 gate is:

1. a lexical grammar RFC is accepted;
2. a lossless syntax-tree ADR is accepted;
3. the three existing examples and five provider benchmark files parse into a
   lossless tree;
4. formatting is idempotent and preserves comments;
5. malformed fixtures return stable diagnostics;
6. syntax-only commands make no semantic assurance claim and the structural
   `evidence` scaffold remains explicitly `unknown`.

**M2: Typed Provider Core and Checked Metatheory** completed on 2026-07-18.
Rust and Lean agree on a frozen provider correspondence vector covering
simultaneous updates, frames, blocked states, affine authority, and property
indexing. Lean checks preservation, progress/blockage, frame soundness,
capability no-duplication/no-fabrication, and indexing without `sorry` or a
project-defined axiom. Full parser-to-Lean compiler correctness is not claimed.

**M3: Reproducible Provider Model Checking** completed for provider suite v2:
one bounded reference acceptance, four structured mutant refutations, exact
identities, independent result readback, and clean-clone reproduction at
`e3f7ec6ae2d14ade78183ff78d58f7198cb76858`.

**M4: Finite Temporal and Runtime Evidence** has its finite milestone complete
for generic lasso/fairness, refinement, synthetic journals, and provider
`NoBlindReplay` over an independently replayed, manually audited observation
graph. General source-derived temporal checking remains open.

**M5: Finite Multi-engine Composition** completed for one hand-constructed
two-observable provider VC, including certificate checking and fail-closed
negative controls.

**M6: Authority-bounded Repair Protocol** completed as a deterministic
three-task baseline linked to the Phase 4 runtime-drift evidence. It is not an
LLM capability result.

**M7: Graded-resource Experiment** completed as a separate annotated-plan
track with a kernel-checked mathematical product algebra. Its promotion gate
remains closed pending typed-core/analyzer integration, verified Rust
correspondence, and semantics for the accepted resource annotations.

**M8: Integration, Independent Reproduction, and Release Hardening** completed
on 2026-07-18 at the bounded research scope:

1. run the complete documented gate from an independent clean checkout and
   record the environment and outcomes;
2. review the final TCB and evidence identities after the implementation
   freeze;
3. keep the frontend, provider core, temporal graph, finite VC, agentic
   protocol, and graded experiment as distinct assurance subjects;
4. decide whether the bounded provider slice is ready for a `0.1.0` tag.

Gate result: items 1–3 passed and are recorded in
[`docs/reproduction-2026-07-18.md`](docs/reproduction-2026-07-18.md) and the
final TCB/evidence audit. Item 4 is decided **not yet**: no `0.1.0` tag is
created because the corrected P model has not been rerun, source-to-temporal
graph and source-to-VC mappings remain manual, and the general surface-to-core
correspondence is unverified. Work continues as pre-alpha integration without
weakening the completed bounded milestones.

**M9: Integrated Source-to-Typed-Core Contract** started on 2026-07-19.
The accepted contract is specified in
[RFC 0013](rfcs/0013-source-to-typed-core.md), informed by the
[2026-07-19 archive research note](docs/research-notes/source-to-typed-core-and-project-identity-2026-07-19.md).

The authoritative pipeline is:

```text
exact source set
  → lossless CST
  → complete untyped surface AST
  → resolved HIR with stable IDs
  → bidirectional elaboration + derivation certificate
  → explicit typed CoreProgram
  → independent kernel validation
  → CheckedProgram for engines and backend adapters
```

M9 deliberately supports a narrow but complete vertical slice: acyclic
explicit imports; closed enumerations; `Bool`, `Nat`, and `Int`; systems and
scalar state; action inputs; total pure expressions; guards; simultaneous
updates; `Once<T>` capabilities; observations; and safety/temporal property
ASTs. Records, maps, indexed types, general functions, open ports,
language-integrated grades, refinement compilation, liveness proving, and code
generation remain outside this gate.

Deliverables:

1. define canonical module, symbol, type, core-node, and derivation identities;
2. add implemented `nmlt-hir` and `nmlt-kernel` boundaries only when their
   responsibilities and tests are real;
3. remove the provider compiler's second parser and make all supported source
   pass through the lossless frontend and resolver;
4. emit an inspectable elaboration certificate and independently validate it
   before constructing `CheckedProgram`;
5. migrate the bounded engine to consume checked core rather than reparsing
   source;
6. bind source-set, resolver, elaborator, kernel, core, and certificate
   identities in evidence;
7. mechanize the supported fragment's typing and operational correspondence.

Substage status (2026-07-19):

- [x] **M9-001 — Freeze the contract.** RFC 0013 fixes the supported fragment,
  identity encodings, numeric policy, certificate DAG, resource limits, and
  Rust/Lean boundary.
- [x] **M9-002 — Complete surface projection.** The hierarchical projection is
  ordered, explicit on unsupported/recovered nodes, and checked against an
  independent CST-origin census.
- [x] **M9-003 — Resolve modules and names.** `nmlt-hir` consumes the canonical
  projection, resolves a closed acyclic import graph, parses every admitted raw
  type/expression into source-derived HIR, assigns owner-derived action locals,
  emits a canonical all-reference `ResolutionMap`, and replays exact spelling,
  candidates, graph closure, and reference-map bijection before construction.
- [x] **M9-004 — Define explicit core.** `nmlt-ir` defines closed typed terms,
  systems, actions with exact frames, capabilities, observations, dedicated
  system-indexed property constructors, HIR-origin/core-node identities,
  canonical program identity, and fail-closed structural/resource validation.
  This is not yet an elaboration or kernel-accepted `CheckedProgram`.
- [ ] **M9-005 — Implement bidirectional elaboration.**
- [ ] **M9-006 — Implement the independent kernel.**
- [ ] **M9-007 — Migrate the engine and remove the second parser.**
- [ ] **M9-008 — Bind semantic evidence identities.**
- [ ] **M9-009 — Mechanize correspondence.**
- [ ] **M9-010 — Reproduce and audit the completed vertical slice.**

Required promotion obligations:

- every accepted surface construct is translated exactly once and no
  unsupported construct is silently skipped;
- resolution is deterministic, unique, import-closed, and span preserving;
- a checked elaboration derivation establishes that the emitted core is typed;
- initialization, guards, simultaneous updates, frames, and action inputs are
  preserved in both directions for the supported fragment;
- affine capability use and behavior/property indexing survive elaboration;
- equivalent source sets have a defined canonicality policy, while changed
  semantic inputs invalidate prior evidence;
- Rust fixtures and the Lean model share identity-bound correspondence vectors.

Negative controls include unresolved and ambiguous names, cyclic imports,
forged or stale certificates, dropped declarations, reordered simultaneous
updates, implicit frames, `Nat`/`Int` coercion, temporal operators admitted as
ordinary Boolean calls, duplicated affine capabilities, cross-system property
use, and source/core identity substitution.

The M9 gate closes only when the canonical provider source reaches the existing
typed engine through this pipeline, all supported canonical examples either
compile or fail with a declared feature-boundary diagnostic, seeded semantic
mutants retain their classifications, independent certificate readback rejects
every negative control, and no manual source-to-core path remains in the
promoted slice. After M9, the next research sequence is behavior-indexed
temporal typing, proof-relevant refinement, compositional open-system semantics,
and richer quantitative mathematics—each with its own mechanization and
comparison gate.
