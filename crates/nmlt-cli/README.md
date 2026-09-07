# `nmlt-cli`

The pre-alpha CLI exposes:

- `check`, `inspect`, and `tokens` for structural work;
- `typecheck` for Rust frontend acceptance of the supported finite slice;
- `elaborate --emit-core` for deterministic `behavior-core-v1`
  production; and
- `explore` for bounded reference exploration with `assurance: none`;
- opt-in v2 elaboration and `trace` for untrusted finite execution witnesses;
- `run` for direct bounded v2 execution from source; and
- `replay` for record consistency with the exact source and executable.

The [R2 guide](../../docs/r2-local-execution.md) describes options, JSON results,
partial runs, and the experimental finite scope. `run` and `replay` always
report `assurance: none` and do not start host jobs.

Lean is not invoked by these commands. Use the separate
`nmlt-artifact-check` executable for Lean artifact interpretation and
conditional theorem-premise checking.
