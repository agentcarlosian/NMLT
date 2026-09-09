# NMLT language sketch

Status: implemented finite slice plus clearly marked future direction.

## Current surface

The primary source fixture demonstrates the surface forms that currently reach
`behavior-core-v1`:

```nmlt
enum ContractFact { Authorized, Ready }

system Sender {
  state ready: Bool = true
  capability permit: Once<Unit>
  port output send: Once<Unit>

  action output send grade { work: 1 } {
    require ready == true
    rely ContractFact.Ready
    guarantee ContractFact.Authorized
    emit permit
    consume permit
    set ready = false
  }

  observe ready
}

system Receiver {
  state accepted: Bool = false
  port input receive: Once<Unit>

  action input receive(permit: Once<Unit>) grade { work: 2 } {
    set accepted = true
  }

  observe accepted
}

compose Network {
  connect Sender.send -> Receiver.receive
}
```

The first behavioral profile supports:

- finite `Bool`, `Unit`, and enum state;
- total finite initializers and explicit observations;
- input and output ports with nominal payload types;
- polarized actions, guards, simultaneous updates, and emitted payloads;
- affine nominal capabilities with matched transfer and receive;
- grades as finite maps from named atoms to natural numbers;
- rely and guarantee atoms from a finite enum;
- exactly two-component compositions with one-to-one connections;
- action hiding; and
- refinement with an explicit bijective state-field map (each field on either
  side has exactly one corresponding field of the same type).

Unsupported general composition, infinite domains, partial or higher-order state
maps, arbitrary grade algebras, and liveness syntax fail at a documented
frontend boundary rather than receiving an approximate semantics.

## Surface versus meaning

The lossless parser recognizes more declaration shells than the behavioral
compiler accepts. Parsing preserves source; it does not assign meaning. The
implemented semantic route is the finite profile above, emitted as canonical
`behavior-core-v1` by default, or opt-in `behavior-core-v2`, and decoded by Lean.

The ordinary typed-core route retains additional expression and property forms
for frontend research. Those forms do not automatically enter the behavioral
artifact or inherit the Lean composition theorem.

Ordinary behavior compilation rejects property declarations. The separate
[finite safety route](r2-safety-invariants.md) preserves `safety ... = always(...)`
predicates and checks initialization/preservation or a reachable counterexample
with Lean. Its observations and hiding use comma-separated
field or action names; comments are trivia. Initializer, guard, and update text
in the artifact is rendered canonically from the typed expression AST.

The first slice lowers authority-related surface forms as follows:

| Surface form | `CoreResourceProfile` field |
|---|---|
| `emit c` together with `consume c` on an output action | `transfers = {c}` |
| `consume c` without emitting it | `consumes = {c}` |
| `Once<T>` input parameter `c` | `receives = {c}` |
| `rely Fact.X` / `guarantee Fact.X` | `relies` / `guarantees` |
| `grade { a: n }` | `grade[a] = n` |
| `require expression` | transition guard, not `resources.requires` |

There is no source form for `resources.requires` yet. That field is reserved in
`behavior-core-v1`; current mutation tests exercise its Lean-side validation by
editing an artifact directly.

## Opt-in affine continuation

With `elaborate --core-version v2`, typed input capability slots remain known
to later actions. `capability` declarations still mean initial ownership.
An action that consumes a known slot before acquisition is dynamically disabled.
The [continuation fixture](../examples/pivot/affine_continuation.nmlt) demonstrates
receive then consume, or receive then return followed by sender consumption.
No new source syntax is needed for those paths. See
[the execution guide](getting-started.md#finite-v2-execution) for checked commands.

The separate executable workflow profile supplies modules, records, bounded
collections, functions, typed worker/Lean jobs and resumption. Finite user safety
predicates have a decoded-model checker. General source composition and
verified correspondence between workflow effects and models remain later work.

## Later language families

Temporal properties, behavior-indexed fairness, probabilistic and hybrid
behavior, user-defined grade algebras, general code generation,
and authenticated runtime observation remain research proposals rather than
current CLI commands or semantic claims. Closed Init proof terms are supported
through the separate pinned Lean job interface with an executable-only host ceiling.

## Design constraints

- One surface construct must not acquire different meanings in Rust and Lean.
- Authority movement, observation, hiding, assumptions, and guarantees remain
  explicit.
- Unsupported forms fail rather than silently weakening a program.
- Artifacts remain inspectable and deterministic.
- Bounded exploration never becomes an unbounded theorem.
- Future proof automation may propose evidence; a named checker and exact
  theorem boundary determine acceptance.
