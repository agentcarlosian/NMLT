# `nmlt-workflow`

Experimental `.nmlt` functions, typed outcomes, and bounded execution.
This is the executable-only value layer for R2. The pure executor rejects job
effects; `execute_with_host` exposes an explicit embedding boundary. The crate
itself owns no process launcher or journal and does not invoke Lean.

The crate consumes the existing lossless CST and complete surface projection,
then interprets retained `fn` and `record` slices. It resolves lexical binders and named
functions, checks every function and branch, rejects recursion, and builds a
private typed tree used by both execution and replay.

Supported: file-root imports, bounded local packages, local module wrappers, typed parameters/results, `Int`, `Bool`,
`Text`, `Outcome<T>`, nominal records, `List<T>`, bounded `fold`, safe `get`,
`length`, immutable `let`, `if`, exhaustive `Ok`/`Err` matching,
acyclic function calls, checked arithmetic, scalar comparisons, and Boolean
operators. Every term retains a source index and local byte span for diagnostics and stopped runs.

See the [user guide](../../docs/r2-pure-workflows.md) and
[RFC 0019](../../rfcs/0019-pure-workflow-source.md) and
[RFC 0020](../../rfcs/0020-workflow-records-and-collections.md) for syntax, bounds, source
routing, and the explicit semantic disposition. Existing M9 and finite
behavioral artifacts do not acquire these constructs or their meaning.

Use `Program::input_value` to decode structured JSON against a resolved type and
`Program::conforms` to validate typed inputs with named records. Standalone
`Value::conforms` has no record environment. Execution checks all input bounds
before copying and checks every completed value against aggregate/work limits.

`compile` checks one source buffer and rejects imports. `compile_package` loads
a bounded import closure through a caller-supplied reader, checks all scopes,
and binds every loaded source in `Program::sources`. Use `Program::source_path`
for runtime locations. The [package guide](../../docs/r2-source-packages.md) and
[RFC 0021](../../rfcs/0021-workflow-source-packages.md) describe naming, file bounds,
source locations, and the version 3 replay contract.

`job_square(Int)` is typed as `Outcome<Int>`. `Program::requires_jobs` reports
conservative transitive effects, including dead branches and imported calls.
`validate_inputs` supports preflight before creating host state. `JobHost`
implementations own scheduling, durability, response validation, and limits;
only scalar results/domain errors cross into values. Host failures stop execution.
The CLI's first adapter is described in the [source job guide](../../docs/r2-source-jobs.md)
and [RFC 0022](../../rfcs/0022-source-local-job-effects.md).
