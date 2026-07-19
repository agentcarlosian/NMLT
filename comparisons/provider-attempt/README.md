# Provider-attempt comparative encodings

This directory freezes one behavioral problem in NMLT,
[TLA+](https://lamport.azurewebsites.net/tla/tla.html),
[Quint](https://quint-lang.org/), and [P](https://p-org.github.io/P/).
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
`../../examples/technicus/provider_attempt.nmlt`. The TLA+, Quint, and P
encodings add explicit pass/fail terminal actions. The canonical NMLT reference
currently has only bind, arm, dispatch, and lost-response actions; consequently
its selection property is checked over the complete reachable graph but is
vacuous in this fixture because no action reaches `selected`. C-PA-03 is
enforced by the zero-count dispatch guard and affine provider capability and is
covered by the reachable graph, but it is not a fourth named NMLT property.

## Encodings

- `tla/ProviderAttempt.tla` plus `.cfg`: relational next-state model checked by
  TLC, including stuttering in `Spec`.
- `quint/provider_attempt.qnt`: typed action model using Quint's action modes,
  delayed assignments, and nondeterministic `any` block.
- `p/`: executable asynchronous state machine that announces snapshots to a
  synchronous safety monitor; the P checker explores nondeterministic response
  outcomes.

Run `tools/validate_comparisons.sh`. It runs NMLT's bounded explicit-state
checker, rejects a truncated frontier or any non-`model_checked` declared
property, and runs Quint's parser and typechecker. Before invoking any model
tool, it unconditionally verifies the exact SHA-256 bindings recorded in
`validation.json` for the TLA+ `.tla` and `.cfg`, the Quint `.qnt`, and all four
P source/project files. It also runs TLC when `TLA2TOOLS_JAR` names a local tool
jar and compiles/systematically tests P when the `p` executable is installed.
The validation record distinguishes byte identity from execution evidence: all
three comparison source sets are byte-bound, while the corrected P source still
awaits a current P/.NET run. The earlier external run predates the enabledness
correction and is not evidence for these bytes.

## Semantic differences that remain visible

- TLA+ represents the system directly as a stuttering-closed behavior.
- Quint distinguishes state, action, and temporal modes and gives the state a
  static type.
- P represents an implementation-facing actor with queued events; properties
  observe announced snapshots rather than arbitrary state predicates.
- NMLT's executable Phase 2/3 fragment checks the behavior-indexed properties
  and enforces consumption of the affine `Once<ProviderEffect>` capability.
  The comparison exhausts this fixture's reachable frontier within the
  reported checker ceilings; it is not a proof of the full planned language,
  the surface-to-core translation, or general compiler correctness.

No comparison result establishes that one language is generally superior.
It establishes only that closely matched frozen protocols and claims can be
stated under each tool's documented semantics. The current validation strength
is not uniform: NMLT and TLC explore these finite models, Quint is
parse/typecheck only, and the current P encoding is unvalidated because P/.NET
was unavailable. The NMLT selection-claim vacuity and non-declarative C-PA-03
encoding noted above are explicit remaining parity gaps.
