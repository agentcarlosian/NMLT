# R5 solo validation and optional qualitative pilot

Current completion policy (2026-10-03): the required track is the executable
[solo validation protocol](solo-validation.md). One maintainer exercises all
three roles; external participants, live models and a human-time improvement
threshold are optional study work. Kernel, NanoDA, fresh rechecks and specified
negative controls remain mandatory. No independent human study has been run.

The [manifest](../examples/evaluation/manifest.json) and its canonical
[digest](../examples/evaluation/manifest.sha256) freeze 30 Lean task cards,
12 finite behavior scenarios with 12 invalid variants, and three discovery
cards. These are separate from the unchanged R0 corpus. They are authored,
inspected, and solved by the implementing assistant. Public reference solutions,
standard elementary lemmas, and the shared composition calibration prevent
claims of a blind model benchmark or uncontaminated theorem discovery.

## What the engineering checks establish

The Lean tasks have six cards each for ordinary statements, dependencies,
repair, context/revision, and interrupted runs. Their mathematical references
live in [Corpus.lean](../mechanization/lean/NMLT/Evaluation/Corpus.lean).
`make r5-evaluation` checks all references and rejects a type-invalid candidate
for each. These are **30 mathematical preflights, not 30 operational
repair/revision/interruption trials**. Those activities require actual retained
operational runs in the solo track or an optional participant study.

The 12 scenarios exercise one new finite dispatch model: completion, monitoring,
retry, three cancellation points, return/reclamation, and cached result reuse.
Each path is accepted and freshly rechecked by the existing Lean executable;
its invalid variant must fail at the specified boundary. Frozen endpoint and
declared-work expectations are additionally checked by an independent Python
index decoder. Valid paths do not prove all possible executions safe, and
declared work is not host CPU usage. The existing source/core correspondence
limits remain unchanged.

The three discovery cards are:

1. Explain the abstract-formation composition result and its policy boundary.
   This deliberately shares the R4 theorem and is explanatory calibration,
   not a held-out discovery claim.
2. Recheck the two surviving worker policies on three generations and delivery
   words of length 0–4: 9,801 fixed cases per policy. No tuning after this input
   freeze is part of the reported comparison.
3. Check all 15 one-step circulant candidates at sizes 8 and 9. The corresponding
   [Lean object theorem](../mechanization/lean/NMLT/Evaluation/Discovery.lean)
   and Python/Lean parity are separate from the computational search. These
   remain known cycles, not a proof for all sizes.

## Matched recorded replay

`make r5-replay` uses the existing R3 adapter with the same bound task,
two recorded candidates, axiom policy, Lean binary, exporter, NanoDA binary,
candidate order, and fresh-recheck policy in both arms. The library arm calls
`lean-task prove` directly from Python. The NMLT arm uses the source workflow
in [replay.nmlt](../examples/evaluation/replay.nmlt) with the same adapter.
The controlled difference is the orchestration surface. Neither arm calls a
model, retrieves extra context, or reuses a previously accepted proof result.

One trial unit is a single batch theorem whose checked dependency closure
contains all 30 reference solutions. It is not counted as 30 independent task
runs. Two repeats reverse arm order (library/NMLT, then NMLT/library). Expected
totals are eight candidates, four rejected invalid terms, four accepted batch
proofs, and four fresh native rechecks. The source-arm records must show the
exact ordered proof requests and real proof-rejection diagnostics; timeout or
tool failure cannot count as rejection. Accepted task/candidate identity,
receipts, exports, and fresh records are checked before a success is counted.

The adapters reconstruct their proof contexts, but operating-system caches and
the shared tool installation are not experimentally controlled. Command wall
time is retained for diagnosis, not used to claim a language speedup or reduced
hands-on time. Live paired trials, model quality, cost, and human usefulness
are unmeasured. A different model, budget, checker policy, prompt, retrieval
setup, or cache treatment requires a new comparison contract for both arms.

## Optional participant study

Recruitment/contact is optional and outside this increment. If this separate
study is undertaken, a coordinator must recruit at
least one independent person in each audience, obtain consent, establish actual
independence, and retain the observed artifacts. One person cannot fill multiple
independent audience slots. The implementing assistant is not a participant.

| Audience | Install, modify, run, and explain |
|---|---|
| Mathematician | Install the pinned tools; reproduce the composition and graph controls; modify an input in a new corpus revision; identify a checked counterexample and explain why a known cycle is not novel. |
| Lean user | Materialize an ordinary/dependency card, bind it, submit a wrong then corrected candidate, and freshly recheck; repeat a context change with an explicit revision; interrupt a separate run and explain retained evidence and unresolved effects. |
| Agent-system engineer | Run a finite scenario and invalid control; change a cancellation/retry input in a new revision; run/replay/recover a native source proof session; explain charges, generations, and the distinction between bounded and host behavior. |

For an ordinary local project containing the selected statement and definitions,
but no reference proofs:

```bash
python tools/experiments/materialize_task.py --task ordinary01 --output target/pilot-ordinary01
```

Bind and prove it through the existing [R3 task guide](r3-lean-tasks.md).
The project starts unproved. `by sorry` in its task statement is not acceptance;
the existing transitive axiom policy rejects candidates depending on it.
The public reference solutions remain exposure, even though they are not copied
into the materialized project.

Use both arms on the same selected cards with the frozen candidate budget and
checking policy, and use at least two paired repetitions with reversed order.
Record `arm_position` as 1 or 2 within each pair. The summarizer requires both
orders for every participant/task before reporting `paired_protocol_complete`
or a met descriptive target. A single repetition or repeated identical order
retains descriptive statistics but records the incomplete protocol.
Record setup, hands-on, diagnosis, proof-maintenance, CPU, peak memory, token
counts, acceptance/recheck outcomes, negative-control denominators, and whether
the participant explains reuse. Include failures and interrupted attempts;
do not remove unsuccessful runs from time or quality denominators. For live
model runs, freeze model/version, prompts, retrieval, caches, stop budget,
environment, and response hashes before running either arm.

## Optional study decision rule and uncertainty

Before the first participant trial, the proposed descriptive target is at least
20% lower median hands-on time without worse acceptance quality. Compute paired
reductions within each participant first, then the median across participants;
repeated runs are not new users. All required results must recheck, every
specified invalid control must be rejected, and participants must complete and
explain their workflows. Baseline time zero leaves the ratio undefined rather
than manufacturing an improvement.

Start from the empty [observation template](../examples/evaluation/pilot-observations.csv).
The summarizer validates paired observations and reports sample size, audience
coverage, quality counts, and the observed participant range:

```bash
python tools/experiments/summarize_pilot.py examples/evaluation/pilot-observations.csv --output target/pilot-summary.json
```

The empty template produces zero participants and no effect estimate. The
range is descriptive, not a confidence interval. Three people would still be
a qualitative pilot; no population-level usefulness claim follows. Manual CSV
entries are not authenticated proof receipts. A coordinator must match them to
actual retained checker artifacts and judge explanations/independence.

The corpus and recorded replay are engineering preparation. The required R5
exit gate is now the distinct solo track: executed role walkthroughs, second
uses, repair/revision/interruption/recovery, controls, retained checker evidence,
complete platform verification and a written local value/limits assessment.
Neither engineering preflights nor a change of policy completes that gate.
If local ceremony outweighs the benefit, retain useful libraries, simplify the
language surface and repeat the focused comparison. No participant results are
inferred from maintainer or AI-assisted runs; this optional human-study schema
retains its independent participant slots and descriptive target.
