# RFC 0018: Bounded local job lifecycle and recovery

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-06
- Tracking issue: not assigned
- Design disposition: executable-only R2 runtime increment; acceptance pending

## Problem and scope

Before source-level jobs can use the [workflow profile](0014-executable-workflow-profile.md),
the runtime needs an exact contract for identity reuse, control ownership,
settlement, cancellation, and incomplete external work after restart. R0's
frozen single-slot Python baseline is comparison evidence, not that implementation.

Introduce `nmlt-runtime`: a pure bounded lifecycle, a locked append-only local
journal, a typed response protocol, and a subprocess worker demonstration.
This does not change `.nmlt` syntax, `behavior-core-v1/v2`, the finite interpreter,
or Lean definitions. All facilities here are **executable-only**. Typed envelopes
and journal replay provide no Lean acceptance, proof, or external authorization.

## Identities, values, and bounds

`RunSpec` binds a caller-supplied run ID, context SHA-256, and `Limits`. The caller
must assign a distinct run ID to each independent run; the library has no global
run registry. The context identifies the intended workflow/toolchain/definition
environment supplied by the caller; it does not verify that environment.

An attempt ID is `(run, task, slot, generation)`. Generations start at one and
increase on every allocation, including allocations cancelled before dispatch.
The lowest free slot with an unused generation is selected. A finished attempt
holds its slot until collection. Exhaustion returns an error before mutation;
there is no counter wrap, implicit eviction, or automatic retry.

| Bound | Supported range |
|---|---|
| Slots | 1–64 |
| Generations per slot | 1–10,000 |
| Total allocated attempts | 1–10,000, including cancelled undispatched attempts |
| Committed events | 1–100,000, including ignored responses and recovery |
| Reserved work budget | Positive `u64` |
| Protocol text/failure messages | At most 4,096 bytes |
| Journal entry / entire journal | 65,536 bytes / 64 MiB |

Names are bounded ASCII identifiers. Protocol version one supports tagged Bool,
Int (`i64`), and Text values. These are Rust adapter values, not new finite-core
or `.nmlt` types. An adapter descriptor binds its name, version one, input type,
and output type. The lifecycle checks types; an actual adapter must additionally
be registered, enforce its specific contract, and validate its outputs.

Requests carry the exact input, matching run context, and a positive work
reservation. A dispatch binding fixes the attempt, adapter descriptor, dispatch
owner, context, and input SHA-256. Responses must match all these fields. Future
protocol/adapter versions fail explicitly. A stale or mismatched response is
recorded as ignored without changing job control, slot occupation, or accounting.

## Control and transitions

Every control handle names an attempt, owner, and revision. Each accepted control
transition and terminal settlement increments the revision. Copies of an older
handle fail even if ownership returns to the same owner. Ownership is transferable
only while reserved. Dispatch consumes the reserved revision; cancellation and
collection use subsequent revisions. This is runtime affinity, not Rust's static
linearity, authentication of an OS principal, or a secret capability.

The application calls this API through a trusted local coordinator. It must not
let untrusted adapter messages invoke owner commands or choose reconciliation
policy. Adapter responses enter only through the response path.

| Operation | Preconditions | Result |
|---|---|---|
| Reserve | Free slot/generation, allocation/event limits, matching context and valid request, sufficient uncommitted budget | Reserved control and held work reservation |
| Transfer | Exact current control, Reserved, different valid owner | New owner and revision |
| Dispatch | Exact current Reserved control | Running; full reservation becomes permanently charged; bound dispatch receipt |
| Cancel before dispatch | Exact Reserved control | Finished Cancelled; unused work reservation returned; identity remains spent |
| Cancel after dispatch | Running or uncancelled Uncertain, exact control | CancelRequested or cancellation-marked Uncertain; charge and slot retained |
| Timeout | Running or CancelRequested, exact control | Uncertain, retaining cancellation intent and charge |
| Deliver completion/failure | Exact dispatch binding, Running, correct output type | Finished observed outcome; adapter-reported usage retained |
| Deliver cancellation acknowledgement | Exact binding, CancelRequested | Finished Cancelled; charge retained |
| Deliver late completion/failure after cancellation | CancelRequested | Ignored for acceptance; response retained in the log for reconciliation |
| Deliver while Uncertain | Exact binding | Ignored; explicit reconciliation required |
| Reconcile | Exact control and binding, Uncertain or CancelRequested; coordinator has adapter evidence of termination | Finished terminal result; cancellation intent always yields Cancelled, even if the response reports completion |
| Collect | Exact current control, Finished | Collected; slot released; persistent outcome returned |
| Recover | Valid replayed journal | Running/CancelRequested become Uncertain; every uncollected control gets a new revision |

Unsolicited cancellation acknowledgements and ill-typed current results are
errors. Reconciliation does not manufacture evidence of termination; the caller
must obtain it under the adapter's documented contract. A late completed value
used to settle cancelled work is retained in the event but is never returned as
that attempt's successful result. Repeated terminal delivery or collection cannot
create another accepted result or release a newer reservation.

## Accounting and reusable results

Accounting derives allocations, dispatches, held reservations, permanently
charged work, and available work from attempt history. Held plus charged work
never exceeds the configured reservation budget. Dispatch moves the entire
reservation into the charged ledger, regardless of later failure, cancellation,
timeout, or restart. Only cancellation before dispatch refunds work. Allocation
and generation budgets are never refunded.

