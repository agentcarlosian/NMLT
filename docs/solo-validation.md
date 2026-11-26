# Solo-development validation

NMLT is developed by one maintainer. The required R4/R5 validation is feasible
without recruiting users or an outside mathematician. External feedback is
optional. Technical independent checking means Lean/kernel, NanoDA and fresh
artifact verification; these requirements remain in force. Maintainer and AI
critique are recorded as such, never as independent human evidence.

## Completion criteria

R4 requires known-result calibration, an exact checked generalization,
refutation or construction, an executed second use in a different example or
question, and an explanation of premises, local value, bounds and prior art.
Repeating cases 00–15 or rediscovering cycles is not by itself a second use or
a novelty result. Failed searches and unsupported conclusions remain unknown.

R5 requires an actual walkthrough for each role:

| Role | Required operations and evidence |
|---|---|
| Mathematician | Reproduce calibration; modify a question/input; check a result and invalid control; apply a checked result in a second example; explain assumptions and novelty/boundedness limits. |
| AI/Lean developer | Submit and diagnose an invalid candidate; repair it; revise context explicitly; independently recheck; interrupt and recover native work; reuse the workflow or checked result in a second task. |
| Software engineer | Change inputs; run bounded worker/proof workflows; exercise cancellation/failure; replay and recover retained work; reuse a component in a second workflow; explain charges, uncertainty and host/formal boundaries. |

The source/corpus preflights remain prerequisites. Each claimed success needs
the corresponding checker receipts and fresh validation. Each specified
negative needs the intended semantic rejection; I/O errors, timeout and failed
tool setup cannot stand in for mathematical or safety rejection. A complete
current platform gate, source/tool identities and retained commands/results are
required. One operator covering all roles is role coverage, not three users.

## Local value and comparisons

Before a run, freeze the task packet, expected operations, controls, environment,
budgets and measures. Record successful and failed attempts, setup, edits,
interventions, diagnosis, maintenance, reuse and resource usage where measured.
Distinguish manual observations from machine-recorded commands and timings;
unknown values stay unknown. Explain a concrete local benefit and any added
ceremony instead of requiring an arbitrary improvement percentage.

When comparing library and NMLT routes, hold adapters, candidates, orchestration
algorithm, prompts/retrieval (if used), cache treatment and checker policies
fixed. Use repeated matched pairs in both orders. Local timings do not establish
population speedup, independent-user usability or model performance. Paid/live
model calls are unnecessary; the current recorded-response route is permitted.

## Separate evidence tracks

Solo reports use `nmlt-solo-validation-v1` and retain role/operation receipts,
negative-control denominators, second uses, actual execution provenance and
explicit limitations. Empty evidence or missing roles, operations, rechecks,
controls or reuse leaves the gate incomplete. A report is executable evidence
indexing, not a new proof-acceptance format or authentication of human judgment.
The summary's `structural_index_complete` flag checks indexed identities, bytes
and specified controls. `operations_reviewed` remains false: reviewing actual
operations and local value is a separate required step. Dispatch/journal equality
records that no new dispatch was observed; it does not prove exactly-once physical
effects.

The optional human-study CSV and `nmlt-qualitative-pilot-summary-v1` stay separate.
Their independent participant slots, repeated reversed-order pairs and proposed
descriptive target are not reinterpreted as solo completion. Synthetic test rows
are not observations. No study is claimed without its actual participants and
reviewed records.

## Current status

Run `make r5-solo` for the frozen internal role walkthrough and `make reproduce`
for the complete required gate. The runner copies task inputs into a new ignored
`target/solo-*` directory, retains actual commands and diagnostics, repairs and
freshly checks proofs, explicitly revises the definition/target, changes worker
inputs, and interrupts/reconciles a native proof session. Its report indexes
evidence with `assurance: none`; it is not a proof-acceptance format or a
substitute for reviewing the actual operation results.

These are criteria to execute, not a declaration of completion. Existing R4/R5
engineering results are documented in [the verification record](r4-r5-verification.md).
Solo operational results and remaining gaps must be recorded after running the
new scenarios. The Windows lifecycle investigation retains its conservative
process policy and failed gate until a verified correction passes.
