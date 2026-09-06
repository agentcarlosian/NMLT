# Approved foundational axioms

The Lean gate rejects `sorry`, `sorryAx`, `admit`, `native_decide`, and locally
declared axioms. NanoDA treats any axiom outside the package allowlist as a hard
error. The package allowlist contains:

- `propext`, for propositional extensionality;
- `Quot.sound`, in quotient reasoning in the retained typed-elaboration
  metatheory and the R1 nested execution example; and
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

The R1 unified declarations `ResourceDynamics.legacy_step_iff`,
`step_projects`, `liftParallel`, `ofWorldRefinement`,
`synchronized_effect_refines`, and `reachable_no_fabrication` report only
`propext`. `parallel_init_iff`, `parallel_observe`, and
`reachable_unique_owner` report no axioms. These names are in the namespace
`NMLT.Behavior.ResourceDynamics`.

`NMLT.Examples.NestedResourceDynamics.finite_execution` reports `propext` and
`Quot.sound`. Its `transfer_moves_once`, `inner_open_transfer_requires_peer`,
and `enclosed_transfer` controls report only `propext`. No R1 behavioral theorem
requires expanding the focused audit's allowlist.

The decoded execution probes `ExecutionClosure.Model.initial_iff`, `step_iff`,
`formed`, `path_owned_origin`, `sync_left_moves_once`, and `sync_right_moves_once`
report only `propext`. So do `ExecutionWitness.Certificate.reachable`,
`no_fabrication`, `unique_owner`, and `owner_origin`, and
`ExecutionLift.simulation` and `simulation_sync_step`. These names are in the
namespace `NMLT.Artifact`. The fresh R1 completion export checks all package
declarations under the unchanged policy; it is not a proof export for each
runtime path-checking invocation.
