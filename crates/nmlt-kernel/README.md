# `nmlt-kernel`

`nmlt-kernel` is the retained independent validator for NMLT's typed
elaboration boundary. It replays the finite M9 formation and typing judgments
for exact resolved HIR, explicit typed core, and an untrusted elaboration
certificate. Only successful replay constructs `CheckedProgram`.

The name is historical. This crate is not the semantic prover, does not define
behavioral meaning, and cannot issue refinement, safety, liveness, model-check,
or evidence claims. Normative behavior semantics live in Lean under
`NMLT.Behavior.ResourceBehavior`; `behavior-core-v1` acceptance belongs to the
Lean artifact decoder.

The retained validator exists to keep an independently checked boundary between
the Rust elaborator and consumers of the ordinary typed core. No
source-to-Lean compiler-correctness theorem is claimed.
