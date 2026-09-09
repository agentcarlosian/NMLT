# `nmlt-eval`

`nmlt-eval` is NMLT's bounded reference explorer for canonical behavioral
artifacts. It is intended for language design, debugging, and operational
cross-checking.

State values support the artifact's finite `Bool`, `Unit`, and declared enum
types. Initializers must be closed; guards must be Boolean; simultaneous
updates preserve their declared field types. Invalid term types and unknown
enum constructors return an evaluation error. `EvalState.values` stores typed
`EvalValue` values; their display forms are `true`/`false`, `()`, and
`Enum.Constructor`.

V1 retains its earlier open reference interpretation. V2 blocks unmatched
transfers/receives, checks reception freshness and affine effects, and requires
current ownership for later consume/retransfer actions. `execution_path` emits
an untrusted binary path with control indices and complete authority states;
only the separate Lean execution checker can accept its semantic claims.

Every result has `assurance: none`. The crate does not invoke Lean and
cannot issue proof, model-check, evidence, or runtime-authorization claims.

R2's `execute` API steps directly from initialization with a required bound,
using the same prepared successors as `explore`. It supports deterministic
first-enabled scheduling or an exact label sequence and returns typed full
states, grades, and a precise stop outcome. It does not enumerate the graph
before running. Its cumulative grades are model annotations, and it has no
host adapters. See [RFC 0017](../../rfcs/0017-finite-local-run-and-replay.md).
