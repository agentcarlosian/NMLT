# Moving job authority between components

[RFC 0027](../rfcs/0027-affine-job-transfer.md) allows direct Job handles to move
through local bindings, function parameters/results, and fold accumulators:

```nmlt
fn launch(input: Int) -> Job<Int> { job_start_square(input) }
fn finish(job: Job<Int>) -> Outcome<Int> { job_collect(job) }
fn main(input: Int) -> Outcome<Int> {
  let job = launch(input);
  finish(job)
}
```

Passing or returning a handle consumes its old binding. An old reference cannot
poll, cancel, collect, or transfer it again. Poll/cancel continue to borrow a
named local. Every normal function/scope exit collects or transfers all its
handles. The same rule applies to each incoming Job accumulator in a fold;
an empty fold returns the initial handle without dispatching or duplicating it.

The [transfer example](../examples/pivot/transfer_jobs.nmlt) starts two real
workers in a reusable component, forwards a handle through iteration, and
collects in another component. Negative input triggers fallback and returns
`Ok(50)`. Replay traverses the same transfers without creating new attempts.

Job handles stay separate from reusable values. Records, lists, outcomes, and
JSON inputs/results cannot carry them. Functions with direct Job parameters or
results are internal components; invoking them as external entries fails before
host state is created. Typecheck version 6 reports `can_run_as_entry` for this
distinction. Control calls require a named local; bind a returned handle before
polling/cancelling/collecting it.

These moves transfer source-language authority within the same host session.
They do not allocate new attempts, reset a reservation, clone an OS capability,
or imply transfer between authenticated OS principals. This remains an
executable-only ownership discipline with `assurance: none`.
