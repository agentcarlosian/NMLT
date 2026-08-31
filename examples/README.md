# Examples

The active typed semantic fixtures live under [`pivot/`](pivot/). The directory
name records the architectural pivot that established `behavior-core-v1`; it is
the durable fixture path for this pre-alpha artifact version.

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

The pre-pivot composition and provider examples are historical syntax fixtures.
The old canonical corpus records frontend breadth, not current behavioral
semantics or model-check support. Active semantic claims use only the canonical
`pivot/` source/artifact pair and the Lean definitions.
