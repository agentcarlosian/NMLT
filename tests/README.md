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

`nmlt-workflow` tests compile and run the pure function/outcome profile, including
all-branch checking, lexical/module resolution, recursion rejection, depth/step
bounds, and arithmetic stops. CLI workflow tests exercise real input changes,
failure handling, immutable reuse, replay, record mutation/duplicates, output
preservation, and explicit routing away from the finite behavioral checker.
Collection controls cover nominal record resolution, recursive type rejection,
exact fields, fold order/scope and shared fuel, empty/maximal lists, safe lookup,
structured input decoding, aggregate growth, and repeated copying. CLI controls
replay structured batches and incomplete value-limit runs, reject nested duplicate
keys, and explicitly reject version 1 pure records.

Package controls cover deterministic/shared imports, file scopes, record-only
libraries, imported diagnostic spans, runtime source indices, cycles, alias/case
collisions, graph/source bounds, dependency changes, manifest mutations, package
moves, and version 1/2 rejection. Linux also exercises symbolic-source rejection.

Source job controls cover transitive/dead-branch/imported effects, typed job
results and fold order, pure-route rejection, real subprocess fallback and
overflow, collection/value reuse, preflight and attempt/step limits, and exact
context/event/journal replay. Supervisor tests exercise blocked stdin, both
bounded output pipes, nonzero exit, unexpected stderr, and direct-child timeout
cleanup. Injected host failures retain uncertain charged work without selecting
fallback. Recovery tests classify a complete interrupted journal prefix without
redispatch and reject torn logs. These controls join the existing workspace gate.

The asynchronous host controls test independent deadlines, late polling, pipe
bounds, cancellation cleanup, slot/attempt limits, session-specific handles,
single collection, uncertain charges, injected partial journal writes, and
captured-observation mutations. Lean adapter unit tests distinguish rejection,
exact empty-axiom acceptance, admitted/wrong-proof controls, and tool failures.
`make r2-async` runs real worker concurrency/replay/recovery in the Rust gate;
`make r2-lean` separately runs actual pinned Lean templates in the complete gate.
These exercise the initial Rust API; source-level asynchronous handles remain open.
