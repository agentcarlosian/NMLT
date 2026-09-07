# R2 first increment: local finite execution

R2 is in progress. The first increment runs the existing finite v2 language
directly from source and replays a structured record. The full executable
workflow language, host adapters, and R2 exit gate remain outstanding.

The [second increment](../crates/nmlt-runtime/README.md) adds a separate Rust
job lifecycle, durable journal, and subprocess adapter prototype. Native source
effects remain outstanding; the commands below still execute only finite steps.

Build once, then use that same executable for both commands:

```bash
cargo build -p nmlt-cli
target/debug/nmlt run examples/pivot/finite_retry.nmlt --behavior Main --max-steps 8 --emit-run target/retry-run.json
target/debug/nmlt replay target/retry-run.json --source examples/pivot/finite_retry.nmlt
```

On Windows the executable is `target/debug/nmlt.exe`. Choose a new output file
for each run: existing records and source files are never overwritten.

The [example](../examples/pivot/finite_retry.nmlt) takes five finite transitions:
transfer a permit, record a simulated failure, retry once, finish while consuming
the permit, and reuse the Boolean result. Its outcome is `quiescent` and its
cumulative `model_grade.work` is 2. These actions do not launch host jobs.

Change `reject_first: Bool = true` to `false` in a copy of the source and run
that copy into a new record. Execution now takes three transitions and reports
model work 1. Replay the new record with the changed source; attempting to replay
the original record with changed bytes is an explicit source mismatch.

By default the runner picks the lexicographically first enabled label. This
deterministic policy can starve another action and makes no fairness claim.
To select a finite path yourself, append `--actions` followed by comma-separated
labels, as in:

```bash
target/debug/nmlt run examples/pivot/affine_continuation.nmlt --behavior Network --max-steps 8 --emit-run target/consume-run.json --actions 'Receiver.receive|Sender.send,Receiver.monitor,Receiver.use'
target/debug/nmlt replay target/consume-run.json --source examples/pivot/affine_continuation.nmlt
```

The quoting works in Bash and PowerShell and protects the `|` inside action
labels. Trying `Receiver.use` first yields an `action_unavailable` record with
the unchanged initial authority and currently enabled alternatives.

| Outcome | Meaning | `run` exit |
|---|---|---|
| `quiescent` | No enabled step under this finite interpretation | 0 |
| `schedule_complete` | All explicitly requested actions ran | 0 |
| `step_limit` | The explicit bound stopped further execution | Nonzero |
| `action_unavailable` | The requested next label was disabled or unknown | Nonzero |
| `grade_overflow` | The next cumulative modeled grade exceeded `u64` | Nonzero |

The bound is required and ranges from 1 to 10,000 steps, including self-loops.
Incomplete outcomes retain the executed prefix. Replay can successfully match
an incomplete run and preserves its original outcome; `matched: true` never
turns a failed run into a successful workflow. Quiescence can be a deadlock.

Records contain full typed states, authority, action labels, grades, source and
artifact hashes, scheduler settings, and the exact executable hash. Binary
changes require a new run or use of the preserved original executable. Identical
source may move: replay takes an explicit `--source` path. Record whitespace and
object-key ordering do not affect matching, but unknown fields are rejected.

Every result reports `assurance: none`. Replay is a consistency check using the
same implementation. It neither invokes Lean nor authenticates the record's
producer. It is not a crash-recovery journal, project lock, or host-effect log.
Incomplete writes remain invalid records. Modeled grades are separate from
observed expenditure and enforced reservations, which are not implemented here.

Existing `elaborate`, `explore`, and `trace` remain available with their earlier
contracts. Use their separate Lean checking path for finite path evidence.
[RFC 0017](../rfcs/0017-finite-local-run-and-replay.md) specifies source routing,
stop precedence, serialization, and the executable-only scope of this increment.
