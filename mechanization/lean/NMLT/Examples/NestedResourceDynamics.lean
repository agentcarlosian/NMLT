import NMLT.Behavior.ResourceDynamics
import NMLT.Examples.ResourceWorldTransfer

namespace NMLT.Examples.NestedResourceDynamics

open NMLT.Behavior NMLT.Behavior.ResourceBehavior NMLT.Behavior.ResourceWorld
open NMLT.Examples.ResourceWorldTransfer

/-- The monitor has an observation-silent, real control transition. Its hidden
classification does not, by itself, erase that transition or assert refinement. -/
def monitor : Behavior Unit Capability ContractFact GradeAtom Unit where
  State := Bool
  init := fun state => state = false
  step := fun before _ after => before = false ∧ after = true
  observe := fun _ => ()
  hidden := fun _ => True
  direction := fun _ => .internal
  payload := fun _ => "Unit"
  owns := fun _ => False
  resources := fun _ => ResourceProfile.empty

def dynamicSender := ResourceDynamics.ofBehavior sender Owner.sender
  (fun world => world.Owns .sender .permit)

def dynamicReceiver := ResourceDynamics.ofBehavior receiver Owner.receiver
  (fun world => ¬ world.Owns .receiver .permit)

/-- Reusing the receiver identity is harmless here: the monitor has no authority
requirements/effects and no connected boundary action. -/
def dynamicMonitor := ResourceDynamics.ofBehavior monitor Owner.receiver (fun _ => True)

def inner := ResourceDynamics.parallel dynamicSender dynamicMonitor (fun _ _ => False)

def outerConnection : ProductAction SenderAction Unit → ReceiverAction → Prop
  | .left .send, .receive => True
  | _, _ => False

def nested := ResourceDynamics.parallel inner dynamicReceiver outerConnection

theorem inner_formed : ResourceDynamics.Composable dynamicSender dynamicMonitor
    (fun _ _ => False) where
  interface := {
    directions := fun impossible => False.elim impossible
    payloads := fun impossible => False.elim impossible
    hiddenLeft := fun impossible => False.elim impossible
    hiddenRight := fun impossible => False.elim impossible
    capabilities := fun _ _ impossible => False.elim impossible
    resources := fun impossible => False.elim impossible
  }
  participants := fun impossible => False.elim impossible

theorem nested_formed : ResourceDynamics.Composable inner dynamicReceiver outerConnection where
  interface := {
    directions := by
      intro action peerAction connected
      cases action with
      | left senderAction =>
          exact senderReceiverComposable.directions
            (leftAction := senderAction) (rightAction := peerAction) (by trivial)
      | right _ => exact False.elim connected
      | sync _ _ => exact False.elim connected
    payloads := by
      intro action peerAction connected
      cases action with
      | left senderAction =>
          exact senderReceiverComposable.payloads
            (leftAction := senderAction) (rightAction := peerAction) (by trivial)
      | right _ => exact False.elim connected
      | sync _ _ => exact False.elim connected
    hiddenLeft := by
      intro action peerAction connected
      cases action with
      | left senderAction =>
          exact senderReceiverComposable.hiddenLeft
            (leftAction := senderAction) (rightAction := peerAction) (by trivial)
      | right _ => exact False.elim connected
      | sync _ _ => exact False.elim connected
    hiddenRight := fun _ impossible => False.elim impossible
    capabilities := fun _ _ impossible => False.elim impossible
    resources := by
      intro action peerAction connected
      cases action with
      | left senderAction =>
          exact senderReceiverComposable.resources
            (leftAction := senderAction) (rightAction := peerAction) (by trivial)
      | right _ => exact False.elim connected
      | sync _ _ => exact False.elim connected
  }
  participants := by
    intro action peerAction connected
    cases action with
    | left senderAction =>
        cases senderAction
        cases peerAction
        exact ⟨.sender, .receiver, senderProfile, receiverProfile, rfl, rfl, by decide⟩
    | right _ => exact False.elim connected
    | sync _ _ => exact False.elim connected

def initial : nested.State := ⟨((false, false), false), before⟩
def transferred : nested.State := ⟨((true, false), true), after⟩
def monitored : nested.State := ⟨((true, true), true), after⟩

theorem initialized : nested.init initial := by
  refine ⟨⟨⟨rfl, rfl⟩, rfl⟩, ⟨rfl, trivial⟩, ?_⟩
  intro owns
  simp [AuthorityWorld.Owns, initial, before] at owns

