# `nmlt-runtime`

Experimental R2 job control and recovery, with an executable-only claim ceiling.
Requires Rust 1.94, as used by the repository's pinned CI toolchain.

- `Lifecycle` is a pure bounded transition model for tests and inspection.
- `Journal` holds the local OS file lock, validates and replays exact records,
  and syncs state transitions before returning dispatch receipts.
- `Request`, `Binding`, `Response`, and `Control` bind types, input/context,
  attempts, owners, and control revisions.
- `worker` implements and validates the exact `local-square` adapter contract.
- `process` supervises bounded children asynchronously, with deadlines independent
  of polling and explicit cancellation/cleanup observations.
- `session` mediates start/poll/wait/cancel/collect through the journal using
  private session-specific handles; snapshots can be verified without launching work.
- `lean` checks fixed zero-addition proof templates with the pinned Lean version,
  executable/source identity, and an exact empty-axiom output policy.
- `Journal::snapshot` captures and checks durable bytes through the locked handle;
  `replay_journal` validates captured bytes without recovering or issuing authority.

The [source job guide](../../docs/r2-source-jobs.md) connects this runtime to
`.nmlt` `job_square` effects with a bounded supervisor and replay without dispatch.
The Rust example below remains the original adapter prototype.

The [Lean and async host guide](../../docs/r2-lean-async.md) describes the initial
Rust API and `async_jobs` example. Run `make r2-async` for the real worker controls
or `make r2-lean R2_LEAN_BIN=/absolute/path/to/lean` for the pinned Lean exercise.
Asynchronous source handles and a general Lean source interface remain next work.

Run a real subprocess attempt with failure and bounded fallback:

```bash
cargo run -p nmlt-runtime --example local_worker -- target/worker.jsonl -3 5
```

The first input fails; the next returns 25, and the output shows that value
reused twice after collection. Choose a new journal path for each run. Use
`make r2-jobs` (or `python tools/check_job_runtime.py`) to exercise failure/retry,
direct success, and all-failed cases. This gate is included in `make ci`.

The lifecycle keeps cancellation pending until acknowledgement or explicit
reconciliation. Late results cannot settle a newer attempt. Restart marks
possible in-flight work uncertain, retains charges, and revokes old control
revisions. Only undispatched reservations are refundable. Missing observed work
stays unknown. Collected values can be copied; job control cannot be reused.

The API intentionally has no snapshot deserializer. To recover, use
`Journal::open(path, &expected_run_spec)` with the same executable. Retain the
expected run specification separately as application configuration. Obtain
external completion evidence before issuing `Reconcile`; opening never starts
work. Unsupported, inconsistent, or torn journals fail closed.

Locks cover the same local file among cooperating processes. Journal copies,
history rollback, file replacement, OS-principal authentication, power-loss
guarantees, and exactly-once external execution are outside this contract.
The pure `Lifecycle` can be cloned for tests and must not be used to authorize
host dispatch. The sample subprocess is trusted arithmetic code; general process
timeouts/isolation, cancellation delivery, Lean jobs, and native `.nmlt` effects
remain integration work.

Unix journals require a single filesystem link. Multiply linked files are
rejected, and adding a hardlink stops further commits; this avoids an observed
WSL mounted-filesystem lock-alias failure. Native Windows locking is tested
separately. Do not replace, unlink, or roll back the active journal.

See [RFC 0018](../../rfcs/0018-bounded-local-job-lifecycle.md) for the complete
transition table, bounds, accounting, and recovery assumptions. No output here
is a Lean proof or a runtime authorization certificate.
