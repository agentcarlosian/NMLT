# Getting started with NMLT

NMLT is pre-alpha research software. The current end-to-end path is deliberately
small: one finite, two-component resource-aware fixture with typed boundary
actions travels from `.nmlt` source through Rust elaboration into a canonical
artifact, then into Lean-defined semantics.

## Prerequisites

- Rust 1.94 with Cargo;
- Lean 4.30 through Elan and Lake;
- a POSIX shell, `python3`, and GNU core utilities including `sha256sum`; and
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
behavior: ConcreteNetwork
assurance: none (reference exploration only)
states: 2
transitions: 1
truncated: false
state 0: ConcreteSender.unit=false, Receiver.bit=false authority=[permit=ConcreteSender]
state 1: ConcreteSender.unit=false, Receiver.bit=true authority=[permit=Receiver]
step 0 -> 1: ConcreteSender.send|Receiver.receive grade=[work=3] transfers=[permit: ConcreteSender -> Receiver]
```

Exploration always reports `assurance: none`. It is an operational design and
debugging tool, not a proof engine.

## Run the repository gates

```bash
make ci
make metatheory
make reproduce
```

- `make ci` runs formatting, compilation, Clippy, Rust tests, artifact
  reproduction, and public-surface checks.
- `make metatheory` builds Lean, tests fail-closed artifact mutations, scans for
  unchecked placeholders, and audits focused theorem axioms.
- `make reproduce` additionally runs the pinned independent NanoDA check over the
  complete `NMLT` module.

Continue with the [language sketch](language-sketch.md),
[architecture](architecture.md), or [current calculus](core-calculus.md).
