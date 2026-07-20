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

#print axioms check_accepts_implies_contract

end NMLT.Behavior.OpenKernelExecution