`observed_work: Option<u64>` is per-attempt adapter-reported usage. Missing usage
stays unknown. Reports larger than the reservation remain visible; the lifecycle
does not silently clamp them or pretend that reservation enforcement bounds host
CPU/time. Model grades, reservation accounting, and observed expenditure remain
different quantities. Actual host resource enforcement is future adapter work.

Collecting consumes the current job control and releases the slot. The returned
Completed/Failed/Cancelled outcome is an immutable value. Copying a completed
value does not create job control. The runtime calls it an observed outcome,
not a proof, checked theorem, or approval for a later external action.

## Journal protocol and crash boundaries

`Journal::create` creates a new file and obtains an exclusive OS file lock.
`Journal::open` locks the existing file before reading it. The lock is held for
the handle's lifetime; contention fails promptly. This uses Rust's
[file locking and synchronization APIs](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock),
and the new crate declares Rust 1.94, matching the repository's tested toolchain.

On Unix, journals must have exactly one filesystem link. Creation, opening,
and every commit check this condition; a new hardlink stops further commits.
This guard follows an observed WSL/Windows-mount failure to coordinate locks
across hardlinks sharing an inode. Native Windows uses its tested OS locking
for aliases. Filesystem assumptions are platform-specific, not inferred from
the API name alone.

The header binds `nmlt-job-journal-v1`, exact executable SHA-256, and the expected
run specification. Canonical JSONL entries carry contiguous sequence numbers,
the previous line's hash, the command, its derived receipt, and a hash of the
complete reconstructed lifecycle. Unknown fields, duplicate fields, changed
encoding, wrong identities, invalid transitions, altered receipts/state hashes,
reordering, and incomplete final entries fail validation.

For each command: derive the next state; serialize the entry; write it; call
`sync_all`; only then publish the new in-memory state and receipt. Host work
starts only after a successful journal Dispatch receipt. A write/sync failure
returns no receipt and poisons the handle. The handle cannot execute more
commands. If the complete entry survived, reopening conservatively classifies
dispatch as uncertain. If only part survived, opening fails and requires external
reconciliation; this increment never truncates the tail automatically.

Open requires the exact caller-supplied RunSpec and executable. It replays all
commands and compares derived receipts/state hashes. It then commits recovery
before exposing unfinished control. Completed values and charges survive;
reserved jobs stay reserved, while possible in-flight work stays uncertain.
No command or host adapter is automatically redispatched during replay/recovery.
Event or byte exhaustion can prevent further operations, including recovery;
the operator must retain and reconcile the log instead of resetting its budget.

This contract assumes one cooperating local filesystem and an intact journal
history. The tests cover process death, not arbitrary power-loss persistence.
Do not unlink/replace the locked file or run from an older snapshot/copy. A hash
chain cannot detect deletion of an entire valid suffix without an external head
anchor, nor authenticate a consistently rewritten log. File locks do not fence
copies, separate kernels, distributed writers, or arbitrary host processes.
`Lifecycle::step` is a pure testing model whose clones are not durable authority.
This design makes no exactly-once external execution claim.

## Adapter example and source disposition

The `local_worker` example reserves and transfers control, commits dispatch,
starts a copy of its own executable over stdin/stdout pipes, validates the typed
response and exact square-result contract, delivers, and collects. Negative
inputs yield a Failed outcome; the next supplied input uses the next generation.
A successful value can be reused after collection. Spawn/transport/validation
failure records Uncertain and does not blindly retry that attempt.

The sample is a blocking, bounded arithmetic subprocess, not a scheduler for
untrusted generated code. It does not implement host cancellation delivery,
process-tree isolation, general process timeouts, a Lean adapter, or resume CLI.
Those remain integration work. `nmlt run`/`replay` retain RFC 0017's finite-only
meaning, and do not read job journals. Native `.nmlt` job syntax, lowering, and
the relationship to finite resource semantics require another reviewed increment.

## Research delta and evidence plan

An archive search identified Khan's *Resume Means Resume* (2026-08-04; archive
first seen 2026-08-06). Its primary text separates durable-state recovery,
consume-once control, and external effects. I use that distinction as design
guidance for dispatch-before-effect, explicit uncertainty, and process-kill
controls; none of the paper's proofs or framework measurements is evidence about
NMLT. The focused refinement found no second directly useful source.
[Primary paper](https://arxiv.org/html/2608.03836v1).

Required implementation controls include owner round-trip/old revision,
early/repeated collection, all response binding dimensions, cancel then late
completion, slot reuse, unknown usage, undispatched-only refund, generation and
budget exhaustion, replay of every phase, locks across processes and aliases,
write/sync failures, malformed/torn logs, and real process kills after dispatch
and after worker output survives without a journal completion record.
A frozen Rust regression alphabet explores two slots and four events, checking
ownership, identity, accounting, and persistent-result invariants. This is an
implementation test, not a formal model-check certificate.

RFC acceptance, independent publication review, source/effect integration, and
R2's complete exit gate remain separate. The journal and typed protocol are not
accepted behavioral artifacts and add no Lean theorem claim.

Executed results and the filesystem-specific locking correction are recorded in
the [second-increment evidence](../docs/reviews/r2-job-lifecycle-increment-2026-09-06.md).
