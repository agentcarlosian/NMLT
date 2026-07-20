import NMLT.Behavior.OpenKernelGenerated.Funs

namespace NMLT.Behavior.OpenKernelExecution

open Aeneas
open NMLT.OpenKernel

/-- The independently checkable obligations exposed by an accepted execution of
the bounded Rust congruence kernel. -/
structure ExecutionContract (raw : Congruence) : Prop where
  payloadIdentityPresent : raw.payload_identity_present = true
  payloadVariantsUnique : raw.payload_variants_unique = true
  payloadWithinCapacity : raw.payload_cardinality ≤ MAX_PAYLOAD_VARIANTS
  leftRefinementAccepted :
    refinement_valid raw.payload_cardinality raw.concrete_left raw.abstract_left
      raw.left_refinement = .ok true
  rightRefinementAccepted :
    refinement_valid raw.payload_cardinality raw.concrete_right raw.abstract_right
      raw.right_refinement = .ok true
  wiringAccepted : wiring_valid raw = .ok true

/-- Successful execution of the Aeneas translation of `nmlt-open-kernel::check`
implies every top-level validation obligation. -/
theorem check_accepts_implies_contract (raw : Congruence)
    (accepted : check raw = .ok true) : ExecutionContract raw := by
  simp only [check] at accepted
  split at accepted
  next payloadPresent =>
    split at accepted
    next variantsUnique =>
      split at accepted
      next withinCapacity =>
        generalize leftEquation :
          refinement_valid raw.payload_cardinality raw.concrete_left raw.abstract_left
            raw.left_refinement = leftResult at accepted
        cases leftResult with
        | fail error => simp at accepted
        | div => simp at accepted
        | ok leftAccepted =>
          cases leftAccepted with
          | false => simp at accepted
          | true =>
            generalize rightEquation :
              refinement_valid raw.payload_cardinality raw.concrete_right raw.abstract_right
                raw.right_refinement = rightResult at accepted
            cases rightResult with
            | fail error => simp at accepted
            | div => simp at accepted
            | ok rightAccepted =>
              cases rightAccepted with
              | false => simp at accepted
              | true =>
                exact {
                  payloadIdentityPresent := payloadPresent
                  payloadVariantsUnique := variantsUnique
                  payloadWithinCapacity := withinCapacity
                  leftRefinementAccepted := leftEquation
                  rightRefinementAccepted := rightEquation
                  wiringAccepted := accepted
                }
      next outsideCapacity => simp at accepted
    next variantsDuplicate => simp at accepted
  next payloadMissing => simp at accepted

/-- Exact bounded readback is the equality decision executed by the translated
Rust kernel. -/
def KernelReadbackExact (expected raw : Congruence) : Prop :=
  Congruence.Insts.CoreCmpPartialEqCongruence.eq expected raw = .ok true

namespace EqualitySoundness

/-- Soundness of the generated equality for the scalar-only grade leaf. -/
theorem grade (left right : Grade)
    (accepted : Grade.Insts.CoreCmpPartialEqGrade.eq left right = .ok true) :
    left = right := by
  rcases left with ⟨leftCost, leftPrivacy, leftEnergy, leftUncertainty⟩
  rcases right with ⟨rightCost, rightPrivacy, rightEnergy, rightUncertainty⟩
  simp only [Grade.Insts.CoreCmpPartialEqGrade.eq] at accepted
  split at accepted
  next costEqual =>
    split at accepted
    next privacyEqual =>
      split at accepted
      next energyEqual =>
        simp only [Std.Result.ok.injEq, decide_eq_true_eq] at accepted
        subst rightCost
        subst rightPrivacy
        subst rightEnergy
        subst rightUncertainty
        rfl
      next energyDifferent => simp at accepted
    next privacyDifferent => simp at accepted
  next costDifferent => simp at accepted

end EqualitySoundness

/-- The translated bound checker exposes its successful structural readback
decision together with the complete execution contract. -/
theorem check_bound_accepts_implies_exact_contract
    (expected raw : Congruence)
    (accepted : check_bound expected raw = .ok true) :
    KernelReadbackExact expected raw ∧ ExecutionContract raw := by
  simp only [check_bound] at accepted
  generalize equalityEquation :
    Congruence.Insts.CoreCmpPartialEqCongruence.eq expected raw = equalityResult at accepted
  cases equalityResult with
  | fail error => simp at accepted
  | div => simp at accepted
  | ok equal =>
      cases equal with
      | false => simp at accepted
      | true =>
          exact ⟨equalityEquation, check_accepts_implies_contract raw accepted⟩

#print axioms check_accepts_implies_contract
#print axioms EqualitySoundness.grade
#print axioms check_bound_accepts_implies_exact_contract

end NMLT.Behavior.OpenKernelExecution
