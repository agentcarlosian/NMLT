# R0 deterministic baseline contracts

Date: 2026-09-06. Contract version: `nmlt-r0-baselines-v1`.

These contracts define three small programs using existing Python and Lean
tools. They calibrate result handling, exact targets, budgets, and recovery
before an NMLT runtime is implemented. They are not an NMLT language release,
a discovery benchmark, or a claim of new mathematics. The proposed runtime
semantics are in [Draft RFC 0014](../rfcs/0014-executable-workflow-profile.md).

The machine-readable case configuration is
[`examples/baselines/manifest.json`](../examples/baselines/manifest.json);
the runner is
[`tools/baselines/run_baselines.py`](../tools/baselines/run_baselines.py).
Those files and their tests determine the executable field names and behavior.
An implementation/configuration change creates a new measured context; do not
silently compare or resume records from a different context.

## Running the calibration corpus

Run from the repository root with Python 3 and a compatible Lean checker:

```sh
python tools/baselines/run_baselines.py --scenario all --output-dir target/r0-baselines --lean-command-json '["lean"]'
```

`--lean-command-json` takes a JSON argument vector rather than shell source.
Use the actual configured command prefix when Lean runs through a wrapper or
WSL. Lean source is supplied through standard input. The proof and discovery
lanes require the exact Lean version selected by the repository pin and the
baseline's minimum patched version, 4.33.1. Availability alone does not satisfy
that compatibility gate. The worker lane does not require Lean.

The proof interruption exercise is:

```sh
python tools/baselines/run_baselines.py --scenario proof --output-dir target/r0-proof --lean-command-json '["lean"]' --stop-after 1
python tools/baselines/run_baselines.py --scenario proof --output-dir target/r0-proof --lean-command-json '["lean"]' --resume
```

Use a dedicated output directory. Outputs are experimental local records, not
checked-in generated results or published proof artifacts. This contract does
not authorize a network call, model workload, external message, or deployment.
The runner makes no model calls.

Each selected lane writes `<output-dir>/<lane>/record.json`; Lean lanes also
write their candidate `.lean` files there. Exit code 0 includes a deliberate
`--stop-after` interruption, so inspect the record's `status` rather than
treating process success as completion. Exit code 1 means a lane missed its
expected outcomes; exit code 2 reports configuration/setup errors with an
unknown mathematical outcome.

## Common identity and result contract

Every record identifies the run, task, and attempt. Resume matching binds the
manifest, runner/source identity, exact selected Lean command and reported
version where applicable, and the fixed target/input context. The records
identify those inputs; they do not prove the implementation interpreted them
correctly. A reported version is not a cryptographic identity for an entire
toolchain or dependency closure.

Process outcomes and semantic judgments are separate. An unavailable checker,
incompatible version, timeout, interrupted run, or tool failure cannot become
mathematical refutation. A proof rejection concerns the supplied candidate;
it does not establish that its target is false. The Lean runner applies its
written empty-transitive-axiom policy to accepted calibration proofs.

Accepted cached proofs are freshly checked on resume. New strategy attempts
and validation of cached results are accounted for separately: rechecking an
artifact is real observed work but does not invent another generated
candidate. Resume under a changed context is rejected or requires a fresh
run; it cannot inherit earlier acceptance.

The baseline Lean claim is ordinary Lean-process acceptance plus recorded
axiom-policy evidence for the fixed target. It is not an independent NanoDA
result, a verified source translator, a proof of the Python runner, or a
certificate accepted by NMLT's behavioral artifact checker. Stronger final
acceptance remains subject to the compatible checker/exporter/comparison
pipeline in the [practical plan](practical-language-plan.md).

## Contract A: fixed Lean proof strategies

**Input:** the exact target `forall n : Nat, 0 + n = n`, with the fixed
calibration environment and three predetermined strategies. No model selects
or edits the target.

| Strategy | Required observation | Meaning |
|---|---|---|
| Wrong term | Lean rejects the candidate | Ordinary type/proof rejection; the target remains open |
| Existing lemma | The fixed target checks using the existing lemma | Known result, accepted under the recorded checker policy |
| Induction | The same fixed target checks using induction | Alternate proof of the same known result |

**Output:** candidate Lean source, diagnostics, checking/axiom-policy results,
identities, and observed elapsed times for the fixed strategies. The exact
target is supplied by the runner's reviewed template; a strategy does not
complete the task by compiling an unrelated true statement.

**Budget:** at most three new strategy attempts. Each Lean checking invocation
has a 30-second timeout and sets Lean's `maxHeartbeats` to 200,000. These are
elaboration/process limits, not a proof-search completeness bound or a total
run-time guarantee. The baseline does not implement a memory limit or a
cross-platform process-tree sandbox; a command wrapper's descendants require
their own host-level containment. Interruption after the first strategy must
leave resumable records without falsely completing the target.

**Acceptance gate:** distinguish the expected rejected candidate from the two
accepted proofs; complete the interrupt/resume exercise; freshly recheck
reused accepted proofs; detect changed context. All reported successes must
concern the same frozen target. No throughput comparison or claim about model
quality follows from three canned strategies.

