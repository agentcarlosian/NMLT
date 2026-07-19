# Provider-attempt suite v2 correction

Status: frozen on 2026-07-18. This suite supersedes
`provider-attempt-seeded-defects-v1`.

## Reason for the new suite version

Suite v1 encoded `NoBlindReplay` as:

```nmlt
always(phase == indeterminate implies next(not enabled(dispatch)))
```

That formula checks enabledness only in the successor state. It therefore
permits one replay from an indeterminate state whenever the replay moves to a
state in which dispatch is disabled. This is an off-by-one error relative to
the intended authority boundary: a system must reconcile ambiguity *before*
another externally effective dispatch becomes enabled.

Suite v2 uses the state-local safety condition:

```nmlt
always(phase == indeterminate implies not enabled(dispatch))
```

The TLA+ comparison already stated the intended condition as
`phase = "indeterminate" => ~ENABLED Dispatch`; v2 brings the NMLT contract
into alignment with it.

## Distinguishing counterexample

The registered `one-shot-replay-regression` control begins in
`phase = indeterminate` with `dispatch` enabled. Taking `dispatch` changes the
phase to `reconciled`, where the action is disabled. This comparison is scoped
to the Phase 3 execution profile, in which `next` ranges over declared action
successors and an identity-stutter successor is supplied only for terminal
states.

- The v1 formula accepts the behavior: its `next(...)` consequent observes
  the reconciled successor, where dispatch is no longer enabled.
- The v2 formula refutes the behavior at state 0: dispatch is enabled in the
  current indeterminate state. Its minimal witness therefore has zero
  transitions and records `dispatch_enabled = true` in the initial
  observation.

The control is not a fifth primary mutant. It is a permanent semantic
regression fixture that distinguishes the superseded and corrected formulas
under that action-step/terminal-stutter profile. RFC 0007 and Phase 4 instead
admit identity stutter at every state; with that universal closure the v1
`next` formula is also refuted at state 0, so no cross-profile transport claim
is made.
The executable engine integration test checks both classifications; the
benchmark integrity validator freezes the exact source, corrected property
binding, and expected `refuted` outcome.

## Identity cascade

The correction changes the canonical `NoBlindReplay` property, the reference
and blind-replay sources, the blind-replay expected witness and affected
intent capsules, and the suite source set. Every affected identifier is
recomputed under the existing v1 artifact-identity domains; the `v2` label is
the suite version, not a new hashing algorithm. After engine stabilization,
the persisted v2 execution results were regenerated and rebound through all
five intent capsules because those results also bind exact engine and
executable identities.
