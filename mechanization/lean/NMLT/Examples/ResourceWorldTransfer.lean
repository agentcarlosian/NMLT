import NMLT.Behavior.ResourceWorld

namespace NMLT.Examples.ResourceWorldTransfer

open NMLT.Behavior.ResourceBehavior
open NMLT.Behavior.ResourceWorld

inductive Owner where | sender | receiver
deriving DecidableEq

inductive Capability where | permit
deriving DecidableEq

inductive ContractFact where | authorized | ready
deriving DecidableEq

inductive GradeAtom where | work
deriving DecidableEq

inductive SenderAction where | send
inductive ReceiverAction where | receive

def senderProfile : ResourceProfile Capability ContractFact GradeAtom where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => True
  receives := fun _ => False
  grade := ⟨fun _ => 1⟩
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

def abstractSenderProfile : ResourceProfile Capability ContractFact GradeAtom := {
  senderProfile with grade := ⟨fun _ => 2⟩
}

instance senderConsumesDecidable : DecidablePred senderProfile.consumes := by
  intro capability
  cases capability
  exact isFalse id

instance receiverConsumesDecidable : DecidablePred receiverProfile.consumes := by
  intro capability
  cases capability
  exact isFalse id

instance senderTransfersDecidable : DecidablePred senderProfile.transfers := by
  intro capability
  cases capability
  exact isTrue trivial

instance receiverTransfersDecidable : DecidablePred receiverProfile.transfers := by
  intro capability
  cases capability
  exact isFalse id

def before : AuthorityWorld Capability Owner where
  owner := fun _ => some .sender

def synchronized : SyncEnabled Owner.sender Owner.receiver
    senderProfile receiverProfile before where
  ownersDistinct := by decide
  leftEnabled := by
    constructor
    · intro capability impossible
      exact False.elim impossible
    · intro capability impossible
      exact False.elim impossible
    · intro capability _
      cases capability
      rfl
    · intro capability impossible
      exact False.elim impossible
    · intro capability impossible
      exact False.elim impossible
    · intro capability impossible
      exact False.elim impossible
    · intro capability _ impossible
      exact False.elim impossible
  rightEnabled := by
    constructor
    · intro capability impossible
      exact False.elim impossible
    · intro capability impossible
      exact False.elim impossible
    · intro capability impossible
      exact False.elim impossible
    · intro capability _ received
      cases capability
      simp [AuthorityWorld.Owns, before] at received
    · intro capability impossible
      exact False.elim impossible
    · intro capability impossible
      exact False.elim impossible
    · intro capability impossible
      exact False.elim impossible
  compatible := by
    constructor <;> intro subject <;> cases subject <;>
      simp [senderProfile, receiverProfile]

def after : AuthorityWorld Capability Owner :=
  syncResult Owner.sender Owner.receiver senderProfile receiverProfile before

def transferStep : SyncStep Owner.sender Owner.receiver
    senderProfile receiverProfile before after :=
  synchronized.toStep

def sender : Behavior SenderAction Capability ContractFact GradeAtom Bool where
  State := Bool
  init := fun state => state = false
  step := fun before _ after => before = false ∧ after = true
  observe := id
  hidden := fun _ => False
  direction := fun _ => .output
  payload := fun _ => "Once<Unit>"
  owns := fun _ => True
  resources := fun _ => senderProfile

def abstractSender : Behavior SenderAction Capability ContractFact GradeAtom Bool where
  State := Bool
  init := fun state => state = false
  step := fun before _ after => before = false ∧ after = true
  observe := id
  hidden := fun _ => False
  direction := fun _ => .output
  payload := fun _ => "Once<Unit>"
  owns := fun _ => True
  resources := fun _ => abstractSenderProfile

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

def productTransfer : ProductStep sender receiver Owner.sender Owner.receiver connection
    ⟨false, false, before⟩ (.sync .send .receive) ⟨true, true, after⟩ := by
  apply NMLT.Behavior.ResourceWorld.ProductStep.synchronize
  · trivial
  · exact ⟨rfl, rfl⟩
  · exact ⟨rfl, rfl⟩
  · exact transferStep

