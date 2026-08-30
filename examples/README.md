# Examples

The active typed semantic slice lives under [`pivot/`](pivot/).

- [`visible_resource_sync.nmlt`](pivot/visible_resource_sync.nmlt) is the
  primary positive source.
- [`visible_resource_sync.behavior-core-v1.json`](pivot/visible_resource_sync.behavior-core-v1.json)
  is its exact canonical artifact.
- [`pivot/negative/`](pivot/negative/) contains boundary-specific rejection
  controls for wiring, ownership, transfer, hidden resources, contract
  discharge, port compatibility, hidden state change, and refinement maps.

The other directories are retained language-design and lossless-frontend
fixtures. Many use surface forms intentionally outside the first finite
behavioral slice; structural parsing of those files is not a semantic claim.

The pre-pivot Paper 1 examples are historical syntax fixtures. Current Paper 1
claims are governed by [`docs/paper-1-claim-ceiling.md`](../docs/paper-1-claim-ceiling.md)
and use only the pivot source/artifact pair.
