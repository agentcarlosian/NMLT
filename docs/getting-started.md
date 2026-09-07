# Getting started with NMLT

NMLT is pre-alpha research software. The current end-to-end path is deliberately
small: one finite, resource-aware open system travels from `.nmlt` source through
Rust elaboration into a canonical artifact, then into Lean-defined semantics.

## Prerequisites

- Rust 1.94 with Cargo;
- Lean 4.33.1 through Elan and Lake;
- a POSIX shell, Python 3.11 or newer as `python3`, and GNU core utilities including `sha256sum`; and
- WSL when running the complete repository gate from Windows.

The repository pins the Rust and Lean versions used by CI.

## The primary program

The canonical source is
[`visible_resource_sync.nmlt`](../examples/pivot/visible_resource_sync.nmlt).
It declares:

- an abstract sender and a concrete sender;
- a receiver with a visible Boolean observation;
- an affine `permit` capability carried by `Once<Unit>`;
- complementary output and input ports;
- named natural-number work grades;
- `Ready` and `Authorized` rely/guarantee facts;
- abstract and concrete binary compositions; and
- an explicit state-field refinement map.

The concrete sender's action contains the central resource-bearing interaction:

```nmlt
action output send grade { work: 1 } {
  rely ContractFact.Ready
  guarantee ContractFact.Authorized
  emit permit
  consume permit
}
```

On an output action, emitting and consuming the same affine capability lowers to
a transfer. The receiver binds that capability through its `Once<Unit>` input.
The synchronized transition changes the receiver's observed bit, moves the
`permit` from sender to receiver, and adds the component grades to `work: 3`.

The source closes with an explicit refinement:

```nmlt
refine ConcreteSender refines AbstractSender {
  map state unit -> unit
}
```

## Check the source

```bash
cargo run -p nmlt-cli -- typecheck \
  examples/pivot/visible_resource_sync.nmlt
```

This runs the Rust frontend and typed acceptance path. It does not invoke Lean
or make a semantic proof claim.

## Emit the canonical behavior artifact

```bash
cargo run -p nmlt-cli -- elaborate \
  examples/pivot/visible_resource_sync.nmlt \
  --emit-core /tmp/visible-resource-sync.json
```

The result is deterministic `behavior-core-v1` JSON containing finite state,
typed term trees, action resource profiles, ports, wiring, and refinement data.
The committed reference artifact is
[`visible_resource_sync.behavior-core-v1.json`](../examples/pivot/visible_resource_sync.behavior-core-v1.json).

Its source digest identifies the exact source bytes supplied to the next step.
It does not prove that Rust translated those bytes correctly; repository
reproduction separately checks byte equality for this fixture.

## Check the artifact in Lean

```bash
cd mechanization/lean
lake exe nmlt-artifact-check /tmp/visible-resource-sync.json \
  ../../examples/pivot/visible_resource_sync.nmlt
cd ../..
```

Lean decodes the artifact, constructs the finite behaviors, decides the current
composition and refinement premises, and constructs the conditional theorem
witnesses. The checked declarations and their boundaries are documented in the
[Lean package guide](../mechanization/lean/README.md) and
[current calculus](core-calculus.md).

## Explore the behavior

```bash
cargo run -p nmlt-cli -- explore \
  --behavior ConcreteNetwork --max-states 8 \
  /tmp/visible-resource-sync.json
```

The reference explorer displays two states and one synchronization:

```text
state 0: authority=[permit=ConcreteSender]
state 1: Receiver.bit=true authority=[permit=Receiver]
step 0 -> 1: ConcreteSender.send|Receiver.receive
             grade=[work=3]
             transfers=[permit: ConcreteSender -> Receiver]
```

Exploration always reports `assurance: none`. It is an operational design and
debugging tool, not a proof engine.

## Finite v2 execution

V2 separates known capability types from initial ownership. The continuation
example can receive a permit and either consume it or return it to the sender.
From the repository root:

```bash
cargo run -p nmlt-cli -- elaborate examples/pivot/affine_continuation.nmlt \
  --core-version v2 --emit-core /tmp/continuation.json
cargo run -p nmlt-cli -- trace --behavior Network \
  --actions 'Receiver.receive|Sender.send,Receiver.monitor,Receiver.use' \
  --emit-path /tmp/consume-path.json --max-states 32 /tmp/continuation.json
cd mechanization/lean
lake exe nmlt-artifact-check /tmp/continuation.json \
  ../../examples/pivot/affine_continuation.nmlt /tmp/consume-path.json
cd ../..
```

The trace command emits an untrusted witness. Lean checks its selected binary
composition, decoded initializer, every control/authority step, and source/core
byte bindings. It accepts this three-step path and derives reachability and
ownership properties. For the return path, use actions
`Receiver.receive|Sender.send,Receiver.giveback|Sender.returned,Sender.finish`.
Attempting `Receiver.use` before acquisition is disabled even though that action
has no Boolean guard.

`make execution` reproduces both v2 cores and all three committed path witnesses,
including the primary fixture's initial synchronized refinement. It also runs
30 rejection controls and compares the resource fixture's complete reachable
graph with Lean: one initial state, eight reachable states, twelve transitions.
This is finite binary execution; host jobs and the general interpreter are R2.

The default `elaborate` and two-argument v1 checker retain their earlier contract.
Migration means recompiling with `--core-version v2`, then regenerating paths;
changing a schema string does not supply the required maps. Unsupported versions
are rejected. Control indices in a path refer to the canonical finite-domain
ordering specified by [RFC 0016](../rfcs/0016-decoded-finite-execution.md).

## Run source directly

R2's first increment supports bounded finite `run` and exact-executable
`replay`, including a simulated failure/retry and reusable result. Follow the
[local execution guide](r2-local-execution.md). The general language and host
adapters remain outstanding; this path always reports `assurance: none`.

## Run the repository gates

```bash
make ci
make metatheory
make reproduce
```

- `make ci` runs formatting, compilation, Clippy, Rust tests, artifact
  reproduction, public-surface checks, R0 workflow harness tests, and the R2
  local subprocess adapter cases.
- `make metatheory` builds Lean, tests fail-closed artifact mutations, scans for
  unchecked placeholders, and audits focused theorem axioms.
- `make reproduce` additionally runs the pinned independent NanoDA check over the
  complete `NMLT` module, the three R0 reference workflows with real Lean, and
  the frozen finite-value comparison and v2 execution gate.
- `make finite-parity` compares the complete reachable graph for one closed
  Bool/Unit/enum fixture: one initial state, four reachable states, and ten
  transitions. It rejects truncation and does not establish compiler correctness.
- `make r2-jobs` exercises the [job runtime prototype](../crates/nmlt-runtime/README.md):
  failure/retry, direct success, and all-failed subprocess cases, with typed
  results and a durable lifecycle log. This does not add host effects to `nmlt run`.

Run `make r0-baselines` to try the frozen proof, discovery, and local worker
examples. Each invocation saves a fresh record directory under
`target/r0-baselines`. The [baseline guide](../examples/baselines/README.md)
explains direct commands, interruption, and resume. These host-language examples
establish a comparison baseline for the complete NMLT workflow interpreter.

Continue with the [language sketch](language-sketch.md),
[architecture](architecture.md), or [current calculus](core-calculus.md).
