import NMLT.Behavior.ResourceBehavior

namespace NMLT.Examples.VisibleResourceSync

open NMLT.Behavior.ResourceBehavior

inductive SenderAction where | send
inductive ReceiverAction where | receive
inductive Capability where | permit
inductive ContractFact where | authorized | ready
inductive GradeAtom where | work

def senderProfile (work : Nat) : ResourceProfile Capability ContractFact GradeAtom where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => True
  receives := fun _ => False
  grade := ⟨fun _ => work⟩
  relies
    | .authorized => False
    | .ready => True
  guarantees
    | .authorized => True
    | .ready => False

def receiverProfile : ResourceProfile Capability ContractFact GradeAtom where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => True
  grade := ⟨fun _ => 2⟩
  relies
    | .authorized => True
    | .ready => False
  guarantees
    | .authorized => False
    | .ready => True

def sender (work : Nat) :
    Behavior SenderAction Capability ContractFact GradeAtom Bool where
  State := Bool
  init := fun state => state = false
  step := fun before _ after => before = false ∧ after = true
  observe := id
  hidden := fun _ => False
  direction := fun _ => .output
  payload := fun _ => "Once<Unit>"
  owns := fun _ => True
  resources := fun _ => senderProfile work

def receiver : Behavior ReceiverAction Capability ContractFact GradeAtom Bool where
  State := Bool
  init := fun state => state = false
  step := fun before _ after => before = false ∧ after = true
  observe := id
  hidden := fun _ => False
  direction := fun _ => .input
  payload := fun _ => "Once<Unit>"
  owns := fun _ => False
  resources := fun _ => receiverProfile

def connection : SenderAction → ReceiverAction → Prop := fun _ _ => True

def senderRefinement : ResourceWeakRefinement (sender 1) (sender 2) where
  mapState := id
  init := id
  observe := id
  observeState := fun _ => rfl
  hiddenPreserved := fun _ => Iff.rfl
  hiddenStep := by
    intro _ _ _ _ hidden
    exact False.elim hidden
  visibleStep := by
    intro _ _ _ step _
    exact step
  resources := by
    intro action
    cases action
    constructor
    · intro _ required
      exact False.elim required
    · intro _
      exact Iff.rfl
    · intro _
      exact Iff.rfl
    · intro _
      exact Iff.rfl
    · intro atom
      cases atom
      decide
    · intro fact relied
      exact relied
    · intro fact guaranteed
      exact guaranteed
  hiddenResources := by
    intro _ hidden
    exact False.elim hidden

def wiring : WiringEquivalent connection connection where
  connected := fun _ _ => Iff.rfl

def senderReceiverComposable (work : Nat) : Composable (sender work) receiver connection where
  directions := by
    intro _ _ _
    trivial
  payloads := by
    intro _ _ _
    rfl
  hiddenLeft := by
    intro _ _ _ hidden
    exact False.elim hidden
  hiddenRight := by
    intro _ _ _ hidden
    exact False.elim hidden
  capabilities := by
    intro _ _ peerOwns
    exact False.elim peerOwns
  resources := by
    intro leftAction rightAction _
    cases leftAction
    cases rightAction
    constructor
    · intro _
      constructor <;> intro capability <;> cases capability <;>
        simp [sender, senderProfile, receiver, receiverProfile]
    · intro impossible
      contradiction

/-- The primary source fixture's concrete sender lifts through its real product. -/
theorem visibleResourceSync_lifts :
    Nonempty
      (ResourceWeakRefinement
        (parallel (sender 1) receiver connection)
        (parallel (sender 2) receiver connection)) :=
  liftParallel senderRefinement wiring
    (senderReceiverComposable 1) (senderReceiverComposable 2)

end NMLT.Examples.VisibleResourceSync