def senderRefinement : ResourceWeakRefinement sender abstractSender where
  mapState := id
  init := id
  observe := id
  observeState := fun _ => rfl
  hiddenPreserved := fun _ => Iff.rfl
  hiddenStep := by
    intro before action after stepped hidden
    exact False.elim hidden
  visibleStep := by
    intro before action after stepped visible
    exact stepped
  resources := by
    intro action
    cases action
    constructor
    · intro capability impossible
      exact False.elim impossible
    · intro capability
      exact Iff.rfl
    · intro capability
      exact Iff.rfl
    · intro capability
      exact Iff.rfl
    · intro atom
      cases atom
      decide
    · intro fact relied
      exact relied
    · intro fact guaranteed
      exact guaranteed
  hiddenResources := by
    intro action hidden
    exact False.elim hidden

def worldRefinement : WorldWeakRefinement sender abstractSender where
  behavior := senderRefinement
  worldResources := by
    intro action
    cases action
    exact {
      profile := senderRefinement.resources .send
      requiresBack := by
        intro capability impossible
        exact False.elim impossible
    }

def wiring : WiringEquivalent connection connection where
  connected := fun _ _ => Iff.rfl

def senderReceiverComposable : Composable sender receiver connection where
  directions := by
    intro leftAction rightAction connected
    cases leftAction
    cases rightAction
    trivial
  payloads := by
    intro leftAction rightAction connected
    cases leftAction
    cases rightAction
    rfl
  hiddenLeft := by
    intro leftAction rightAction connected hidden
    exact False.elim hidden
  hiddenRight := by
    intro leftAction rightAction connected hidden
    exact False.elim hidden
  capabilities := by
    intro capability leftOwns rightOwns
    exact False.elim rightOwns
  resources := by
    intro leftAction rightAction connected
    cases leftAction
    cases rightAction
    exact ⟨fun _ => synchronized.compatible, fun impossible => False.elim (by
      cases impossible)⟩

def abstractSynchronized : SynchronizationCompatible
    abstractSenderProfile receiverProfile := by
  constructor <;> intro subject <;> cases subject <;>
    simp [abstractSenderProfile, senderProfile, receiverProfile]

def abstractSenderReceiverComposable :
    Composable abstractSender receiver connection where
  directions := senderReceiverComposable.directions
  payloads := senderReceiverComposable.payloads
  hiddenLeft := senderReceiverComposable.hiddenLeft
  hiddenRight := senderReceiverComposable.hiddenRight
  capabilities := senderReceiverComposable.capabilities
  resources := by
    intro leftAction rightAction connected
    cases leftAction
    cases rightAction
    exact ⟨fun _ => abstractSynchronized, fun impossible => False.elim (by
      cases impossible)⟩

def liftedProductTransfer :
    ProductStep abstractSender receiver Owner.sender Owner.receiver connection
      (mapProductState worldRefinement ⟨false, false, before⟩)
      (.sync .send .receive)
      (mapProductState worldRefinement ⟨true, true, after⟩) :=
  liftSynchronized worldRefinement wiring senderReceiverComposable
    abstractSenderReceiverComposable productTransfer

/-- The synchronized product moves `permit` once and the sender cannot retain it. -/
theorem permit_moves_exactly_once :
    before.Owns Owner.sender Capability.permit ∧
      after.Owns Owner.receiver Capability.permit ∧
      ¬ after.Owns Owner.sender Capability.permit :=
  ProductStep.synchronized_left_transfer_moves_once productTransfer trivial

/-- The same authority-changing synchronization is preserved by refinement. -/
theorem permit_transfer_lifts_dynamically :
    Nonempty
      (ProductStep abstractSender receiver Owner.sender Owner.receiver connection
        (mapProductState worldRefinement ⟨false, false, before⟩)
        (.sync .send .receive)
        (mapProductState worldRefinement ⟨true, true, after⟩)) :=
  ⟨liftedProductTransfer⟩

end NMLT.Examples.ResourceWorldTransfer
