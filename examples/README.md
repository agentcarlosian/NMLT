# Examples

The active typed semantic slice currently lives under
[`pivot/`](pivot/). The directory name records the transition and will
be replaced with durable `getting-started`, `open-systems`, and
`negative-controls` paths before the public stack is finalized.

- [`visible_resource_sync.nmlt`](pivot/visible_resource_sync.nmlt) is the
  primary positive source.
- [`visible_resource_sync.behavior-core-v1.json`](pivot/visible_resource_sync.behavior-core-v1.json)
  is its exact canonical artifact.
- [`pivot/negative/`](pivot/negative/) contains boundary-specific rejection
  controls for wiring, ownership, transfer, hidden resources, contract
  discharge, port compatibility, hidden state change, and refinement maps.

The other directories are retained language-design and lossless-frontend
fixtures. Many use surface forms intentionally outside the first finite
behavioral slice. They are not executable semantic examples and do not inherit
the Lean theorem.

The pre-pivot Paper 1 and provider examples are historical syntax fixtures.
The old canonical corpus records frontend breadth, not current verifier or
model-check support. Current Paper 1
claims are governed by [`docs/paper-1-claim-ceiling.md`](../docs/paper-1-claim-ceiling.md)
and use only the pivot source/artifact pair.