theorem open_port_retains_leaf :
    inner.control.direction (.left .send) = .output ∧
    inner.control.payload (.left .send) = "Once<Unit>" ∧
    inner.effect (.left .send) = .atomic Owner.sender senderProfile :=
  ⟨rfl, rfl, rfl⟩

theorem peer_hiding_retained : nested.control.hidden (.left (.right ())) := trivial

theorem right_open_port_retains_leaf :
    (ResourceDynamics.parallel dynamicMonitor dynamicReceiver (fun _ _ => False)).control.direction
      (.right .receive) = .input ∧
    (ResourceDynamics.parallel dynamicMonitor dynamicReceiver (fun _ _ => False)).control.payload
      (.right .receive) = "Once<Unit>" ∧
    (ResourceDynamics.parallel dynamicMonitor dynamicReceiver (fun _ _ => False)).effect
      (.right .receive) = .atomic Owner.receiver receiverProfile :=
  ⟨rfl, rfl, rfl⟩

/-- The inner open send cannot run alone; it must receive an actual peer. -/
theorem inner_open_transfer_requires_peer {before after : inner.State} :
    ¬ inner.step before (.left .send) after := by
  intro step
  exact step.2.noTransfer .permit trivial

/-- The same deferred send is executable when the outer receiver is connected. -/
theorem transfer : nested.step initial (.sync (.left .send) .receive) transferred := by
  refine ⟨ResourceBehavior.ProductStep.synchronize trivial
    (ResourceBehavior.ProductStep.fromLeft ⟨rfl, rfl⟩ (fun _ => id))
    ⟨rfl, rfl⟩, ?_⟩
  exact transferStep

theorem transfer_moves_once
    {before after : nested.State}
    (step : nested.step before (.sync (.left .send) .receive) after) :
    before.authority.Owns Owner.sender .permit ∧
      after.authority.Owns Owner.receiver .permit ∧
      ¬ after.authority.Owns Owner.sender .permit := by
  have effectStep : SyncStep Owner.sender Owner.receiver senderProfile receiverProfile
      before.authority after.authority := step.2
  exact ⟨effectStep.left_transfer_was_owned trivial,
    effectStep.left_transfer_changes_owner trivial,
    effectStep.left_transfer_not_retained trivial⟩

private theorem emptyStep (actor : Owner) (world : AuthorityWorld Capability Owner) :
    LocalStep actor (ResourceProfile.empty : ResourceProfile Capability ContractFact GradeAtom)
      world world where
  enabled := by constructor <;> simp [ResourceProfile.empty]
  noTransfer := by simp [ResourceProfile.empty]
  noReceive := by simp [ResourceProfile.empty]
  consumed := by simp [ResourceProfile.empty]
  preserved := fun _ _ => rfl

theorem monitor_step : nested.step transferred (.left (.right ())) monitored := by
  refine ⟨ResourceBehavior.ProductStep.fromLeft
    (ResourceBehavior.ProductStep.fromRight ⟨rfl, rfl⟩ (fun _ => id))
    (fun receiverAction => ?_), emptyStep .receiver after⟩
  cases receiverAction
  exact id

theorem monitor_observation_and_authority_unchanged :
    nested.observe transferred = nested.observe monitored := rfl

theorem finite_execution : ResourceDynamics.Path nested initial
    [.sync (.left .send) .receive, .left (.right ())] monitored :=
  .cons transfer (.cons monitor_step (.nil monitored))

theorem reachable : ResourceDynamics.Reachable nested monitored :=
  ⟨initial, _, initialized, finite_execution⟩

/-- A completed inner synchronization also remains executable inside another
product. The outer product inherits its effect; it does not apply it twice. -/
def closedPair := ResourceDynamics.parallel dynamicSender dynamicReceiver connection
def enclosed := ResourceDynamics.parallel closedPair dynamicMonitor (fun _ _ => False)

theorem enclosed_transfer : enclosed.step
    ⟨((false, false), false), before⟩ (.left (.sync .send .receive))
    ⟨((true, true), false), after⟩ := by
  exact ⟨ResourceBehavior.ProductStep.fromLeft
    (ResourceBehavior.ProductStep.synchronize trivial ⟨rfl, rfl⟩ ⟨rfl, rfl⟩)
    (fun _ => id), transferStep⟩

theorem completed_sync_cannot_rendezvous_again
    {third : ResourceDynamics.Effect Capability ContractFact GradeAtom Owner}
    {before after : AuthorityWorld Capability Owner} :
    ¬ (ResourceDynamics.Effect.synchronize (closedPair.effect (.sync .send .receive))
      third).Step before after := id

end NMLT.Examples.NestedResourceDynamics
