# Integration tests

`make execution` regenerates the v2 cores and three finite path witnesses,
runs 30 rejection controls through the actual Lean executable, and compares
the continuation fixture's complete reachable resource graph with Lean. The
corpus includes true-guard use before acquisition, repeat consumption, sender
use after transfer, hidden-world changes, sentinel indices, malformed maps,
byte-binding mismatches, and a failed declared refinement.

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

`make finite-parity` regenerates the
[finite value fixture](../examples/pivot/finite_value_cycle.nmlt), builds Lean,
and compares the complete reachable state/transition sets from Rust with the
Lean-defined control behavior. The fixture is closed and resource-free; this
four-state comparison does not establish general Rust-to-Lean correspondence.

`make r2-jobs` exercises the executable-only local subprocess prototype with
failure/fallback, direct success, and exhausted attempts. Crate-local
`nmlt-runtime` tests cover lifecycle bounds, stale controls/responses, cancellation,
accounting, journal corruption and I/O failures, locking, and real process death
both before work and after worker output survives without a completion record.
These are implementation regressions, not Lean checks or host-execution proofs.
