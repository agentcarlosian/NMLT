# Provider-attempt comparative encodings

This directory freezes one behavioral problem in NMLT, TLA+, Quint, and P.
The comparison is semantic, not a line-count contest: each model represents
authorization, arming, at-most-once dispatch, response evaluation, terminal
selection/rejection, and an indeterminate response that cannot be replayed.

## Common state and claims

| Concept | Common interpretation |
|---|---|
| `phase` | Lifecycle from proposed through a terminal outcome |
| `bound` | The attempt is bound to an authorized request |
| `armed` | Dispatch is explicitly enabled by authority |
| `dispatch_count` | Monotone count, restricted to zero or one |
| `response_intact` | Evaluation is bound to an intact response |
| `evaluation_passed` | Evaluation accepted the response |
| C-PA-01 | A dispatch implies `bound` and `armed` |
| C-PA-02 | A selected result has intact, passing evidence |
| C-PA-03 | Dispatch happens at most once |
| C-PA-04 | An indeterminate outcome cannot enable another dispatch |

The NMLT source of record is
`../../examples/technicus/provider_attempt.nmlt`. The comparison models add
explicit pass/fail terminal actions so all four common claims are exercised.

## Encodings

- `tla/ProviderAttempt.tla` plus `.cfg`: relational next-state model checked by
  TLC, including stuttering in `Spec`.
- `quint/provider_attempt.qnt`: typed action model using Quint's action modes,
  delayed assignments, and nondeterministic `any` block.
- `p/`: executable asynchronous state machine that announces snapshots to a
  synchronous safety monitor; the P checker explores nondeterministic response
  outcomes.

Run `tools/validate_comparisons.sh`. It records exact tools when available and
fails if an installed tool rejects its model. The repository CI validates NMLT
and Quint; TLA+ and P are additionally validated by the pinned commands in
`validation.json` because their toolchains are not installed by default.

## Semantic differences that remain visible

- TLA+ represents the system directly as a stuttering-closed behavior.
- Quint distinguishes state, action, and temporal modes and gives the state a
  static type.
- P represents an implementation-facing actor with queued events; properties
  observe announced snapshots rather than arbitrary state predicates.
- NMLT's proposed distinction is a behavior-indexed property plus a linear
  `Once<ProviderEffect>` capability and evidence identity. Those features are
  still candidate semantics, not implemented verification.

No comparison result establishes that one language is generally superior.
It establishes only that the same frozen protocol and claim set can be stated
and checked under each tool's documented semantics.
