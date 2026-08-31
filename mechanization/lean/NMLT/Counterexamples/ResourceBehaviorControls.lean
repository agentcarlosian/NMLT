import NMLT.Behavior.ResourceBehavior

namespace NMLT.Counterexamples.ResourceBehaviorControls

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

def hiddenSender := { sender 1 with hidden := fun _ => True }

/-- Omitting hidden-boundary isolation admits a connected hidden action. -/
theorem hiddenBoundaryConnection_isRejected :
    ¬ Composable hiddenSender receiver connection := by
  intro formed
  exact formed.hiddenLeft (show connection .send .receive from trivial) trivial

def disconnected : SenderAction → ReceiverAction → Prop := fun _ _ => False

/-- A concrete and abstract connection relation that disagree are not wiring-equivalent. -/
theorem wiringMismatch_isRejected :
    ¬ WiringEquivalent connection disconnected := by
  intro equivalent
  exact (equivalent.connected .send .receive).mp trivial

/-- Shared nominal authority violates the product's affine ownership premise. -/
theorem duplicateCapabilityOwnership_isRejected :
    ¬ CapabilityPartition
      (fun _ : Capability => True) (fun _ : Capability => True) := by
  intro partition
  exact partition .permit trivial trivial

def receiverWithoutReceive : ResourceProfile Capability ContractFact GradeAtom where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := ⟨fun _ => 2⟩
  relies := receiverProfile.relies
  guarantees := receiverProfile.guarantees

/-- A transfer without the peer receive is not synchronization-compatible. -/
theorem unmatchedTransfer_isRejected :
    ¬ SynchronizationCompatible (senderProfile 1) receiverWithoutReceive := by
  intro compatible
  exact (compatible.transfer .permit).mp trivial

/-- Pointwise grade monotonicity rules out a concrete widening from two to three. -/
theorem gradeIncrease_isRejected :
    ¬ ResourceRefines (senderProfile 3) (senderProfile 2) := by
  intro refines
  have impossible : 3 ≤ 2 := refines.grade .work
  exact (by decide : ¬ 3 ≤ 2) impossible

def receiverWithoutGuarantee : ResourceProfile Capability ContractFact GradeAtom where
  requires := receiverProfile.requires
  consumes := receiverProfile.consumes
  transfers := receiverProfile.transfers
  receives := receiverProfile.receives
  grade := receiverProfile.grade
  relies := receiverProfile.relies
  guarantees := fun _ => False

/-- A peer must discharge every reliance of the synchronized action. -/
theorem undischargedRely_isRejected :
    ¬ SynchronizationCompatible (senderProfile 1) receiverWithoutGuarantee := by
  intro compatible
  exact compatible.senderRely .ready trivial

def hiddenConsumption : ResourceProfile Capability ContractFact GradeAtom where
  requires := fun _ => False
  consumes := fun _ => True
  transfers := fun _ => False
  receives := fun _ => False
  grade := Grade.zero
  relies := fun _ => False
  guarantees := fun _ => False

/-- A hidden step with authority effects cannot be treated as abstract stutter. -/
theorem hiddenResourceEffectAgainstStutter_isRejected :
    ¬ ResourceRefines hiddenConsumption ResourceProfile.empty := by
  intro refines
  exact (refines.consumes .permit).mp trivial

end NMLT.Counterexamples.ResourceBehaviorControls
