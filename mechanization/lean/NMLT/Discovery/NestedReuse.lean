import NMLT.Discovery.CompositionPremises

/-! A second use of the visibility lemma: nested synchronization with a monitor.
This is a raw static-product example, deliberately outside formation policy.
It is neither a source artifact nor a dynamic authority/liveness theorem. -/
namespace NMLT.Discovery.NestedReuse

open NMLT.Behavior.ResourceBehavior
open NMLT.Examples.ResourceWorldTransfer
open NMLT.Discovery.CompositionPremises (liftWithVisibleConnections)

def monitor : Behavior Unit Capability ContractFact GradeAtom Bool where
  State := Bool
  init := fun state => state = false
  step := fun before _ after => before = false ∧ after = true
  observe := id
  hidden := fun _ => False
  direction := fun _ => .internal
  payload := fun _ => "Unit"
  owns := fun _ => False
  resources := fun _ => ResourceProfile.empty

def inner := parallel sender receiver connection
def innerAbstract := parallel abstractSender receiver connection

def innerRefinement : ResourceWeakRefinement inner innerAbstract :=
  liftWithVisibleConnections senderRefinement ⟨fun _ _ => Iff.rfl⟩
    (fun _ hidden => hidden)

def monitorConnection : ProductAction SenderAction ReceiverAction → Unit → Prop
  | .sync _ _, _ => True
  | _, _ => False

def concrete := parallel inner monitor monitorConnection
def abstract := parallel innerAbstract monitor monitorConnection

/-- Reuse the same general lemma at a second, nested boundary. -/
def refinement : ResourceWeakRefinement concrete abstract :=
  liftWithVisibleConnections innerRefinement ⟨fun _ _ => Iff.rfl⟩ (by
    intro action peer connected hidden
    cases action with
    | left action => exact connected
    | right action => exact hidden
    | sync left right => exact hidden)

def action : ProductAction (ProductAction SenderAction ReceiverAction) Unit :=
  .sync (.sync .send .receive) ()

theorem concreteInitialAndEnabled :
    concrete.init ((false, false), false) ∧
    concrete.step ((false, false), false) action ((true, true), true) := by
  refine ⟨⟨⟨rfl, rfl⟩, rfl⟩, ?_⟩
  exact ProductStep.synchronize trivial
    (ProductStep.synchronize trivial ⟨rfl, rfl⟩ ⟨rfl, rfl⟩) ⟨rfl, rfl⟩

theorem nestedStepLifts :
    abstract.init ((false, false), false) ∧
    abstract.step ((false, false), false) action ((true, true), true) :=
  ⟨refinement.init concreteInitialAndEnabled.1,
    refinement.visibleStep concreteInitialAndEnabled.2 (fun hidden => hidden)⟩

theorem resourceBound :
    (concrete.resources action).grade.use .work = 3 ∧
    (abstract.resources action).grade.use .work = 4 := ⟨rfl, rfl⟩

/-- The theorem must not be mistaken for permission to form this source product. -/
theorem formationStillRejected : ¬ Composable inner monitor monitorConnection := by
  intro formed
  exact formed.directions (leftAction := .sync .send .receive)
    (rightAction := ()) trivial

def undersized : Behavior
    (ProductAction (ProductAction SenderAction ReceiverAction) Unit)
    Capability ContractFact GradeAtom ((Bool × Bool) × Bool) :=
  { abstract with resources := fun a =>
  { abstract.resources a with grade := ⟨fun _ => 2⟩ } }

/-- A lower advertised work bound cannot be accepted by an arbitrary state map. -/
theorem undersizedBoundRejected :
    ¬ Nonempty (ResourceWeakRefinement concrete undersized) := by
  intro ⟨candidate⟩
  have impossible : 3 ≤ 2 := (candidate.resources action).grade .work
  exact (by decide : ¬ (3 ≤ 2)) impossible

end NMLT.Discovery.NestedReuse
