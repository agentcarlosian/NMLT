# Run local jobs from source

R2 now connects workflow functions to the existing local square subprocess.
`job_square(Int)` returns `Outcome<Int>`: negative inputs and square overflow
produce `Err`, and successful results can be reused as ordinary values.
This is experimental, executable-only behavior with `assurance: none`.

Build the CLI, then run the [fallback example](../examples/pivot/job_fallback.nmlt).
The output file and jobs directory must both be new, with existing parents:

```bash
cargo build -p nmlt-cli
target/debug/nmlt run examples/pivot/job_fallback.nmlt --entry main --arg input=-3 --arg fallback=5 --max-steps 100 --emit-run target/job-run.json --jobs-dir target/job-state --max-jobs 2 --job-timeout-ms 10000
target/debug/nmlt replay target/job-run.json --source examples/pivot/job_fallback.nmlt
```

On Windows use `target/debug/nmlt.exe`. The first worker rejects -3; the fallback
returns 25; the source returns `Ok(50)` by adding the collected value twice.
Two attempts are charged, one slot is reused, and value reuse starts no job.
The record uses `nmlt-local-job-run-v1`. Replay checks the captured responses and
journal without launching workers or changing the live journal.

Set `--max-jobs 1` with fresh output paths to see a `job_stopped` limit outcome
before fallback. `--max-steps` bounds expression evaluation separately. The job
limit is 1–16 total attempts; the worker timeout is 1–30,000 milliseconds. Each
output pipe is limited to 64 KiB. Timeout/output/protocol/process failures stop
execution and retain uncertain charged work; they are not source `Err` values.
The record reports whether direct-child cleanup was confirmed. These limits
do not impose CPU/memory quotas or contain process trees.

Use `typecheck <source> --profile workflow` to inspect each entry's
`requires_jobs` flag. Effects propagate through imports and function calls,
including unselected branches. Running an effectful entry without all three
job options fails before work begins. Pure entry points retain the existing
effect-free run/replay route. The language does not expose commands, shell
strings, job handles, or source cancellation controls in this increment.

The jobs directory contains `context.json`, saved before dispatch, and a locked
`journal.jsonl` that records intent before work. If the process is interrupted:

```bash
target/debug/nmlt jobs-recover target/job-state
```

This requires the original executable and classifies unfinished journal state.
It does not redispatch or resume the source workflow, locate/kill surviving
workers, or reconcile a replacement result. A torn log is rejected. A failed
setup or write may leave an empty/partial run record; retain the context and
journal for inspection. Repeated recovery of unfinished work advances revisions
and uses the bounded event budget. Completed journals remain unchanged.

Source bytes, imports, typed program, entry, inputs, adapter, and limits are
bound into the run context. Replay rejects mismatches and altered events.
It checks consistency, not authenticated provenance or physical host facts.
Job records do not establish Lean acceptance or verified host execution.

See [RFC 0022](../rfcs/0022-source-local-job-effects.md) for the exact semantics,
failure boundaries, compatibility, and remaining R2 work.
