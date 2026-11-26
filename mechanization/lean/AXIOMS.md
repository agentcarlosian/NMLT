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

The R2 finite safety universe and initialization/preservation theorems in
`NMLT.Artifact.FiniteInvariant`, plus
`NMLT.Artifact.InvariantWitness.Certificate.safe`, report `propext` and
`Quot.sound`. `Counterexample.reachable_violation` reports only `propext`.
They are included in the unchanged focused behavioral audit. The executable
predicate parser and JSON checker remain in the package's runtime trust boundary.

The R4 wiring-premise control in `NMLT.Counterexamples.WiringPremise` reports
only `propext` for `disconnectedComposable` and `matchedWiring_lifts`.
`wiringDiffers`, `concreteInitialAndEnabled`, `abstractInitialAndEnabled`,
`noDisconnectedSync`, and
`disconnectedWiring_blocksLift` report no axioms. All are included in the
focused behavioral audit.

The broader R4/R5 audit explicitly includes
`Discovery.CompositionPremises.liftWithVisibleConnections`, both initial
enabledness witnesses, all `case00` through `case15` declarations, the finite
construction row/control declarations, `Evaluation.Corpus.bundle`, and
`Evaluation.Discovery.largerObjects`. They are imported into `NMLT`, so the
package's full independent export includes them. No allowlist change is made.
The experimental Python summaries themselves provide no proof assurance.

The second-use example `Discovery.NestedReuse` adds explicit probes for its
nested refinement, admitted initial/enabled witness, lifted step, resource
bound, formation rejection and undersized-bound refutation. It uses the same
focused allowlist and full independent package export. It asserts no dynamic
authority theorem or accepted source product.

The workflow-job bridge `NMLT.Behavior.WorkflowJobBridge` connects the bounded
affine job-slot lifecycle (`dispatch`/`start` -> `settle` -> `deliver`/`collect`,
generation-pinned non-interference, double-collect rejection, and no-fabrication)
into `NMLT.Behavior.ResourceDynamics`. Its theorems (`bridge_composable`,
`initial_valid`, `start_transfers_handle`, `collect_returns_handle`,
`stale_delivery_rejected`, `stale_start_rejected`, `double_collect_rejected`,
`workflow_cannot_fabricate_handle`, `host_cannot_dispatch_alone`,
`generation_zero_cycle`, `next_cycle_reachable`, `reachable_handle_owner_unique`,
and `reachable_no_fabrication`) are included in the focused behavioral audit under
the unchanged `propext`/`Quot.sound` allowlist (reporting no axioms or at most
`propext`). This models the abstract slot-lifecycle protocol; it does not prove
Rust implementation or OS process correspondence.
