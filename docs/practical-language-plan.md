# NMLT practical language plan

Date: 2026-09-06. Status: detailed design companion to the active execution plan; specific language and trust-boundary changes require disposition through the RFC process. Audience: NMLT maintainers and prospective users in AI-assisted Lean development, mathematics, and software engineering.

This design serves all three audiences through one executable language and shared libraries. [Plan.md](../Plan.md) is authoritative for execution priorities and milestone status, including R0–R5 and the retained M1–M3 foundation. The [research and alternatives](research-notes/practical-language-strategy-2026-09-06.md) explain the recommendation and cite its evidence. No feature below is implemented merely because it appears here.

## Outcome

Make NMLT a useful language for programs that coordinate computations, resources, and checkable claims. A user should be able to read inputs, call a tool, inspect its typed result, branch or retry within a budget, reuse an output, and reproduce what was checked.

The first alpha must include three complete workflows:

| User | Program they can write and run | Useful output |
|---|---|---|
| AI/Lean developer | Compare bounded proof strategies on a pinned Lean project, recover interrupted attempts, and check the final result | Ordinary Lean proof patches, diagnostics, and an honest cost/result comparison |
| Mathematician | Explore a reviewed lemma family, generate and refute conjectures, and prove a surviving statement | Counterexamples, explicit revised conjectures, reusable Lean lemmas, and a human-readable explanation |
| Software engineer | Execute a permit-controlled worker protocol, including receive-then-use, cancellation, and failure | Running local software, model/trace comparisons, and explicitly scoped invariant evidence |

All three use the same interpreter, project format, adapter protocol, and result records. Lean remains the language for mathematical definitions and proofs. The proposed NMLT layer supplies executable workflow composition and resource contracts.

## Decisions to settle before expanding syntax

1. **One source meaning:** share behavioral parsing, names, types, source spans, and lowering facilities with the retained frontend where appropriate. Record which route is authoritative for each construct. Do not require behavioral programs to pass through a kernel that currently checks a different ordinary language.
2. **One transition meaning:** derive exploration and execution from the same specified step relation, with external responses represented explicitly. Finite model checking applies only to a declared finite abstraction of host data and effects.
3. **Native Lean interchange:** refer to Lean projects, declarations, and proof patches. Avoid translating general mathematics through the current finite behavioral AST.
4. **Two kinds of resource:** reusable proofs and immutable artifacts are ordinary persistent values; job ownership, exclusive writes, and budget reservations can be affine capabilities. Modelled grades and observed resource expenditure are separate fields.
5. **Scoped evidence:** distinguish a typed program, an observed execution, an accepted finite trace, a proved model property, and a Lean theorem. No single success flag should merge these meanings.
6. **Compatibility and evolution:** preserve `behavior-core-v1` fixtures as versioned regression evidence. Specify `behavior-core-v2` decoding, rejection of unsupported versions, source maps, and migration examples before switching defaults.

Runtime work changes the current deferral in [ADR 0004](decisions/0004-language-mathematics-pivot.md). Open a focused RFC for that extension; do not revive removed historical components automatically. The ideas in [postponed RFC 0011](../rfcs/0011-authority-bounded-agentic-repair.md) are design input, not active implementation evidence.

## R0 — Establish the checker and product baseline

Owners: integration maintainer and Lean maintainer. Dependencies: none.
Completed at calibration scope on 2026-09-06; see the
[checker and workflow evidence record](reviews/r0-checker-and-baselines-2026-09-06.md).
The requirements below describe the completed baseline gate, not an implemented
NMLT interpreter. R1 completed at finite scope; R2 completed at local pre-alpha scope on 2026-09-09.

