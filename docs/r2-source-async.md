# Asynchronous and resumable source jobs

Source workflows start worker and pinned Lean jobs, poll them, request
cancellation, and collect typed outcomes. Affine handles can move through
functions, branches and bounded folds; they are collected or transferred on
every normal path. This profile is executable-only with `assurance: none`.
The original [RFC 0024](../rfcs/0024-scoped-source-job-controls.md) is extended by
[transfer](../rfcs/0027-affine-job-transfer.md),
[proof terms](../rfcs/0029-pinned-init-proof-terms.md), and
[durable resumption](../rfcs/0030-durable-source-resumption.md).

```bash
cargo build -p nmlt-cli
target/debug/nmlt run examples/pivot/async_fallback.nmlt --entry main --arg input=-3 --arg fallback=5 --max-steps 100 --emit-run target/async-run.json --jobs-dir target/async-jobs --job-slots 2 --max-jobs 2 --job-timeout-ms 30000
target/debug/nmlt replay target/async-run.json --source examples/pivot/async_fallback.nmlt
target/debug/nmlt jobs-recover target/async-jobs
```

Use `nmlt.exe` on Windows. The initial output file and job directory must be
new. The example starts both jobs before collecting either: negative input
fails, the backup returns 25, and its reusable value produces `Ok(50)`.

| Operation | Type and effect |
|---|---|
| `job_start_square(input)` | Reserves and dispatches a `Job<Int>` |
| `job_start_lean("existing_lemma")` | A `Job<Text>` for a legacy zero-addition template |
| `job_start_lean_check(statement, proof)` | A `Job<Text>` for dynamic closed Init proof terms |
| `job_poll(h)` | Borrows the named handle and returns readiness as `Bool` |
| `job_cancel(h)` | Borrows the handle; reports confirmed cancellation, with collection still required |
| `job_collect(h)` | Consumes the named handle and returns `Outcome<T>` within its deadline |
| `job_square(input)` | Convenience start/collect using this same host when selected |

The template literals are `wrong_term`, `existing_lemma`, `induction`, and
`admitted`. The [proof-term interface](r2-lean-terms.md) takes ordinary `Text`
inputs rather than a fixed strategy selection. Both require the pinned direct
`--lean-bin` executable. Acceptance requires a matching checked target and the
exact empty-axiom report; rejection does not refute the statement.

Handles never enter serializable source values or external entry inputs/outputs.
They can be moved once, passed to components and returned as direct `Job<T>`
results. Records, lists and outcomes cannot contain them. Branches consume the
same outer handles; folds transfer their job accumulator once per iteration.
See [the transfer guide](r2-job-transfer.md). Collected values are freely reusable.

Slots are bounded to 1–4, attempts to 1–16, and timeout to 1–30,000 ms. Slots
cannot exceed attempts. Expression/value-work limits and observed host work are
separate. Timeout and ambiguous effects remain uncertain and charged. Confirmed
cancellation becomes `Err("job cancelled")` when collected. The common
[process supervisor](../rfcs/0028-contained-process-lifecycle.md) records its
platform-specific tree/resource policy; it is not a filesystem/network sandbox.

The job directory retains source files, the exact executable, manifests,
versioned session context, generated inputs, completed observations, the runtime
journal and `source-journal.jsonl`. Source intents are flushed before effects;
replies are flushed before source decisions. The `nmlt-async-source-run-v2`
record binds these identities, inputs, limits and decisions. Replay needs no
live journal and starts no worker or Lean process. Captured consistency does
not authenticate physical observations or provide a fresh Lean check.

[Resume saved work](r2-recovery.md) with `jobs-resume`; unresolved external effects
require explicit acknowledgement before failure/fallback. Completed data and
lost collect replies are reused without redispatch. Source records are bounded
to 32 MiB, the decision journal to 16 MiB. All project job runs use this host.
Standalone convenience-worker runs select it with `--job-slots`; the earlier
synchronous format retains its separate inspection/replay route.

`typecheck --profile workflow` reports version 6 and each entry's job-effect
flags. The finite and pure profiles remain separate and retain their claim
ceilings. The [safety invariant guide](r2-safety-invariants.md) describes the
finite model feature; workflow effects do not automatically inherit it.

```bash
make r2-source-async
make r2-source-lean R2_LEAN_BIN=/absolute/path/to/lean-4.33.1/bin/lean
make r2-lean-terms R2_LEAN_BIN=/absolute/path/to/lean-4.33.1/bin/lean
```
