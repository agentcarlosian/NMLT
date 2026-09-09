# R0 role-based pilot protocol

Date: 2026-09-06. Status: prepared evaluation plan; no user study or outreach
has been conducted under this protocol.

The user directed R0 to focus on the plan rather than target named people.
These task cards therefore describe roles and observable work. There is no
participant roster or nomination prerequisite for R0. Later R5 evaluation can
use the cards when the corresponding runnable NMLT workflows exist.

The [baseline contracts](r0-baseline-contracts.md) describe today's small
Python/Lean calibration programs. The [practical-language plan](practical-language-plan.md)
defines the later alpha. Do not ask someone to assess an unimplemented runtime
as though it were a finished language.

## Questions the evaluation must answer

- Can a user run a complete program, change its input, and understand failure?
- Does NMLT improve the same task over ordinary Lean/Python/Rust tooling?
- Can the user distinguish an observed result, finite evidence, a formal
  theorem, and a proposed mathematical contribution?
- Does reusable composition reduce work on a second task without hiding new
  assumptions?
- Are interruption, cancellation, stale output, and resource use understandable?

## Preparation and comparison discipline

Before conducting the later pilot, freeze a task packet per role: inputs,
expected outcomes, exact checking environment, a tutorial/calibration example,
a distinct evaluation task, permitted help, and budgets. Keep the evaluation
task out of implementation/prompt tuning. Choose accessible tasks that can be
completed with both the existing-tool baseline and the implemented NMLT
version; no task should require proving an open theorem.

Use the same host, dependencies, preloaded artifacts, and limits for each
comparison. If model help is allowed, hold its configuration and budget fixed
and record actual calls; a deterministic prerecorded-response replay is a
separate mode and cannot be reported as live model performance. R0 itself
requires no model calls.

Suggested initial session envelope: 10 minutes of orientation, 20 minutes per
tool condition, and 10 minutes of explanation/debrief. This is a proposed
human-evaluation budget, not an implemented runner watchdog. Finalize it and
the assistance policy before collecting results. Counterbalance condition
order across sessions when feasible and report learning/order effects. With
one session per audience, treat results as qualitative rather than a reliable
performance estimate.

## Task card A: AI-assisted Lean developer

**Role:** someone able to read a small Lean declaration and use an existing
Lean project. Expertise in NMLT is not assumed.

**Purpose:** evaluate exact-target handling, useful diagnostics, recovery, and
proof reuse. R0's three fixed proof strategies are tutorial material, not the
held-out task.

**Later evaluation packet:** a pinned small project, a reviewed target, a
failed candidate, an interruptible attempt, and a second lemma that can reuse
the first. Freeze the target and dependencies before the session.

**Actions:**

1. Run the initial attempt and explain its rejection.
2. Complete a valid proof of the same target without weakening assumptions.
3. Interrupt and resume work; identify what was reused and rechecked.
4. Export ordinary Lean source, check it in the pinned environment, and reuse
   the lemma in the second task.

**Success evidence:** exact-target agreement, accepted Lean artifacts with
policy/dependency information, explicit failure/unknown outcomes, and an
explanation of remaining trust boundaries. A completed scheduler entry alone
does not pass. The evaluation must record maintainability and reuse effort,
not merely whether an agent produced a large proof file.

## Task card B: mathematical explorer

**Role:** someone comfortable evaluating a mathematical claim, assumptions,
and a counterexample. Familiarity with Lean may vary and must be recorded as
context for the observations.

**Purpose:** help formulate and refine a worthwhile question, inspect negative
evidence, and obtain a reusable formal result. The R0 addition example is
calibration; the later packet must concern an external domain or lemma family
that a mathematical reviewer considers useful.

**Later evaluation packet:** a fixed finite construction or modest lemma
family with known calibration results, a precise candidate that can be
refuted, and a meaningful surviving claim. An expert-selected open instance
is optional and is never required for successful completion.

**Actions:**

1. State the question and explain its assumptions in ordinary mathematical
   language; inspect the formal version for agreement.
2. Run the experiment and examine a counterexample to the exact claim.
3. Revise the statement explicitly and explain why the old evidence does or
   does not transfer.
