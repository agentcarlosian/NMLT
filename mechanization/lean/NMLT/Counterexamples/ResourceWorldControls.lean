import NMLT.Examples.ResourceWorldTransfer

namespace NMLT.Counterexamples.ResourceWorldControls

open NMLT.Behavior.ResourceBehavior
open NMLT.Behavior.ResourceWorld
open NMLT.Examples.ResourceWorldTransfer

def wronglyOwned : AuthorityWorld Capability Owner where
  owner := fun _ => some .receiver

/-- A transfer cannot start unless the sender owns the capability. -/
theorem transferWithoutOwnership_isRejected :
    ¬ Enabled Owner.sender senderProfile wronglyOwned := by
  intro enabled
  have senderOwns := enabled.transfers Capability.permit trivial
  simp [AuthorityWorld.Owns, wronglyOwned] at senderOwns

/-- An unconnected local transition cannot silently perform a boundary transfer. -/
theorem isolatedTransfer_isRejected :
    ¬ ∃ after, LocalStep Owner.sender senderProfile before after := by
  rintro ⟨_, step⟩
  exact step.noTransfer Capability.permit trivial

/-- Functional authority worlds cannot assign one capability to two owners. -/
theorem duplicateOwnership_isRejected
    (world : AuthorityWorld Capability Owner) :
    ¬ (world.Owns Owner.sender Capability.permit ∧
      world.Owns Owner.receiver Capability.permit) := by
  intro duplicate
  have equalOwners := world.ownership_unique duplicate.1 duplicate.2
  exact Owner.noConfusion equalOwners

def consumingProfile : ResourceProfile Capability ContractFact GradeAtom where
  requires := fun _ => False
  consumes := fun _ => True
  transfers := fun _ => False
  receives := fun _ => False
  grade := ⟨fun _ => 0⟩
  relies := fun _ => False
  guarantees := fun _ => False

instance consumingDecidable : DecidablePred consumingProfile.consumes := by
  intro capability
  cases capability
  exact isTrue trivial

def consumingEnabled : Enabled Owner.sender consumingProfile before where
  requires := by
    intro capability impossible
    exact False.elim impossible
  consumes := by
    intro capability _
    cases capability
    rfl
  transfers := by
    intro capability impossible
    exact False.elim impossible
  receivesFresh := by
    intro capability impossible
    exact False.elim impossible
  consumeTransfer := by
    intro capability consumed impossible
    exact False.elim impossible
  consumeReceive := by
    intro capability consumed impossible
    exact False.elim impossible
  transferReceive := by
    intro capability impossible
    exact False.elim impossible

def afterConsumption : AuthorityWorld Capability Owner :=
  localResult consumingProfile before

def consumingStep : LocalStep Owner.sender consumingProfile before afterConsumption :=
  consumingEnabled.toLocalStep
    (fun _ impossible => False.elim impossible)
    (fun _ impossible => False.elim impossible)

/-- Hidden consumption is a real authority-world change, so it cannot stutter. -/
theorem hiddenConsumption_changesWorld : afterConsumption ≠ before := by
  intro unchanged
  have vacantAfter := consumingStep.consumed Capability.permit trivial
  rw [unchanged] at vacantAfter
  simp [AuthorityWorld.Vacant, before] at vacantAfter

/-- The complete hidden-resource premise rejects that changing step. -/
theorem hiddenConsumption_cannotRefineStutter :
    ¬ ResourceRefines consumingProfile ResourceProfile.empty := by
  intro refines
  exact (refines.consumes Capability.permit).mp trivial

def noRequirement : ResourceProfile Capability ContractFact GradeAtom :=
  ResourceProfile.empty

def abstractRequiresPermit : ResourceProfile Capability ContractFact GradeAtom := {
  ResourceProfile.empty with
  requires := fun _ => True
}

def vacant : AuthorityWorld Capability Owner where
  owner := fun _ => none

def noRequirementEnabled : Enabled Owner.sender noRequirement vacant where
  requires := by
    intro capability impossible
    exact False.elim impossible
  consumes := by
    intro capability impossible
    exact False.elim impossible
  transfers := by
    intro capability impossible
    exact False.elim impossible
  receivesFresh := by
    intro capability impossible
    exact False.elim impossible
  consumeTransfer := by
    intro capability impossible
    exact False.elim impossible
  consumeReceive := by
    intro capability impossible
    exact False.elim impossible
  transferReceive := by
    intro capability impossible
    exact False.elim impossible

def annotationRefines : ResourceRefines noRequirement abstractRequiresPermit where
  requires := by
    intro capability impossible
    exact False.elim impossible
  consumes := fun _ => Iff.rfl
  transfers := fun _ => Iff.rfl
  receives := fun _ => Iff.rfl
  grade := fun _ => Nat.le_refl _
  relies := by
    intro fact impossible
    exact False.elim impossible
  guarantees := by
    intro fact impossible
    exact False.elim impossible

/--
The old annotation order alone does not transport dynamic enabledness: the
abstract profile may require authority absent from both the concrete profile
and the world.
-/
theorem annotationRefinement_doesNotPreserveEnabled :
    ResourceRefines noRequirement abstractRequiresPermit ∧
      Enabled Owner.sender noRequirement vacant ∧
      ¬ Enabled Owner.sender abstractRequiresPermit vacant := by
  refine ⟨annotationRefines, noRequirementEnabled, ?_⟩
  intro enabled
  have owns := enabled.requires Capability.permit trivial
  simp [AuthorityWorld.Owns, vacant] at owns

end NMLT.Counterexamples.ResourceWorldControls
