# Integration tests

This directory holds cross-component fixtures. Crate-local unit, negative,
benchmark, and CLI integration tests remain beside their Rust crates.

`fixtures/malformed-unclosed.nmlt` is expected to fail structural parsing and
serves as a small repository-level diagnostic fixture. The frozen malformed
benchmark control and crate-level frontend tests exercise the same rejection
boundary in `make ci`.

`baselines/` tests the frozen R0 reference-workflow harness, including interruption
and resume, explicit checker failures, and local worker response handling. Run it
with `make r0-baseline-tests`. Mock checker tests establish orchestration behavior;
`make r0-baselines` separately exercises the actual pinned Lean binary. See the
[baseline contracts](../docs/r0-baseline-contracts.md) for the evidence boundary.
