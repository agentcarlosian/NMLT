import NMLT.Behavior.ResourceDynamics

namespace NMLT.Behavior.WorkflowJobBridge

open NMLT.Behavior NMLT.Behavior.ResourceBehavior NMLT.Behavior.ResourceWorld

/-- Principals in the bounded affine workflow/host job-slot protocol. -/
inductive Owner where
  | host
  | workflow
deriving DecidableEq

/-- Nominal affine capability representing live ownership of one job-slot handle. -/
inductive Capability where
  | slotHandle
deriving DecidableEq

/-- Rely/guarantee facts linking dispatch, settlement, and handle return. -/
inductive JobFact where
  | bound
  | dispatched
  | settled
  | returning
deriving DecidableEq

/-- Resource grade atom accounting for each bounded job-slot attempt. -/
inductive JobGrade where
  | attempt
deriving DecidableEq

/-- Bounded single-slot host control phases indexed by monotonic generation. -/
inductive HostPhase where
  | ready (nextGen : Nat)
  | running (gen : Nat)
  | finished (gen : Nat)
deriving DecidableEq

/-- Workflow control phases holding at most one affine handle at a time. -/
inductive WorkflowPhase where
  | idle
  | awaiting (gen : Nat)
deriving DecidableEq

/-- Host-side lifecycle transitions for one bounded slot. -/
inductive HostAction where
  | dispatch (gen : Nat)
  | settle (gen : Nat)
  | deliver (gen : Nat)
deriving DecidableEq

/-- Workflow-side affine handle operations (`job_start_*` and `job_collect`). -/
inductive WorkflowAction where
  | start (gen : Nat)
  | collect (gen : Nat)
deriving DecidableEq

def hostDispatchProfile : ResourceProfile Capability JobFact JobGrade where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => True
  receives := fun _ => False
  grade := ⟨fun _ => 1⟩
  relies
    | .bound => True
    | _ => False
  guarantees
    | .dispatched => True
    | _ => False

def workflowStartProfile : ResourceProfile Capability JobFact JobGrade where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => True
  grade := Grade.zero
  relies
    | .dispatched => True
    | _ => False
  guarantees
    | .bound => True
    | _ => False

def hostDeliverProfile : ResourceProfile Capability JobFact JobGrade where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => True
  grade := Grade.zero
  relies
    | .returning => True
    | _ => False
  guarantees
    | .settled => True
    | _ => False

def workflowCollectProfile : ResourceProfile Capability JobFact JobGrade where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => True
  receives := fun _ => False
  grade := Grade.zero
  relies
    | .settled => True
    | _ => False
  guarantees
    | .returning => True
    | _ => False

def startCompatible :
    SynchronizationCompatible hostDispatchProfile workflowStartProfile where
  transfer := fun _ => Iff.rfl
  noReverseTransfer := fun _ => Iff.rfl
  senderRely := by
    intro fact relied
    cases fact <;> exact relied
  receiverRely := by
    intro fact relied
    cases fact <;> exact relied

def collectCompatible :
    SynchronizationCompatible workflowCollectProfile hostDeliverProfile where
  transfer := fun _ => Iff.rfl
  noReverseTransfer := fun _ => Iff.rfl
  senderRely := by
    intro fact relied
    cases fact <;> exact relied
  receiverRely := by
    intro fact relied
    cases fact <;> exact relied

def hostBehavior : Behavior HostAction Capability JobFact JobGrade HostPhase where
  State := HostPhase
  init := fun state => state = .ready 0
  step := fun before action after =>
    match action with
    | .dispatch gen => before = .ready gen ∧ after = .running gen
    | .settle gen => before = .running gen ∧ after = .finished gen
    | .deliver gen => before = .finished gen ∧ after = .ready (gen + 1)
  observe := id
  hidden := fun _ => False
  direction
    | .dispatch _ => .output
    | .settle _ => .internal
    | .deliver _ => .input
  payload
    | .dispatch _ => "JobHandle"
    | .settle _ => "Unit"
    | .deliver _ => "JobHandle"
  owns := fun _ => True
  resources
    | .dispatch _ => hostDispatchProfile
    | .settle _ => ResourceProfile.empty
    | .deliver _ => hostDeliverProfile

def workflowBehavior : Behavior WorkflowAction Capability JobFact JobGrade WorkflowPhase where
  State := WorkflowPhase
  init := fun state => state = .idle
  step := fun before action after =>
    match action with
    | .start gen => before = .idle ∧ after = .awaiting gen
    | .collect gen => before = .awaiting gen ∧ after = .idle
  observe := id
  hidden := fun _ => False
  direction
    | .start _ => .input
    | .collect _ => .output
  payload
    | .start _ => "JobHandle"
    | .collect _ => "JobHandle"
  owns := fun _ => False
  resources
    | .start _ => workflowStartProfile
    | .collect _ => workflowCollectProfile

