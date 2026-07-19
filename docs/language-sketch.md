# Language sketch

Status: illustrative design fixture, non-normative.

## Surface layers

The flagship NMLT language is expected to expose four coordinated layers:

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
    always(phase == indeterminate implies not enabled(dispatch))

  refine RuntimeJournal {
    observe phase, response
    stutter internal_bookkeeping
  }
}
```

The current lossless frontend recognizes the Phase 1 declaration shells,
including modules and surface-only data/record/function forms, systems, state,
actions, `require`, explicit `set` targets, capabilities, properties,
observations, and hiding. It projects that structure into a complete untyped
surface artifact while preserving unsupported or recovered nodes explicitly.
M9 strengthens that boundary into an ordered, origin-censused surface
projection: modules are not flattened, imports and enums are structured,
system/action parameters remain distinct, and no semantic CST node may vanish.
The first M9 feature profile remains intentionally narrower than everything the
lossless parser recognizes.
Expression precedence, name resolution, typing, effects, temporal meaning,
refinement meaning, and general execution remain narrow or deferred; the full
example above is still an illustrative design fixture rather than an
end-to-end verified program.

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