4. Inspect a checked result and reuse its lemma, construction, or
   counterexample in a follow-up question.
5. Explain what is known, what is still a conjecture, and what prior-art
   review would be necessary for a novelty claim.

**Success evidence:** an understandable and reusable checked output, which
may be a refutation or known result. Finite instances cannot stand in for a
universal proof. Newness to a run, new formalization, and mathematical novelty
must remain separate labels. Do not reward a large count of trivial lemmas or
promise new mathematics within the session.

## Task card C: software engineer

**Role:** someone able to run and modify a small local Rust/Python worker or
state-machine program. Formal-methods expertise is not required.

**Purpose:** connect resource semantics to real local execution and diagnose
differences between the model and worker behavior.

**Later evaluation packet:** an exclusive-permit worker, fixed success/failure
inputs, bounded cancellation and late-response scenarios, and a user-written
invariant over a declared finite abstraction. The intended accepted inputs,
environment traces, and required outcomes must be specified before evaluation.

**Actions:**

1. Change an ordinary input, transfer the permit, run work, and collect its
   output in a later action.
2. Cause the specified typed failure and explain the spent resource count.
3. Cancel an attempt and inspect a late response after slot reuse.
4. Run the invariant/trace check, inspect a genuine failing path in the
   negative control, and explain the exact finite bounds.
5. Reuse the worker component in a second workflow and identify which
   property transfers and which assumptions must be checked again.

**Success evidence:** all required outcomes within the declared scenario
bounds, correct ownership/accounting, a useful diagnostic path, and a clear
model/runtime distinction. An always-failing worker, skipped input, or added
deadlock cannot pass as a cheaper implementation. Host correctness or
exactly-once external execution cannot be inferred from the finite model.

## Observation form

Copy this form for each future session. Leave fields blank until observed;
do not populate them with assumed users or predicted results.

| Field | Record |
|---|---|
| Role and relevant prior experience | |
| Task packet/version and calibration versus evaluation split | |
| Tool condition and condition order | |
| Host, dependency/checker versions, warm-up/cache policy | |
| Start/end times and interruptions | |
| Setup minutes / hands-on task minutes / waiting minutes | |
| Completed actions and required outcomes missed | |
| Help requests and interventions, with reason | |
| Failure diagnosis: first interpretation and eventual cause | |
| Model configuration/budget and observed use, if permitted | |
| Artifact paths, exact target/context, and checking result | |
| Resource reservations, dispatches, settlement, unavailable measurements | |
| Resume/replay behavior and work repeated | |
| Second-task reuse: what transferred and what needed rechecking | |
| User's explanation of evidence scope and uncertainty | |
| Misleading terms, cumbersome steps, missing capability | |
| Useful output and why it mattered to this task | |
| Remaining blockers or unfinished outcomes | |

Ask the following open questions after the work:

1. What did the tool establish, and what remains uncertain?
2. Which step required the most explanation or manual repair?
3. What would you need to change to use the result in another project?
4. Which baseline step became easier or harder with NMLT, and why?
5. Which single change would make this workflow worth using again?

## Analysis and decision rule

First check task correctness and evidence interpretation. Report failures,
timeouts, rejected candidates, and interventions along with successes. Keep
waiting time separate from hands-on work; do not claim model or runtime speed
improvement from a change in human setup time.

The practical plan proposes a 20% reduction in median hands-on task time as a
possible meaningful advantage. Freeze an appropriate threshold and comparison
design before the actual evaluation. A small pilot can reveal usability
problems and useful mechanisms; it cannot establish that numerical threshold
with strong statistical confidence or validate market demand.

Require a useful complete workflow for each audience before claiming an
all-three alpha. A useful result may be a checked counterexample, known lemma,
or working local protocol; it need not be a new theorem. If one audience's
workflow is unfinished, report its actual readiness separately. If a small
library is as effective as new syntax, retain the library and simplify the
language rather than adding surface features to improve the demonstration.

R0 completes this preparation by supplying the cards, contracts, and form.
Executing this later pilot and collecting actual user evidence remain future
R5 work, not a requirement to identify specific people now.