def dynamicHost := ResourceDynamics.ofBehavior hostBehavior Owner.host
  (fun world => world.Owns .host .slotHandle)

def dynamicWorkflow := ResourceDynamics.ofBehavior workflowBehavior Owner.workflow
  (fun world => ¬ world.Owns .workflow .slotHandle)

/-- Generation-pinned wiring between host dispatch/deliver and workflow start/collect. -/
def bridgeConnection : HostAction → WorkflowAction → Prop
  | .dispatch hostGen, .start workflowGen => hostGen = workflowGen
  | .deliver hostGen, .collect workflowGen => hostGen = workflowGen
  | _, _ => False

/-- Unified `ResourceDynamics.Behavior` modeling the bounded affine job-slot protocol. -/
def bridge := ResourceDynamics.parallel dynamicHost dynamicWorkflow bridgeConnection

theorem bridge_composable :
    ResourceDynamics.Composable dynamicHost dynamicWorkflow bridgeConnection where
  interface := {
    directions := by
      intro leftAction rightAction connected
      cases leftAction with
      | dispatch _ =>
          cases rightAction with
          | start _ => trivial
          | collect _ => exact False.elim connected
      | settle _ => exact False.elim connected
      | deliver _ =>
          cases rightAction with
          | start _ => exact False.elim connected
          | collect _ => trivial
    payloads := by
      intro leftAction rightAction connected
      cases leftAction with
      | dispatch _ =>
          cases rightAction with
          | start _ => rfl
          | collect _ => exact False.elim connected
      | settle _ => exact False.elim connected
      | deliver _ =>
          cases rightAction with
          | start _ => exact False.elim connected
          | collect _ => rfl
    hiddenLeft := fun _ hidden => False.elim hidden
    hiddenRight := fun _ hidden => False.elim hidden
    capabilities := fun _ _ rightOwns => False.elim rightOwns
    resources := by
      intro leftAction rightAction connected
      cases leftAction with
      | dispatch _ =>
          cases rightAction with
          | start _ => exact ⟨fun _ => startCompatible, fun impossible => by cases impossible⟩
          | collect _ => exact False.elim connected
      | settle _ => exact False.elim connected
      | deliver _ =>
          cases rightAction with
          | start _ => exact False.elim connected
          | collect _ => exact ⟨fun impossible => by cases impossible, fun _ => collectCompatible⟩
  }
  participants := by
    intro leftAction rightAction connected
    cases leftAction with
    | dispatch _ =>
        cases rightAction with
        | start _ =>
            exact ⟨.host, .workflow, hostDispatchProfile, workflowStartProfile,
              rfl, rfl, fun eq => Owner.noConfusion eq⟩
        | collect _ => exact False.elim connected
    | settle _ => exact False.elim connected
    | deliver _ =>
        cases rightAction with
        | start _ => exact False.elim connected
        | collect _ =>
            exact ⟨.host, .workflow, hostDeliverProfile, workflowCollectProfile,
              rfl, rfl, fun eq => Owner.noConfusion eq⟩

def hostWorld : AuthorityWorld Capability Owner :=
  ⟨fun _ => some .host⟩

def workflowWorld : AuthorityWorld Capability Owner :=
  ⟨fun _ => some .workflow⟩

private theorem startSyncStep :
    SyncStep Owner.host Owner.workflow hostDispatchProfile workflowStartProfile
      hostWorld workflowWorld where
  enabled := {
    ownersDistinct := fun eq => Owner.noConfusion eq
    leftEnabled := {
      requires := fun _ impossible => False.elim impossible
      consumes := fun _ impossible => False.elim impossible
      transfers := fun capability _ => by cases capability; rfl
      receivesFresh := fun _ impossible => False.elim impossible
      consumeTransfer := fun _ impossible _ => False.elim impossible
      consumeReceive := fun _ impossible _ => False.elim impossible
      transferReceive := fun _ _ impossible => False.elim impossible
    }
    rightEnabled := {
      requires := fun _ impossible => False.elim impossible
      consumes := fun _ impossible => False.elim impossible
      transfers := fun _ impossible => False.elim impossible
      receivesFresh := fun capability _ owns => by
        cases capability
        exact Owner.noConfusion (Option.some.inj owns)
      consumeTransfer := fun _ impossible _ => False.elim impossible
      consumeReceive := fun _ impossible _ => False.elim impossible
      transferReceive := fun _ impossible _ => False.elim impossible
    }
    compatible := startCompatible
  }
  consumedLeft := fun _ impossible => False.elim impossible
  consumedRight := fun _ impossible => False.elim impossible
  transferredLeft := fun capability _ => by cases capability; rfl
  transferredRight := fun _ impossible => False.elim impossible
  preserved := fun _ _ _ notTransferred _ => False.elim (notTransferred trivial)

