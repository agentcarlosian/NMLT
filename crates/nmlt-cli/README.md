# `nmlt-cli`

The pre-alpha CLI exposes:

- `check`, `inspect`, and `tokens` for structural work;
- `typecheck` for Rust frontend acceptance of the supported finite slice;
- `elaborate --emit-core` for deterministic `behavior-core-v1`
  production; and
- `explore` for bounded reference exploration with `assurance: none`;
- opt-in v2 elaboration and `trace` for untrusted finite execution witnesses;
- `run --behavior` for direct bounded v2 execution from source;
- `run --entry` and `typecheck --profile workflow` for source functions and effect summaries;
- `run --entry ... --jobs-dir ... --max-jobs ... --job-timeout-ms ...` for bounded local jobs;
- `jobs-recover` for unfinished journal classification without redispatch; and
- `replay` for record consistency with the exact source and executable.

The [R2 guide](../../docs/r2-local-execution.md) describes options, JSON results,
partial runs, and the experimental finite scope. `run` and `replay` always
report `assurance: none`. Finite and pure execution do not start host jobs.
The [pure workflow guide](../../docs/r2-pure-workflows.md) covers named inputs,
typed `Ok`/`Err` values, records, bounded lists/folds, acyclic calls, matching,
and version 3 pure run replay. The [package guide](../../docs/r2-source-packages.md)
covers sibling imports, file-aware diagnostics, and dependency manifests.
The [source job guide](../../docs/r2-source-jobs.md) covers typed `job_square`
effects, durable context, supervision, collection, and journal replay without
launching workers. Workflow typecheck output is version 4; pure runs remain
version 3 and local job runs use their own version 1 profile.

Lean is not invoked by these commands. Use the separate
`nmlt-artifact-check` executable for Lean artifact interpretation and
conditional theorem-premise checking.
