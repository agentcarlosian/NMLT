# `nmlt-cli`

The pre-alpha CLI exposes:

- `check`, `inspect`, and `tokens` for structural work;
- `typecheck` for Rust frontend acceptance of the supported finite slice;
- `elaborate --emit-core` for deterministic `behavior-core-v1`
  production; and
- `explore` for bounded reference execution with `assurance: none`.

Lean is not invoked by these commands. Use the separate
`nmlt-artifact-check` executable for Lean artifact interpretation and
conditional theorem-premise checking.