private theorem collectSyncStep :
    SyncStep Owner.host Owner.workflow hostDeliverProfile workflowCollectProfile
      workflowWorld hostWorld where
  enabled := {
    ownersDistinct := fun eq => Owner.noConfusion eq
    leftEnabled := {
      requires := fun _ impossible => False.elim impossible
      consumes := fun _ impossible => False.elim impossible
      transfers := fun _ impossible => False.elim impossible
      receivesFresh := fun capability _ owns => by
        cases capability
        exact Owner.noConfusion (Option.some.inj owns)
      consumeTransfer := fun _ impossible _ => False.elim impossible
      consumeReceive := fun _ impossible _ => False.elim impossible
      transferReceive := fun _ impossible _ => False.elim impossible
    }
    rightEnabled := {
      requires := fun _ impossible => False.elim impossible
      consumes := fun _ impossible => False.elim impossible
      transfers := fun capability _ => by cases capability; rfl
      receivesFresh := fun _ impossible => False.elim impossible
      consumeTransfer := fun _ impossible _ => False.elim impossible
      consumeReceive := fun _ impossible _ => False.elim impossible
      transferReceive := fun _ _ impossible => False.elim impossible
    }
    compatible := collectCompatible.symm
  }
  consumedLeft := fun _ impossible => False.elim impossible
  consumedRight := fun _ impossible => False.elim impossible
  transferredLeft := fun _ impossible => False.elim impossible
  transferredRight := fun capability _ => by cases capability; rfl
  preserved := fun _ _ _ _ notTransferred => False.elim (notTransferred trivial)

private theorem settleLocalStep (world : AuthorityWorld Capability Owner) :
    LocalStep Owner.host ResourceProfile.empty world world where
  enabled := {
    requires := fun _ impossible => False.elim impossible
    consumes := fun _ impossible => False.elim impossible
    transfers := fun _ impossible => False.elim impossible
    receivesFresh := fun _ impossible => False.elim impossible
    consumeTransfer := fun _ impossible _ => False.elim impossible
    consumeReceive := fun _ impossible _ => False.elim impossible
    transferReceive := fun _ impossible _ => False.elim impossible
  }
  noTransfer := fun _ impossible => impossible
  noReceive := fun _ impossible => impossible
  consumed := fun _ impossible => False.elim impossible
  preserved := fun _ _ => rfl

def initialState : bridge.State :=
  ⟨(.ready 0, .idle), hostWorld⟩

def runningZeroState : bridge.State :=
  ⟨(.running 0, .awaiting 0), workflowWorld⟩

def finishedZeroState : bridge.State :=
  ⟨(.finished 0, .awaiting 0), workflowWorld⟩

def nextCycleState : bridge.State :=
  ⟨(.ready 1, .idle), hostWorld⟩

theorem initial_valid : bridge.init initialState :=
  ⟨⟨rfl, rfl⟩, rfl, fun owns => Owner.noConfusion (Option.some.inj owns)⟩

/-- Starting a job moves handle ownership from `host` to `workflow` without retention. -/
theorem start_transfers_handle
    {gen : Nat} {before after : bridge.State}
    (step : bridge.step before (.sync (.dispatch gen) (.start gen)) after) :
    before.authority.Owns .host .slotHandle ∧
      after.authority.Owns .workflow .slotHandle ∧
      ¬ after.authority.Owns .host .slotHandle := by
  have effectStep : SyncStep .host .workflow hostDispatchProfile workflowStartProfile
      before.authority after.authority := step.2
  exact ⟨effectStep.left_transfer_was_owned trivial,
    effectStep.left_transfer_changes_owner trivial,
    effectStep.left_transfer_not_retained trivial⟩

/-- Collecting a settled job returns handle ownership from `workflow` to `host`. -/
theorem collect_returns_handle
    {gen : Nat} {before after : bridge.State}
    (step : bridge.step before (.sync (.deliver gen) (.collect gen)) after) :
    before.authority.Owns .workflow .slotHandle ∧
      after.authority.Owns .host .slotHandle ∧
      ¬ after.authority.Owns .workflow .slotHandle := by
  have effectStep : SyncStep .host .workflow hostDeliverProfile workflowCollectProfile
      before.authority after.authority := step.2
  exact ⟨effectStep.right_transfer_was_owned trivial,
    effectStep.right_transfer_changes_owner trivial,
    effectStep.right_transfer_not_retained trivial⟩

