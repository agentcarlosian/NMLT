# Pure source workflows

R2 now runs input-dependent pure functions from `.nmlt` source. The
[example](../examples/pivot/pure_fallback.nmlt) handles a failed first input,
tries a fallback, and reuses a successful value. This route reports
`assurance: none`: it runs no host worker and performs no Lean checking.

From the repository root, in Bash or PowerShell:

```bash
cargo build -p nmlt-cli
target/debug/nmlt typecheck examples/pivot/pure_fallback.nmlt --profile workflow
target/debug/nmlt run examples/pivot/pure_fallback.nmlt --entry main --arg input=-3 --arg fallback=5 --max-steps 1000 --emit-run target/pure-run.json
target/debug/nmlt replay target/pure-run.json --source examples/pivot/pure_fallback.nmlt
```

Choose a new output filename for every run. The program's first call returns
`Err("negative input")`. The fallback computes `25`, and the matching branch
uses that value twice to return `Ok(50)` after 26 expression steps. Set
`--arg input=4` with a new output path to return `Ok(32)`. Set both inputs
negative to retain `Err("negative input")` as the program's domain result.

An entry is any named function selected by `--entry`. Function parameters are
positional inside the language; CLI inputs use exact parameter names. Supply
`--arg name=value` for each parameter, with integer, Boolean, or quoted
JSON string syntax. For example, pass a Text input as the single shell argument
`'message="hello"'` in Bash or PowerShell.
Extra, missing, duplicate, or ill-typed inputs are errors.

For a batch of related inputs, use a record and `List<T>`. The
[batch example](../examples/pivot/batch_summary.nmlt) accepts a list of
`Work.Item` records, tries each item's fallback when necessary, and folds the
results into a `Summary` record:

```bash
target/debug/nmlt run examples/pivot/batch_summary.nmlt --entry main --arg 'items=[{"input":-3,"fallback":5},{"input":4,"fallback":9},{"input":-1,"fallback":-2}]' --max-steps 1000 --emit-run target/batch-run.json
target/debug/nmlt replay target/batch-run.json --source examples/pivot/batch_summary.nmlt
```

This produces `accepted: 2`, `rejected: 1`, and `total: 41`. An empty input
list produces zeros. Lists contain at most 256 items. Record input objects
must contain exactly their declared fields; duplicate JSON keys are errors.
Outcome inputs use `{"Ok": value}` or `{"Err": "message"}`. The structured-input
commands above use Bash quoting.

Declare `record Name { field: Type, ... }`, construct
`new Name { field: expression, ... }`, and read `value.field`.
`fold(items, initial, acc, item => body)` visits items in order and requires
the body to return the accumulator's type. It consumes the same step budget
as calls and other expressions. `length(items)` returns Int;
`get(items, index)` returns `Ok(value)` or `Err("list index out of bounds")`.
Use an annotation such as `let xs: List<Int> = []; body` for an empty list
whose element type cannot otherwise be inferred.

```nmlt
fn positive(x: Int) -> Outcome<Int> {
  if x < 0 { Err("negative input") } else { Ok(x) }
}

fn twice(input: Int) -> Outcome<Int> {
  let result = positive(input);
  match result {
    Ok(value) => Ok(value + value),
    Err(message) => Err(message)
  }
}
```

Use local `module Name { ... }` wrappers for named groups and `Name.function(...)`
for qualified calls. Unqualified calls search the current module and then the
file root. Functions can call later declarations but cannot be recursive.
`let name: Outcome<Int> = Err("..."); body` supplies the expected payload type
when it cannot otherwise be inferred. Both match arms are mandatory and checked;
an Err payload is Text, and an Ok payload has the declared result type.

| Execution result | CLI exit | Meaning |
|---|---|---|
| `returned` with `Ok(...)` or another value | 0 | Pure evaluation produced that value |
| `returned` with `Err(...)` | 0 | Evaluation completed with a domain failure value |
| `step_limit` | Nonzero | Explicit expression budget exhausted; no returned value |
| `depth_limit` | Nonzero | Runtime expression depth exceeded; no returned value |
| `integer_overflow` | Nonzero | Checked Int arithmetic overflowed; no returned value |
| `value_limit` | Nonzero | An aggregate value exceeded its size/depth bound |
| `value_work_limit` | Nonzero | Repeated value production/copying exhausted its accounting budget |

An overflow stops the execution; it does not automatically become the function's
Err branch. Replay preserves all these outcomes, including incomplete ones.
No outcome is a successful host job or accepted proof merely because evaluation
or replay returned exit zero.

Bounds include 128 KiB source, 64 functions, 16 parameters per function,
4,096-byte text, and an explicit 1–100,000 expression-step limit. Full grammar,
depth limits, precedence, and unsupported constructs are in
[RFC 0019](../rfcs/0019-pure-workflow-source.md) and
[RFC 0020](../rfcs/0020-workflow-records-and-collections.md). Each aggregate
value is capped at 4,096 nodes and 65,536 payload/name bytes; all inputs and
completed expression values together consume at most 1,000,000 value-work units.
These are deterministic evaluator bounds, not host memory or time quotas.

Run records bind source bytes, executable, typed program, inputs, selected
entry, limits, and the result. Replay recompiles and reevaluates with the exact
recorded executable. Moving identical source is allowed. Whitespace changes
to source require a fresh run; formatting changes to record JSON are allowed.
Unknown fields, duplicate keys, unsupported formats, and inconsistent results
are rejected. These consistency checks do not authenticate a record's producer.
The current profile and pure record formats are version 3. Keep the original
executable to replay version 1/2 records, or run the source again to create a new
record. Older pure records are not silently migrated. Run output serializes
tagged values with their nominal record names; CLI input uses ordinary JSON.

`--entry` selects the pure workflow route. Existing finite execution uses
`--behavior`, described in the [finite guide](r2-local-execution.md).
The two modes cannot be mixed. The pure route explicitly rejects systems,
properties, and other unsupported declarations; the finite
route does not turn pure functions into behavioral artifacts. Ordinary `check`
still requires a system; use `typecheck --profile workflow` as shown above.

Use [source packages](r2-source-packages.md) to import sibling `.nmlt` libraries
and bind every dependency's source identity during replay. Compiler diagnostics
and runtime stops retain the originating file and local byte span.

The [source job guide](r2-source-jobs.md) adds opt-in `job_square` effects through
the durable [Rust job runtime](../crates/nmlt-runtime/README.md). An effectful
entry is rejected by this pure execution route. Workflow typecheck output is
version 4 and exposes `requires_jobs`; pure run records remain version 3.
Source Lean integration, asynchronous source job control, and the complete R2 exit gate
remain outstanding.
