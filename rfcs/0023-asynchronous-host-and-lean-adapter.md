# RFC 0023: Asynchronous host controls and the first Lean adapter

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-07
- Tracking issue: not assigned
- Design disposition: initial executable-only R2 host API; acceptance pending

## Problem and non-goals

RFC 0022 connects synchronous source effects to one worker. Start the next
increment with a real asynchronous host API and a bounded Lean adapter, so
polling, cancellation, settlement, and recovery behavior can be tested before
source syntax exposes transferable job controls.

This increment does not add `.nmlt` asynchronous handles, a general Lean source
loader, arbitrary proof text, mathlib integration, a new proof kernel, dependency
locks, process-tree containment, or automatic workflow resumption. Source
`job_square` retains its existing route. Source handle typing and a broader Lean
interface are subsequent work. The accepted disposition remains executable-only.

## Asynchronous process supervision

`process::Process` starts a trusted caller-configured command without a shell.
It returns after spawning the direct child and its supervisor; the caller need
not block until completion. The supervisor drains stdout/stderr and writes stdin
on separate bounded tasks. Stdin and each output pipe are capped at 64 KiB.
The process timeout is positive and at most 30 seconds; session limits use
1–30,000 integer milliseconds. It runs independently of polling. A job
that finishes in time remains complete when observed later. Output, exit status,
and both pipes must finish before a completed observation becomes available.

`poll` is nonblocking, `wait` waits for a bounded terminal observation, and
`cancel` signals the supervisor then waits for cleanup evidence. Child termination
uses kill plus bounded reaping polls for up to one second. Wrapper waits allow
an additional bounded margin and report uncertain I/O/cleanup when that margin
expires. The same result is retained on repeated observation. Dropping a process
requests best-effort termination; it is not evidence of successful cleanup.
The supervisor owns a child guard so failure unwinding also attempts termination.

Nonzero child exit is observable data, since Lean rejection is useful. Timeouts,
output limits, failed spawn/I/O, and cancellation carry explicit failure kinds
and a `child_reaped` observation. A completed output implies confirmed direct
child exit; an absent exit code can represent signal termination. These bounds
depend on OS scheduling and do not impose an OS CPU/memory quota or contain
descendants. The supplied worker/templates do not spawn children.

## Durable session and handle contract

`session::Session` combines the supervisor with the existing locked journal.
Creation requires a new directory. A canonical `manifest.json` binds a parent
context digest, runtime bounds, worker executable digest, and optional Lean
identity. Its hash becomes the journal context; the manifest is synced before
journal creation. Exact runtime executable identity remains in the journal.

Sessions allow 1–4 slots, 1–16 attempts, and a 1–30,000 ms timeout. The slot count
cannot exceed the attempt bound. Each attempt reserves one work unit, charged
at durable dispatch. The event bound is `6 * max_attempts + 1`; generations are
bounded by total attempts. Slot reuse requires terminal collection. These work
units are declared reservations, not wall time or measured resource expenditure.

`start_square` and `start_lean` validate tool identity and request construction,
reserve, then durably dispatch before spawning. They return a private `Handle`.
Handles implement neither `Clone` nor serialization and cannot be constructed
by callers. A per-session identity also rejects handles from a different live
session even when their run identifiers and contexts happen to match. The
session keeps revisioned journal controls private.

`poll` and `wait` return `Pending`, `Ready`, `Uncertain`, or `Collected`. A ready
process observation is validated against its adapter before settlement. Repeated
pending polls do not append journal events. `collect(&mut handle)` returns no
value for pending work; after settlement it durably collects exactly once and
returns an ordinary reusable outcome. Repeated collection fails. Uncertain work
cannot be collected or free its slot.

`cancel(&mut handle)` first polls. An already settled result wins and returns
false. Otherwise it durably records cancellation before signalling the child.
Confirmed direct-child cleanup acknowledges cancellation, including completion
that arrives after that durable request; the late result is not promoted.
Cancellation returns true once settled. Unconfirmed cleanup leaves an uncertain
cancel-requested attempt and returns an error. Prior charges remain and observed
work stays unknown. Collection is still required to free a cancelled slot.

Lifecycle precondition failures, such as a full slot pool, leave the session
usable. Errors during persistence stop live session authority and request
best-effort cleanup of its processes. The existing journal poisoning and torn-log
rejection rules apply. Session drop also requests cleanup by dropping process
handles, but does not claim durable cancellation acknowledgements.