## Contract B: conjecture, counterexample, revised statement

**Input:** natural-number addition, with finite test inputs `a, b` in
`{0, 1, 2, 3, 4}`, and two predetermined candidates:

| Candidate | Claim | Required result |
|---|---|---|
| Left projection | For every `a, b : Nat`, `a + b = a` | Find `(0, 1)` and check that it refutes the universal claim in Lean |
| Commutative | For every `a, b : Nat`, `a + b = b + a` | Check all 25 finite pairs and separately check the universal `Nat.add_comm` proof |

**Output:** both exact claims, finite evidence, the refuting witness and Lean
check, and the proof of the second claim. The second claim is an explicit
revision/new candidate; proving it does not solve the false first claim.
Python predicates and Lean target templates must express the same reviewed
claims. A false predicate result alone is computational evidence until its
claimed counterexample is checked in the stated formal context.

**Budget:** two candidates and the fixed 25-pair finite domain. Each Lean
invocation has the same 30-second timeout. The witness must be checked against
the negated universal claim, not merely against an unrelated arithmetic fact.

**Acceptance gate:** check the witness, label the 25-pair sweep as finite
evidence, and label commutativity as a known theorem. This is a calibration
exercise in claim discipline. It does not meet the later goal of a useful
external-domain mathematical discovery workflow on its own.

## Contract C: local permit-controlled work

**Input:** four predetermined scenarios, each with at most one job slot and
two generations. The worker performs pure local Python computation. Its
authority controller is an in-memory baseline, not the future NMLT interpreter
and not a proof that arbitrary host processes respect ownership.
Computation is synchronous; delayed response delivery is simulated. Its local
`cancelled` status ends acceptance for that attempt, not a claim that a running
host process acknowledged cancellation or stopped computing.

| Scenario | Fixed input and required outcome | Charged dispatches |
|---|---|---:|
| Success | Transfer the permit, later dispatch `3`, collect `9` | 1 |
| Failure | Dispatch `-1`, record the typed failure | 1 |
| Cancel before dispatch | Cancel while work remains undispatched | 0 |
| Cancel, late response, slot reuse | Dispatch `4`, cancel, reuse the slot with a distinct generation for `5`; reject late/duplicate results and collect `25` for the active attempt | 2 |

**Output:** explicit control/events, run/task/attempt/generation identity,
typed outcomes, immutable collected values where appropriate, and dispatch
accounting. Slot reuse must not make the old generation current again. A late
response may be recorded, but cannot restore cancelled authority or produce a
second collected result. Cancellation does not refund a dispatched attempt.

**Acceptance gate:** all four required outcomes hold with the stated dispatch
counts and generation bounds. A no-op or always-failing worker fails this
contract even if it is cheaper. These four scripted scenarios are the coverage
denominator. They are not exhaustive model checking, a liveness theorem, or an
exactly-once external-effects guarantee.

## Frozen evaluation rules and corpus separation

1. Freeze manifest and implementation identities before collecting a run.
   Record deliberate changes as a new context and preserve failures in the
   accounting. Never replace a failed candidate after seeing its score while
   presenting it as part of the same frozen comparison.
2. The three programs above are **public calibration/development cases**.
   Their known outcomes are intentionally visible. No R0 held-out evaluation
   corpus or comparative user-study result exists merely because these run.
3. Runner tests with a controlled checker validate orchestration behavior only.
   A separate real Lean run is needed for the proof and discovery acceptance
   claims. Keep missing tooling distinct from a rejected mathematical case.
4. Record observed elapsed time separately from declared attempt/domain bounds.
   No model usage is incurred. CPU work, memory, charges, and other measures
   that are not collected must remain unavailable rather than inferred from
   the semantic grade or treated as measured zero.
5. Before R2--R5 comparison, freeze a separate evaluation set and budgets that
   were not used to tune syntax, prompts, or strategies. Disclose public-data
   contamination limits. Record the same host, checker, cache/warm-up policy,
   task inputs, model responses or model configuration, and resource limits
   for both existing-tool and NMLT runs.
6. Evaluate correctness before speed. A budget exhaustion is an explicit
   inconclusive result; dropping work, narrowing a target, or weakening the
   axiom policy cannot count as improvement. All finite claims name their
   exact input/trace bounds and coverage count.
7. For the later protocol search, freeze all permitted input/environment
   traces and required outcomes within the finite model. Check every case
   before scoring. Preserve completion where required and reject added
   deadlock, missing enabled work, or an unjustified cheap failure branch.
8. The [pilot protocol](r0-pilot-protocol.md) measures setup, interventions,
   diagnosis, replay, and reuse. It supplies task cards and observation fields,
   not invented participants or completed user evidence.

## R0 completion record

Record the actual checker command/version and case results alongside their
output paths. This R0 lane completes when all three baselines have their
expected outcomes, the published contracts match implementation, and the
protocol is prepared. A diagnosed tooling blocker is useful progress but is
not a passing Lean baseline: the checker-dependent part remains unfinished
until it runs successfully. Recruiting specific people is not an R0
requirement; user evaluation remains a later activity.