- Review a compatible patched Lean version, with 4.33.1 as the minimum candidate identified by this research. The former 4.30 pin predates the August kernel/runtime fixes; R0 upgraded it to 4.33.1. Update the exporter and independent checker as a compatible set, then rebuild and freshly recheck the existing corpus. Use the established export/comparator path; the postmortem's `lake check` command is a future feature, not this plan's assumed interface. [Upstream postmortem, 2026-08-24](https://leodemoura.github.io/blog/2026-8-24-postmortem-for-the-kernel-soundness-bug-hunt/)
- Preserve the existing passing baseline and audited working changes in separately reviewable commits when implementation is authorized. A toolchain migration must not silently weaken theorem statements or the axiom policy.
- Write the three workflows above as input/output contracts and small baseline programs using existing Lean/Python/Rust tools. Use batch Lean project checking first; evaluate LeanInteract only after compatibility with the patched checker is established. Its published supported range currently ends before 4.33.1. [LeanInteract](https://github.com/augustepoiroux/LeanInteract)
- Freeze benchmark definitions, baseline configurations, model/resource budgets, success meanings, and stop conditions. Start with deterministic adapters and existing proof attempts; model access and compute limits are configuration decisions for implementation, not prerequisites for this planning work.
- Prepare reusable pilot task cards and an observation form for each audience. As directed by the maintainer, R0 does not require naming or recruiting participants; independent user evaluation remains a later R5 activity.

Exit: a fresh checker compatibility record; one complete baseline example per audience; a written explanation of the specific friction NMLT will attempt to reduce. Do not deepen a language feature whose benefit is indistinguishable from an existing small library.

## R1 — Complete the semantic basis for real execution

Owners: Lean/semantics maintainer with Rust support. Dependencies: R0 checker baseline. This is the principal research uncertainty.

R1 now implements `ResourceDynamics.Behavior`, constructed nested examples,
v2 decoded initial execution and finite paths, source-level received-authority
continuation, and ownership-origin results. Frozen value and resource graphs
compare Rust exploration with the actual Lean relations. The primary decoded
synchronization also lifts through the unified refinement theorem with an
initialized abstract image. [RFC 0015](../rfcs/0015-unified-resource-bearing-behavior.md)
and [RFC 0016](../rfcs/0016-decoded-finite-execution.md) remain Under review.
The [completion evidence](reviews/r1-finite-execution-2026-09-06.md) records the
passed M1–M3 gates; the authoritative execution plan keeps R2 as the next milestone.

Implement the existing M1–M3 in reviewable increments:

- Unify dynamic ownership with behavior state, initialization, observations, and product steps.
- Preserve open ports, direction, payloads, and relevant hiding under composition. Test nested binary composition before promising arbitrary n-ary composition.
- Relate the old and new semantics by a proved projection/replacement result with its exact scope. State formation policy separately from the premises needed by lifting.
- Extend decoded artifacts with initial ownership and dynamic transitions. Exhibit an actual initial synchronized step and exact authority movement.
- Define finite reachability and allow received authority to be consumed or transferred in a later action. Prove the relevant ownership invariant over those paths.
- Bring the Rust evaluator into parity with the finite Bool/Unit/enum slice. Compare accepted states, enabled steps, and bounded paths with Lean on a frozen corpus.

Exit: decoded receive-then-use and receive-then-transfer examples, meaningful local/synchronized/hidden cases, and checked finite paths from real initial states. Negative controls must test the target property; a model rejected by a formation rule is not automatically a counterexample to a theorem with that rule removed.

An intermediate theorem is useful progress. A conditional lifting result alone does not satisfy this exit gate.

## R2 — Deliver the smallest useful executable language

Completed on 2026-09-09 at local, finite, pre-alpha scope: typed projects,
dependency locks, finite user safety invariants, affine handle transfer,
contained local processes, variable Init proof terms and durable source/project
resumption. The [completion audit](r2-completion-tracker.md) and
[validation record](reviews/r2-completion-2026-09-09.md) cover the full
requirements. The increment descriptions below are implementation history.
RFC acceptance and publication review remain separate.

Owners: Rust/compiler maintainer and integration maintainer. Dependencies: R1 for formal execution claims; interface and UX prototypes may proceed during R1.

Started on 2026-09-06. The [first increment](r2-local-execution.md) supplies
bounded execution and replay over the existing finite v2 slice. The requirements
below still govern the complete milestone; subsequent increments add source
functions, packages, and the first local job adapter.

The second increment supplies a [Rust job runtime prototype](../crates/nmlt-runtime/README.md)
with bounded identities, revisioned control, cancellation/settlement, and locked
journal recovery. Supported Lean adapters remain integration work; the
subprocess example is not completion of this milestone.

The third increment adds [pure source functions](r2-pure-workflows.md), scalar
entry inputs, local modules, typed outcomes and exhaustive matching, immutable
bindings, checked arithmetic, and bounded evaluation/replay. This profile is
executable-only. The fourth increment adds nominal records, structured inputs,
bounded lists/folds, and aggregate value limits under RFC 0020. Cross-file
packages are now provided by the fifth increment: canonical imports, per-file
resolution, typed library components, and complete source manifests. The sixth
increment adds [source local jobs](r2-source-jobs.md): typed worker effects,
explicit budgets, bounded pipes/timeouts, durable context, replay, and recovery
inspection. The seventh increment starts [Lean and asynchronous host APIs](r2-lean-async.md)
with fixed proof templates, independent child deadlines, and private control
handles. The eighth increment adds [scoped source controls](r2-source-async.md)
with checked branch/loop ownership and captured-session replay. Later increments implement closed Init terms, transferable handles, finite
safety predicates, process containment, the project loop and durable resumption.
Completed validation is recorded in the audit above.

Deliver a local interpreter before an optimizing compiler or distributed runtime. Initially support a single host and a small bounded number of concurrent jobs.

Start with a fixed pool of job slots and a declared maximum number of attempts per run. Bind each result to its slot, attempt/generation, owner, inputs, and expected output type; stop before identity counters can wrap or be reused ambiguously. Specify allocation, transfer, cancellation, and settlement against that bounded universe before adding dynamic unbounded allocation.

| Facility | Minimum useful behavior | Boundary to specify |
|---|---|---|
| Program structure | Entry point, modules/imports, reusable parameterized components | Closed dependencies and stable name resolution |
| Values | Records, tagged outcomes, finite enums, basic total functions, pattern matching, bounded collections/iteration | Which values are finite model state and which remain opaque host data |
| External work | Start, observe, cancel, and collect a typed job; local Lean and one simple worker adapter | Response validation, permission ownership, timeout, and failure |
| Resource use | Reserve and account for job attempts and declared work budgets | Observed usage versus semantic grades; cancellation does not erase prior spend |
| Recovery | Persist completed artifacts, resume attempts, reject stale results | A crashed external action may have completed; uncertain outcomes stay explicit |
| Project loop | `init`, `run`, `test`, `replay`, `fmt`, and a precisely named checking command | Finite and workflow routes retain separate scopes; the local project loop is implemented |
| Diagnostics | Source spans, expected/actual values, related locations, JSON output | One error vocabulary shared by CLI and future editor integration |
| Dependencies | Local packages, exact tool/dependency lockfile, compatibility errors | A public package registry is unnecessary for the alpha |

Give every added construct a semantic disposition: either it lowers into the specified core with an appropriate Lean interpretation and validation, or it is explicitly executable-only. The R1 theorem does not automatically cover new records, functions, iteration, or effects. Applying a finite model property to concrete execution requires a stated simulation/admissibility obligation for the abstraction, or an explicit trusted-abstraction assumption.

Implement one complete user-defined safety-invariant feature: preserve its exact predicate in the artifact, generate initialization and step-preservation obligations, and return Lean-checked evidence or a counterexample with its finite scope. Keep unsupported property forms as explicit errors. This closes the practical gap between parsing a property declaration and actually checking a user's requirement.

The host adapter is a named trust boundary. Use a small process/serialization interface that can be implemented in Rust or Python. Record effect requests and responses, then validate outputs with the appropriate checker. Existing OS process isolation and resource limits should contain untrusted generated work; no claim about all host behavior follows merely from typing the adapter.

For recovery, provide idempotency keys and explicit acknowledgement states where the host supports them. Do not promise exactly-once external execution from local ownership tracking. Track reservation settlement and uncertain in-flight work after cancellation or restart.

Exit: a user changes real input, runs a complete local NMLT program, handles a failed tool call, reuses its successful output, and replays the record without editing JSON by hand. The interpreter and explorer share the implementation of their common step operations, and the remaining Rust-to-Lean correspondence limits are documented.

Lifecycle controls must include a delayed result from a previous occupant of a reused slot: it cannot settle the new attempt, release its reservation, or acquire its authority. Exercise cancellation followed by a late completion and restart with an uncertain in-flight action.

## R3 — Make existing Lean projects a supported workflow

The first [bounded local-task increment](r3-lean-tasks.md) implements source and
target binding, separate proof candidates, actual dependency exports and fresh
independent proof rechecking. The second increment discovers source imports
with the pinned Lean/Lake parser across explicitly selected local/vendor roots
and checks the retained closure again during proof reconstruction.
The third increment captures exact proof exports up to 16 MiB in bounded files,
records byte counts and digests, and repeats independent checking on a fresh
reconstruction; ordinary process limits are unchanged.
R3 remains in progress; the integration and scale
requirements below remain the milestone's complete gate.

Owners: Lean integration maintainer. Dependencies: R2; a batch adapter spike and baseline can start in R0.

- Accept a pinned Lean project and a reviewed target declaration. Record the target's environment, assumptions, imported definition closure, and exact statement identity. Source hashes aid identification; Lean-side checking must establish the formal target match.
- Separate candidate statements, candidate proof patches, and checked results. Changing a statement creates an explicit revision and invalidates dependent acceptance records. A proposed mathematical generalization is legitimate, but it does not solve the old obligation automatically.
- Reuse existing Lean project tooling for diagnostics and retrieval. Compare LeanInteract with the normal build/REPL route; add LSP integration for editor workflows when needed. Add Pantograph only if coupled subgoal search is a demonstrated requirement.
- Reconstruct ordinary Lean files and check them in a clean pinned environment. Inspect transitive axioms under a written policy; use trusted challenge comparison and external rechecking for final accepted proof artifacts. An empty tactic goal list or a successful worker response is insufficient.
- Represent the proof-dependency graph as exact formal dependencies with separate draft/planning status. Incomplete or provisional lemmas cannot become a closed root theorem by being marked complete in the scheduler.
- Export readable proof patches, minimized imports where practical, definitions, dependencies, and explanations linked to exact declarations. Keep familiar Lean editor workflows available.

Exit: the frozen proof task set completes with ordinary accepted Lean output or an explicit failure/unknown outcome; every reported success can be independently rechecked without the generating model session. Identical model responses are not required for replay: replay checks recorded artifacts and execution decisions.

## R4 — Make mathematical discovery an explicit program

Owners: mathematical/domain reviewer and integration maintainer. Dependencies: R3 for integrated execution; question selection and deterministic baselines can start during R0–R1.

The workflow is: select a reviewed question, generate candidates, search for counterexamples, attempt proofs, examine generalizations, assess novelty, and publish an inspectable result for review. Publishing means preparing a reviewable artifact; external publication remains a separate action.

Keep these outcomes distinct:

| Output | Evidence required | What it does not establish |
|---|---|---|
| Conjecture | Exact statement, assumptions, provenance | Truth |
| Computational support | Instances, bounds, evaluator, inputs and outputs | A universal theorem |
| Refutation | Witness checked against the actual negated claim | Failure merely because a prover timed out |
| Lean theorem | Checked proof, target and environment, axioms/dependencies | Faithfulness to informal intent or novelty |
| Candidate contribution | Prior-art search, explanation, independent domain review | An automatic certificate of mathematical importance |

First experiment: classify premises of an NMLT composition theorem. Freeze the conclusion and a family of assumptions; compare deterministic subset analysis, ordinary Lean automation, and agent suggestions. A necessity witness must satisfy the retained assumptions and falsify the conclusion. Require admitted initial states and enabled behaviors so a weaker theorem is not vacuous.

Second experiment: find a useful simplification or resource improvement for the permit-controlled worker protocol. Before search, freeze permitted inputs/environment traces and their required observations and outcomes. Check every case within the declared finite bounds before scoring improvements, including completion where required; reject added deadlocks, missing enabled work, and cheap failure branches that replace required success. Compare with exhaustive or solver-based search on the same bounded problem. These are bounded contract checks, not a general liveness result.

Third experiment: a small finite construction laboratory, such as graphs satisfying a fixed exact predicate. Recover known constructions first. Check final objects in Lean, hold out instance sizes, and only then attempt a parameterized family or explanatory lemma. Distinguish isomorphic rediscoveries and new formalizations from candidate mathematical novelty.

Exit: reproduce a known result and obtain at least one useful checked generalization, counterexample, or construction that a domain reviewer can explain and reuse. A successful refutation is a useful outcome. No milestone guarantees a new theorem on a deadline.

## R5 — Validate usefulness for all three audiences

Owners: integration maintainer with independent pilot users. Dependencies: R2–R4.

R0 freezes the small public calibration corpus in its baseline contracts.
Finalize and freeze a separate evaluation corpus before comparative testing.
Its proposed initial scope is:

- 30 Lean tasks across ordinary library lemmas, multi-lemma dependencies, proof repair, definition/context sensitivity, and interrupted runs. Separate tuning projects from evaluation projects; report public-benchmark contamination limits.
- 12 small behavior scenarios covering ownership, composition, outcomes, and budgets, with a separately enumerated set of invalid variants. Exhaustive claims must name the exact finite bounds.
- Three discovery exercises with known answers or constructions for calibration, plus an optional expert-selected open instance. Keep failed and rejected candidates in the accounting.

These counts are proposed scope, not existing benchmarks or performance results. Finalize them against the actual implementation budget before collecting comparison data.

| Question | Measurement and gate |
|---|---|
| Are accepted results reliable within the declared scope? | Every reported proof/trace success rechecks; every specified invalid control is rejected. Report the finite test denominator, not a universal reliability claim. |
| Does the new language help? | Include a library-only baseline with identical adapters, orchestration algorithm, prompts, retrieval, caching policy, and checking policy. Compare language/runtime changes separately from proving-system improvements; also measure an existing Lean coding-agent stack. Record time, setup effort, interventions, proof maintenance, and resource use. |
| Does composition add value? | Reuse a component in a second workflow and demonstrate which property can be reused, under which premises, instead of rechecking everything from scratch. |
| Can each audience use it? | At least one independent user per audience installs, changes, runs, and explains a workflow. Treat this as qualitative pilot evidence, not market validation. |
| Is further language growth justified? | Pre-register a useful advantage before the comparison; a suggested target is a 20% reduction in median hands-on task time without worse acceptance quality. Explain sample size and uncertainty rather than presenting the threshold as statistical proof. |

If the language adds ceremony without a measured benefit, preserve the useful Lean library and adapters, simplify the surface, and repeat the focused comparison. If only one workflow is ready, label that limited readiness while continuing toward the all-three objective.

Use recorded-response replay to isolate deterministic workflow differences, then paired repeated live runs to measure model variability. Freeze the model, tasks, resource limits, and warm/cold-start conditions; vary the order of user trials to reduce learning effects. Report improvements caused by retrieval or prompts separately from benefits attributable to NMLT.

## Parallel work and sequencing

The critical dependency chain is R0 → R1 → R2 → R3 → R4 → R5. R0 baseline/adapter work, R2 source/UX design, and R4 domain preparation can proceed alongside the R1 mathematics; their prototypes must not borrow claims from unfinished semantics.

Staff by responsibility, even if one person holds several roles: compiler/runtime, Lean semantics/checking, and integration/domain evaluation. Avoid a calendar promise before R0 resolves tool compatibility and R1 resolves the central theorem design. At each review, require a runnable or checkable artifact and update effort estimates from that evidence.

The first implementation sequence should be:

1. Make the existing P3 audit patch reviewable as its own commit, then perform the Lean/exporter/checker compatibility update separately.
2. Record the executable-profile RFC and three baseline task contracts.
3. Start M1 semantic unification while a separate branch builds the batch Lean adapter and baseline comparison.
4. Add the decoded dynamic witness and receive-then-use path before promoting runtime resource guarantees.
5. Integrate the smallest interpreter and reuse it for all three pilot workflows.

## Work to defer until the pilots justify it

Own foundation-model training; a replacement for Lean/mathlib; a second proof kernel; an optimizing native backend; a general package registry; distributed deployment; automatic large-scale paper generation; and a separate bespoke UI for each audience.

Fairness/liveness remain after the unified safety and finite-path work. Richer refinement maps, larger data domains, non-binary composition, and verified translation are important later expansions. Each needs an actual user problem and an explicit semantics/evidence plan.

The purpose of this plan is to obtain a useful program people can run and a mathematical contribution people can examine, while keeping those two achievements distinguishable.
