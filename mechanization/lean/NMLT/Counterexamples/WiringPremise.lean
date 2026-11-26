import NMLT.Examples.ResourceWorldTransfer

namespace NMLT.Counterexamples.WiringPremise

open NMLT.Behavior.ResourceBehavior
open NMLT.Examples.ResourceWorldTransfer

/-- The only changed input in this experiment is the abstract connection. -/
def disconnected : SenderAction → ReceiverAction → Prop := fun _ _ => False

theorem wiringDiffers : ¬ WiringEquivalent connection disconnected := by
  intro equivalent
  exact (equivalent.connected .send .receive).mp trivial

theorem disconnectedComposable : Composable abstractSender receiver disconnected where
  directions := by
    intro _ _ connected
    exact False.elim connected
  payloads := by
    intro _ _ connected
    exact False.elim connected
  hiddenLeft := by
    intro _ _ connected
    exact False.elim connected
  hiddenRight := by
    intro _ _ connected
    exact False.elim connected
  capabilities := senderReceiverComposable.capabilities
  resources := by
    intro _ _ connected
    exact False.elim connected

/-- The matched case reproduces the existing conditional composition result. -/
theorem matchedWiring_lifts :
    Nonempty (ResourceWeakRefinement
      (parallel sender receiver connection)
      (parallel abstractSender receiver connection)) :=
  liftParallel senderRefinement wiring senderReceiverComposable
    abstractSenderReceiverComposable

theorem concreteInitialAndEnabled :
    (parallel sender receiver connection).init (false, false) ∧
      (parallel sender receiver connection).step (false, false)
        (.sync .send .receive) (true, true) := by
  constructor
  · exact ⟨rfl, rfl⟩
  · exact ProductStep.synchronize trivial ⟨rfl, rfl⟩ ⟨rfl, rfl⟩

theorem abstractInitialAndEnabled :
    (parallel abstractSender receiver disconnected).init (false, false) ∧
      (parallel abstractSender receiver disconnected).step (false, false)
        (.left .send) (true, false) := by
  constructor
  · exact ⟨rfl, rfl⟩
  · exact ProductStep.fromLeft ⟨rfl, rfl⟩ (fun _ connected => connected)

theorem noDisconnectedSync (before after :
    (parallel abstractSender receiver disconnected).State) :
    ¬ (parallel abstractSender receiver disconnected).step before
      (.sync .send .receive) after := by
  intro step
  cases before with
  | mk leftBefore rightBefore =>
    cases after with
    | mk leftAfter rightAfter =>
      cases step with
      | synchronize connected _ _ => exact connected

/-- With the same components but disconnected abstract wiring, the actual
    synchronized step has no abstract match under any state map. This refutes
    the weakened lifting conclusion, beyond rejecting a formation premise. -/
theorem disconnectedWiring_blocksLift :
    ¬ Nonempty (ResourceWeakRefinement
      (parallel sender receiver connection)
      (parallel abstractSender receiver disconnected)) := by
  intro ⟨refinement⟩
  have visible : ¬ (parallel sender receiver connection).hidden
      (.sync .send .receive) := by
    intro hidden
    exact hidden
  have abstractStep := refinement.visibleStep concreteInitialAndEnabled.2 visible
  exact noDisconnectedSync _ _ abstractStep

end NMLT.Counterexamples.WiringPremise
