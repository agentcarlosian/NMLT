# RFC 0022: Source-level bounded local job effects

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-07
- Tracking issue: not assigned
- Design disposition: executable-only R2 increment; acceptance pending

## Problem and non-goals

Connect RFC 0018's durable job protocol to the source language introduced in
RFCs 0019–0021. A source program must accept an input, obtain a typed subprocess
result, choose a bounded fallback, and reuse the collected value. The first
adapter is the existing deterministic `local-square` version 1 worker.

This does not add arbitrary commands, a shell, remote tools, a Lean adapter,
asynchronous source job handles, cancellation syntax, automatic retry/resume,
process-tree isolation, or a theorem about host execution. Existing finite
artifacts, shared frontend rules, M9, and Lean semantics remain unchanged.

## Static semantics and lowering

Reserve `job_square` as a built-in identifier. The existing call expression
projection admits `job_square(input)`; lower it to a dedicated private typed
node after requiring exactly one `Int` argument. Its result is `Outcome<Int>`.
It is not a source-defined function or a configurable adapter name.

Check every function and branch, including unused imported code. Mark a
function effectful if it contains this node or calls an effectful function.
Propagate through the existing acyclic whole-package call graph. The summary
is conservative across `if`, `match`, Boolean short-circuiting, and empty folds.
Unreachable effects still require opt-in; an unrelated pure entry remains pure.
Store the summary in the sealed program identity. No job control enters a
workflow `Value`, so records, lists, and ordinary reuse cannot copy control.

`Program::requires_jobs` exposes the summary. `execute` rejects effectful entries
before evaluation. `execute_with_host` requires an explicit `JobHost`; embedding
hosts own durability, response validation, and budgets. The trait exposes only
an integer input, its source location, a typed integer/domain-error result, or a
host stop. `validate_inputs` supports preflight before creating host state.

## Dynamic semantics and bounds

One typed expression step enters the job node. Evaluate its argument first,
using the existing step/depth/value limits, then ask the host once. A successful
worker result yields `Ok(value)`; negative input or square overflow yields the
worker's bounded `Err(message)`. Matching can choose another job. Reusing the
returned integer launches no work. Selected-branch and fold evaluation order
is unchanged; effects occur in the evaluator's existing left-to-right order.

The CLI requires all of `--jobs-dir`, `--max-jobs`, and `--job-timeout-ms`, plus
the existing entry/input/step/output options. Bounds are 1–16 total attempts,
one live slot, 1–30,000 ms per worker, and the existing 1–100,000 expression
steps. Each attempt reserves one work unit, charged durably on dispatch, with
at most `6 * max_jobs + 1` journal events. This unit is adapter accounting, not
CPU seconds or measured expenditure. Collection frees the slot, not the charge.

Validate the package, all inputs, execution limits, adapter identity, and a
1 MiB compact context limit before creating files. Require a new output file
and a new jobs directory. Persist and sync `context.json` before creating the
locked `journal.jsonl`; journal dispatch intent is synced before spawning.
The context binds exact executable bytes, the complete source manifest, typed
program, entry, inputs, limits, and adapter. A run identifier separates attempts.

Each job reserves and dispatches under one internal owner. The fixed current
executable's private worker command consumes a bounded JSON dispatch and emits
a response. Source text cannot choose a command or arguments. The supervisor
uses concurrent pipe I/O, accepts at most 64 KiB on each output pipe, bounds
stdin to 64 KiB, and checks elapsed time. Unexpected stderr, invalid/excess
output, nonzero exit, timeout, or I/O failure stops the host. It requests direct
child termination and polls for reaping for up to one additional second.
`child_reaped` reports whether no child started or direct-child exit was
confirmed. Memory/CPU, descendants, executable replacement races, OS scheduling,
and filesystem stalls are outside these process limits. The fixed worker does
not spawn descendants; bounded pipe tasks are not joined indefinitely after
failed cleanup. This is a cooperating local host contract, not isolation.

Validate exact response shape, binding, adapter, input hash, result, and observed
work against the deterministic worker before delivery. Settle and collect
through the journal before returning a domain value. A limit stops before a new
reservation. A host failure uses `JobStopped { reason: host_failure }`, writes
`Timeout` to retain an uncertain attempt, and keeps the charge and unknown
observed work. It never enters the source's `Err` branch. Journal I/O failure
aborts without a completed run record; existing poisoning rules apply.

## Records, replay, and recovery

Job runs use `nmlt-local-job-run-v1` / `local-job-workflow-v1`, with
`assurance: none`. The record contains its manifest, ordered source/input/response
or host-failure events, exact canonical journal bytes, final accounting, and
execution stop. The serialized record is capped at 4 MiB. The locked handle
captures the journal bytes and validates its own state, tail, and length; it
does not reopen a possibly replaced pathname.

Replay requires the exact executable and complete source closure, compiles
again, checks the context/limits, and validates the embedded journal using
`replay_journal`. This API reconstructs the pure lifecycle and commands without
locking, recovering, or issuing live dispatch authority. The evaluator consumes
recorded events at exact source locations and inputs. Its reconstructed commands,
receipts, final state, accounting, steps, and stop must match, with no unused or
missing events. Typed responses are independently recomputed by the deterministic
adapter validator. Duplicate/unknown JSON fields and noncanonical journal lines
fail. No process is launched and no live journal is changed during replay.

Host failures and cleanup flags are recorded observations, not authenticated
physical facts. Consistency checking is not provenance authentication; a fully
rewritten, internally consistent record is not ruled out. Pure run/replay stays
on version 3 and remains effect-free. Workflow typecheck output moves to version
4 and reports `requires_jobs` for each entry. Older runs require their original
executable; all executable changes already invalidate exact-binary replay.

`jobs-recover <directory>` reads the bounded exact manifest, requires its original
executable, opens the locked journal against that context, and durably classifies
unfinished attempts using RFC 0018's `Recover`. It reports state only. It does
not find or kill surviving workers, redispatch, accept replacement results, or
resume the workflow. Torn/corrupt logs fail. Repeated recovery advances revisions
of unfinished attempts and consumes the finite event budget. Completed logs do
not change. File sync does not promise parent-directory crash durability or
protection from copied/rolled-back journals. Failed setup can leave an empty
output or partial directory; failed final writes leave the context and journal.

## Examples and negative controls

The [source example](../examples/pivot/job_fallback.nmlt) with input -3 and
fallback 5 dispatches two jobs, collects the square 25, and returns `Ok(50)`.
A one-job budget stops before the fallback. A timed-out host also stops before
fallback and retains uncertain charged work. `job_square(true)` fails typing;
an effect hidden behind `if false` still fails pure execution.

Tests cover conservative/transitive/imported effects, folds, type rejection,
real subprocess success/domain failure/overflow, value reuse, preflight path
and input rejection, step/job limits, both output pipes, blocked stdin, timeout
cleanup, injected uncertain failures, replay tampering, and recovery of complete
unfinished journal prefixes. Existing process-death journal tests remain.

## Theorem consequences and remaining questions

No new Lean interpretation, preservation theorem, verified translation,
authenticated worker result, or host scheduling guarantee is claimed. The
Rust host implementation joins the explicit trusted-component inventory for
these executable-only claims. Run the complete Rust/Lean/NanoDA/parity/baseline
gate to detect regressions in the retained semantics. RFC acceptance and
independent publication review remain open.

Subsequent work must specify supported Lean jobs, affine asynchronous source
controls, cancellation/reconciliation evidence, stronger host containment,
resumption policy, and the full real-input R2 acceptance workflow.