/-- Generation pinning prevents a stale host delivery from settling a mismatched handle. -/
theorem stale_delivery_rejected
    {hostGen workflowGen : Nat} {before after : bridge.State}
    (mismatch : hostGen ≠ workflowGen) :
    ¬ bridge.step before (.sync (.deliver hostGen) (.collect workflowGen)) after := by
  intro step
  cases step.1 with
  | synchronize connected _ _ => exact mismatch connected

/-- Generation pinning also prevents starting a workflow handle at a mismatched generation. -/
theorem stale_start_rejected
    {hostGen workflowGen : Nat} {before after : bridge.State}
    (mismatch : hostGen ≠ workflowGen) :
    ¬ bridge.step before (.sync (.dispatch hostGen) (.start workflowGen)) after := by
  intro step
  cases step.1 with
  | synchronize connected _ _ => exact mismatch connected

/-- Affine handle return makes double collection impossible without a fresh dispatch. -/
theorem double_collect_rejected
    {gen nextGen : Nat} {before after final : bridge.State}
    (first : bridge.step before (.sync (.deliver gen) (.collect gen)) after) :
    ¬ bridge.step after (.sync (.deliver nextGen) (.collect nextGen)) final := by
  intro second
  exact (collect_returns_handle first).2.2 (collect_returns_handle second).1

/-- The workflow cannot fabricate a handle without synchronizing with the host. -/
theorem workflow_cannot_fabricate_handle
    {gen : Nat} {before after : bridge.State} :
    ¬ bridge.step before (.right (.start gen)) after := by
  intro step
  exact step.2.noReceive .slotHandle trivial

/-- The host cannot transfer slot ownership without a synchronized workflow receiver. -/
theorem host_cannot_dispatch_alone
    {gen : Nat} {before after : bridge.State} :
    ¬ bridge.step before (.left (.dispatch gen)) after := by
  intro step
  exact step.2.noTransfer .slotHandle trivial

theorem step_start_zero :
    bridge.step initialState (.sync (.dispatch 0) (.start 0)) runningZeroState :=
  ⟨ResourceBehavior.ProductStep.synchronize rfl ⟨rfl, rfl⟩ ⟨rfl, rfl⟩, startSyncStep⟩

theorem step_settle_zero :
    bridge.step runningZeroState (.left (.settle 0)) finishedZeroState :=
  ⟨ResourceBehavior.ProductStep.fromLeft ⟨rfl, rfl⟩
    (fun rightAction connected => by cases rightAction <;> exact connected),
    settleLocalStep workflowWorld⟩

theorem step_collect_zero :
    bridge.step finishedZeroState (.sync (.deliver 0) (.collect 0)) nextCycleState :=
  ⟨ResourceBehavior.ProductStep.synchronize rfl ⟨rfl, rfl⟩ ⟨rfl, rfl⟩, collectSyncStep⟩

/-- Concrete initialized execution of generation 0 (`start` -> `settle` -> `collect`),
advancing the host slot cleanly to generation 1. -/
theorem generation_zero_cycle :
    ResourceDynamics.Path bridge initialState
      [.sync (.dispatch 0) (.start 0), .left (.settle 0), .sync (.deliver 0) (.collect 0)]
      nextCycleState :=
  .cons step_start_zero (.cons step_settle_zero (.cons step_collect_zero (.nil nextCycleState)))

theorem next_cycle_reachable : ResourceDynamics.Reachable bridge nextCycleState :=
  ⟨initialState, _, initial_valid, generation_zero_cycle⟩

theorem reachable_handle_owner_unique
    {state : bridge.State} {left right : Owner}
    (reachable : ResourceDynamics.Reachable bridge state)
    (leftOwns : state.authority.Owns left .slotHandle)
    (rightOwns : state.authority.Owns right .slotHandle) :
    left = right :=
  ResourceDynamics.reachable_unique_owner reachable leftOwns rightOwns

theorem reachable_no_fabrication
    {state : bridge.State}
    (initiallyVacant : ∀ initial, bridge.init initial → initial.authority.Vacant .slotHandle)
    (reachable : ResourceDynamics.Reachable bridge state) :
    state.authority.Vacant .slotHandle :=
  ResourceDynamics.reachable_no_fabrication initiallyVacant reachable

end NMLT.Behavior.WorkflowJobBridge
