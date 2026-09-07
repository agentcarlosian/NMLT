# Examples

The active typed semantic slice currently lives under
[`pivot/`](pivot/). The directory name records the transition and will
be replaced with durable `getting-started`, `open-systems`, and
`negative-controls` paths before the public stack is finalized.

- [`visible_resource_sync.nmlt`](pivot/visible_resource_sync.nmlt) is the
  primary positive source.
- [`visible_resource_sync.behavior-core-v1.json`](pivot/visible_resource_sync.behavior-core-v1.json)
  is its exact canonical artifact.
- Its [v2 artifact](pivot/visible_resource_sync.behavior-core-v2.json) and
  [initial path](pivot/visible_resource_sync.behavior-execution-v1.json) exercise
  decoded synchronization and initialized unified refinement.
- [`affine_continuation.nmlt`](pivot/affine_continuation.nmlt), its
  [v2 artifact](pivot/affine_continuation.behavior-core-v2.json), and
  [consume](pivot/receive_consume.behavior-execution-v1.json)/[return](pivot/receive_retransfer.behavior-execution-v1.json)
  paths exercise source-level continuation and exact resource steps. Run
  `make execution` to regenerate and check them.
- [`finite_value_cycle.nmlt`](pivot/finite_value_cycle.nmlt) and its
  [canonical artifact](pivot/finite_value_cycle.behavior-core-v1.json) freeze a
  closed Bool/Unit/enum graph for `make finite-parity`: one initial state, four
  reachable states and ten transitions are compared with Lean's definitions.
- [`pivot/negative/`](pivot/negative/) contains boundary-specific rejection
  controls for wiring, ownership, transfer, hidden resources, contract
  discharge, port compatibility, hidden state change, and refinement maps.
- [`finite_retry.nmlt`](pivot/finite_retry.nmlt) is R2's executable finite
  retry/reuse simulation. The [run/replay guide](../docs/r2-local-execution.md)
  shows how to change its input; it does not start a host worker.

The other directories are retained language-design and lossless-frontend
fixtures. Many use surface forms intentionally outside the first finite
behavioral slice. They are not executable semantic examples and do not inherit
the Lean theorem.

The pre-pivot composition and provider examples are historical syntax fixtures.
The old canonical corpus records frontend breadth, not current behavioral
semantics or model-check support. Active semantic claims use only the canonical
`pivot/` source/artifact fixtures and the Lean definitions, with each check's
stated scope.
