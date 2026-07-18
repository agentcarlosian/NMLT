# Language sketch

Status: illustrative design fixture, non-normative.

## Surface layers

NMLT is expected to expose four coordinated layers:

1. pure mathematical definitions;
2. systems, state, events, and actions;
3. temporal properties, contracts, and refinements;
4. verification requests and evidence inspection.

## Illustrative syntax

```nmlt
system ProviderAttempt(req: Request) {
  state phase: Phase = proposed
  state dispatch_count: Nat = 0
  state response: Option<Response> = none

  capability provider_call:
    Once<Provider, request_hash = hash(req)>

  action authorize {
    require phase == proposed
    set phase = authorized
  }

  action dispatch {
    require phase == authorized
    consume provider_call
    set phase = dispatched
    set dispatch_count = dispatch_count + 1
  }

  safety DispatchRequiresAuthority =
    always(dispatch_count > 0 implies authorized_before_dispatch)

  temporal NoBlindReplay =
    always(phase == indeterminate implies next(not enabled(dispatch)))

  refine RuntimeJournal {
    observe phase, response
    stutter internal_bookkeeping
  }
}
```

Current parsing recognizes only balanced `system Name { ... }` declarations.
Every other construct above is a proposed design fixture.

## Candidate declaration families

```text
module, import, data, type, fn, theorem
system, state, event, port, capability, budget
action, require, set, emit, consume
safety, temporal, fair, assume, guarantee
compose, hide, observe, refine, monitor
verify, evidence
```

## Syntax design constraints

- The mathematical core must remain readable without ASCII punctuation noise.
- State updates must identify every modified location; accidental frame
  conditions should be statically diagnosable.
- Nondeterminism, partiality, effects, and assumptions must be explicit.
- Surface process notation must elaborate to inspectable actions.
- Verification annotations select methods but cannot alter proposition meaning.
- Bounds belong to evidence scope, not to the meaning of an unbounded property.

## Open questions

- Whether actions use relational blocks, explicit primed variables, or both.
- Whether the language is indentation-sensitive.
- How proof terms and tactics appear at the surface.
- Whether effect grades are inferred, declared, or mixed.
- How executable foreign functions expose semantic contracts.
- Which temporal operators belong to the stable core.
