# Approved foundational axioms

The Lean gate rejects `sorry`, `sorryAx`, `admit`, `native_decide`, and locally
declared axioms. Its focused theorem audit permits only these Lean foundations:

- `propext`, for propositional extensionality;
- `Quot.sound`, in the retained typed-elaboration metatheory's quotient
  reasoning.

`NMLT.Behavior.ResourceBehavior.liftParallel` and
`NMLT.Artifact.SemanticClosure.Certificate.lifted` currently report only
`propext`. `Classical.choice` and `Lean.trustCompiler` are not approved for
these behavioral theorems.

The dynamic authority declarations
`NMLT.Behavior.ResourceWorld.ProductStep.synchronized_left_transfer_moves_once`
and `NMLT.Behavior.ResourceWorld.SyncStep.owner_after_is_explained` report no
axioms. The concrete control
`NMLT.Examples.ResourceWorldTransfer.permit_moves_exactly_once` and the
synchronized-step refinement declarations
`NMLT.Behavior.ResourceWorld.liftSynchronized` and
`NMLT.Examples.ResourceWorldTransfer.permit_transfer_lifts_dynamically`, plus
the artifact-bound
`NMLT.Artifact.SemanticClosure.Certificate.liftedSynchronized`, report only
`propext`.