## First Lean adapter

`lean::Toolchain::open` identifies a regular executable by SHA-256 and probes its
version with a bounded process. Require the repository pin, currently Lean
4.33.1. Recheck executable bytes before dispatch. Record the complete reported
version and a digest of the adapter contract/templates. Metadata probing precedes
allocation and is not charged as a proof attempt.

The adapter accepts a `Strategy` for the fixed R0 calibration target
`forall n : Nat, 0 + n = n`. Available templates are `WrongTerm`, `ExistingLemma`,
`Induction`, and the admission negative control `Admitted`. It generates the
entire source internally, imports `Init`, disables automatic implicit variables,
sets 200,000 heartbeats, defines `NMLTJob.checked_target`, and requests its
transitive axiom report. No caller-supplied Lean syntax is interpolated.

Run the executable directly with `--stdin --threads=1 --memory=512`. Clear the
inherited environment except necessary Windows OS/temp variables and use the
executable's own directory, so caller `LEAN_PATH` and project cwd do not select
imports. These are requested Lean limits, not OS containment. The local standard
library and dynamic libraries remain trusted and are not fully fingerprinted;
this is not a closed toolchain lock or protection against concurrent replacement.

An input is canonical text encoding of the Lean request, which includes exact
tool identity, strategy, and generated-source digest. The runtime binding also
identifies run, task, generation, owner, and parent context. The adapter returns
`Text` containing the submitted-source digest only when an eligible positive
template exits successfully, stderr is empty, and stdout is exactly the empty
transitive-axiom report for the named target, allowing surrounding whitespace.
The two negative templates cannot be promoted by a purported empty report.

Exit 1 with Lean error diagnostics records candidate rejection; this is not a
refutation of the target and may reflect Lean's own resource failure. Zero exit
without the exact allowed report fails the output/axiom policy. Unsupported
exits, invalid encoding, external timeout, or pipe failure remain uncertain host
outcomes. Lean observed work is `None`; the declaration of one work unit does
not invent a measurement. The source digest is reusable evidence identification,
not a `.olean` artifact, a trusted source-language proof value, or independent
kernel-checking evidence.

## Snapshots and recovery

A `Snapshot` contains its manifest, exact journal bytes, and ordered process
observations. `verify_snapshot` reconstructs the lifecycle and validates each
configured dispatch and corresponding settlement/timeout. Square outputs must
match the deterministic adapter and exact response encoding. Lean observations
must match the canonical request, generated source, configured checker identity,
and verdict policy. Missing, surplus, altered, misbound, or out-of-order
observations fail. The API performs no process launch or journal mutation.

This is captured-record consistency, not replay of arbitrary Rust host code,
fresh Lean checking, or authenticated physical provenance. A completely forged,
internally consistent host observation is not excluded. The example's serialized
snapshot is capped at 16 MiB and must retain its exact pretty-JSON encoding.

`session::recover` validates the bounded canonical manifest and opens the journal
against the exact original runtime executable/context. It returns classified
lifecycle state only; handles and live processes are never reconstructed.
Pending attempts become uncertain, and torn logs fail. It does not locate or
kill surviving workers, redispatch, reconcile unknown results, or resume code.
Copies/rollback and parent-directory crash durability retain RFC 0018's limits.

## Examples, validation, and theorem consequences

The [async_jobs example](../crates/nmlt-runtime/examples/async_jobs.rs) launches
two jobs before waiting, handles wrong proof/input, collects and reuses a square,
requests cancellation, reuses slots, rejects admission, and obtains lemma and
induction receipts. The actual cancellation race is recorded rather than forced
to claim that cancellation always wins.

Tests cover independent deadlines, late polling, pipe limits, cancellation and
cleanup, full pools, exhausted attempts, cross-session handles, stale collection,
uncertain charges, partial journal writes, snapshot mutations, source/identity
binding, exact axiom reports, and negative-control promotion. `make r2-async`
joins the Rust gate; `make r2-lean` exercises the actual pinned Lean executable
and joins reproduction and the Lean CI job. The complete retained Rust/Lean/
NanoDA/parity/baseline gate remains required.

No Lean definitions, theorem statements, artifact schemas, source grammar, or
existing source evaluator semantics change. Runtime code and the configured
Lean installation form an explicit executable trust boundary. Independent
publication review and RFC acceptance remain open. Next work is source-level
affine handle checking, a general supported Lean request interface, dependency
identity, and reconciliation/resumption policy.
