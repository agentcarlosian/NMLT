# RFC 0030: Durable source resumption

- Status: Under review
- Authors: Codex, for NMLT maintainer review
- Created: 2026-09-09

## Durable decisions

The asynchronous source host flushes a source operation intent before executing
the operation, then flushes its reply before allowing the evaluator to use the
result. The canonical hash-chained source journal is bounded to 16 MiB and binds
the exact source context. Runtime dispatch inputs and generated Lean files are
retained before launch. An observation is saved before its settlement command
is committed. Source snapshots, inputs, budgets, implementation identity and
tool identities precede all source effects.

The runtime configuration advances to `nmlt-async-session-v3`, the source
context/run to version 2, and project invocation storage to version 2. Complete
source records are bounded to 32 MiB and include the decision journal plus its
effective source events. The event allowance is `8 * max_attempts + 8`; repeated
recovery cannot create an unbounded control history. Existing record versions
require their original executable.

## Restore and reconcile

Resumption requires the original executable, captured source closure and input
context, and exact configured tool identities. Under the exclusive runtime
journal lock, it validates saved observations and completes an interrupted
settlement append before classifying unresolved work. Private handles belong to
the new session instance. A never-dispatched reservation can dispatch once.
An already-dispatched attempt is never relaunched by recovery.

The evaluator re-executes the original source prefix with recorded replies.
Polling decisions and branch choices are preserved. It then continues a pending
intent or the next source operation. A lost collect reply reads the retained
collected outcome without issuing another collection. A captured uncertain
control failure can reopen only that last operation; its earlier failed reply
remains in the decision journal. The original expression bound applies to the
whole reconstructed logical execution. Job attempts, generations, reservations
and charged work are never reset.

Uncertain external effects require explicit operator reconciliation.
`--acknowledge-uncertain-effects <reason>` records a typed acknowledgement and
settles each still-uncertain attempt as failed. This is not an accepted tool
result, proof of cancellation, refund, or assertion that the effect did not
physically happen. The original reservation stays charged. Source code can
collect that failure and select a bounded fallback. The attempt's run/task/
slot/generation binding is its idempotency identity; no exactly-once claim is
made for an external service that does not honor it.

## Commands and incomplete writes

`jobs-resume` continues a saved source session and writes a new result record.
`resume <run-directory> --project <directory>` reconstructs the saved project
invocation and produces a new project record. The working sources and manifest
may have changed; they do not replace the captured inputs. All new project job
runs use the resumable host, including the convenience `job_square` operation.
Standalone legacy synchronous records retain their inspection/replay route;
passing `--job-slots` selects the resumable route for a new such run.

An incomplete final append is rejected by default. The explicitly acknowledged
`jobs-repair` command first validates every complete row and preserves the raw
incomplete suffix, original/prefix hashes and reason in a quarantine artifact.
It then trims only that suffix under the journal lock. It never discards a
malformed complete row, repairs a missing header, acknowledges unknown effects,
or dispatches work. A subsequent resumption still validates both journals and
requires any uncertainty acknowledgement. Repairs across files are recoverable
steps, not a filesystem transaction.

## Trust and validation

The filesystem is a cooperating local single-writer store. Flushed contents,
rename behavior, pinned executable/tool files, and OS process observations are
trusted. A copied or rolled-back store is not fenced and coherently forged
records are not authenticated. Unix directory flushes are used where available;
Windows evidence publication retains the local filesystem's rename boundary.
Process cleanup has the platform-specific limits in RFC 0028. Replay remains
captured consistency, not a fresh external check or host execution theorem.

Validation covers nine coherent durable-prefix boundaries, saved observations
awaiting settlement, lost collect replies, a captured timeout/retry, explicit
uncertainty with retained spend, exhausted attempt budgets, corrupted sources,
missing evidence, incomplete tails and quarantine, and project resumption after
working edits. Actual process death, stale generations, cancellation/late results
and exclusive journal ownership retain the runtime's separate tests.
