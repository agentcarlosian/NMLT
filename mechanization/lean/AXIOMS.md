# Approved foundational axioms

The Lean gate rejects `sorry`, `sorryAx`, `admit`, `native_decide`, and locally
declared axioms. NanoDA treats any axiom outside the package allowlist as a hard
error. The package allowlist contains:

- `propext`, for propositional extensionality;
- `Quot.sound`, in the retained typed-elaboration metatheory's quotient
  reasoning; and
- `Classical.choice`, used by executable JSON decoding and finite premise
  decision procedures in the artifact modules.

The focused behavioral theorem audit is stricter: it accepts only `propext` and
`Quot.sound` and rejects `Classical.choice`.

`NMLT.Behavior.ResourceBehavior.liftParallel` and
`NMLT.Artifact.SemanticClosure.Certificate.lifted` currently report only
`propext`. `Classical.choice` is not accepted for these behavioral theorems,
and `Lean.trustCompiler` is not approved anywhere in the package.

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

The complete one-step dynamic declarations
`NMLT.Behavior.ResourceWorld.liftProductSteps`,
`NMLT.Examples.ResourceWorldTransfer.dynamicallyMatchedProductTransfer`, and
`NMLT.Artifact.SemanticClosure.Certificate.liftedStep` also report only
`propext`. The pointwise hidden-world preservation lemma and the profile-level
hidden-consumption control report no axioms; the concrete world-change control
reports only `propext`.
