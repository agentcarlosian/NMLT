import NMLT.Behavior.Refinement
import NMLT.Behavior.OpenComposition

namespace NMLT.Behavior.OpenRefinement

open NMLT.Behavior
open NMLT.Behavior.OpenComposition

/-- A canonical finite predicate is a truth table over `Fin cardinality`.
    There is one Boolean at each payload index, so representation order and
    duplicate-value questions are absent from the logical model. -/
abbrev FinitePredicate (cardinality : Nat) := Fin cardinality → Bool

def PredicateSubset {cardinality : Nat}
    (left right : FinitePredicate cardinality) : Prop :=
  ∀ message, left message = true → right message = true

theorem PredicateSubset.refl {cardinality : Nat}
    (predicate : FinitePredicate cardinality) :
    PredicateSubset predicate predicate := by
  intro _ accepted
  exact accepted

theorem PredicateSubset.trans {cardinality : Nat}
    {first second third : FinitePredicate cardinality} :
    PredicateSubset first second → PredicateSubset second third →
      PredicateSubset first third := by
  intro firstToSecond secondToThird message accepted
  exact secondToThird message (firstToSecond message accepted)

/-- Total assume/guarantee data for a finite boundary alphabet. Payload types
    are nominal identities; this model deliberately has no subtyping rule. -/
structure Contract (Label PayloadId : Type) (cardinality : Nat) where
  direction : Label → Direction
  payloadType : Label → PayloadId
  assumption : Label → FinitePredicate cardinality
  guarantee : Label → FinitePredicate cardinality

/-- Open refinement is behavioral refinement plus a complete, injective label
    renaming. Inputs are contravariant (`A_abstract ⊆ A_concrete`) and outputs
    are covariant (`G_concrete ⊆ G_abstract`). Exact payload equality excludes
    representation-changing substitution from this relation. -/
structure Refinement
    {concreteId abstractId cardinality : Nat}
    {ConcreteLabel AbstractLabel PayloadId : Type}
    {concreteModel : Model concreteId} {abstractModel : Model abstractId}
    (concreteContract : Contract ConcreteLabel PayloadId cardinality)
    (abstractContract : Contract AbstractLabel PayloadId cardinality) where
  behavior : NMLT.Behavior.Refinement concreteModel abstractModel
  mapLabel : ConcreteLabel → AbstractLabel
  labelInjective : Function.Injective mapLabel
  labelSurjective : Function.Surjective mapLabel
  direction : ∀ label,
    concreteContract.direction label = abstractContract.direction (mapLabel label)
  payloadType : ∀ label,
    concreteContract.payloadType label = abstractContract.payloadType (mapLabel label)
  assumption : ∀ label,
    PredicateSubset
      (abstractContract.assumption (mapLabel label))
      (concreteContract.assumption label)
  guarantee : ∀ label,
    PredicateSubset
      (concreteContract.guarantee label)
      (abstractContract.guarantee (mapLabel label))

namespace Refinement

def identity {identity cardinality : Nat} {Label PayloadId : Type}
    {model : Model identity} (contract : Contract Label PayloadId cardinality) :
    Refinement (concreteModel := model) (abstractModel := model) contract contract where
  behavior := NMLT.Behavior.Refinement.identity model
  mapLabel := id
  labelInjective := fun {_ _} equality => equality
  labelSurjective := fun label => ⟨label, rfl⟩
  direction := fun _ => rfl
  payloadType := fun _ => rfl
  assumption := fun label => PredicateSubset.refl (contract.assumption label)
  guarantee := fun label => PredicateSubset.refl (contract.guarantee label)

def compose
    {firstId secondId thirdId cardinality : Nat}
    {FirstLabel SecondLabel ThirdLabel PayloadId : Type}
    {firstModel : Model firstId} {secondModel : Model secondId}
    {thirdModel : Model thirdId}
    {firstContract : Contract FirstLabel PayloadId cardinality}
    {secondContract : Contract SecondLabel PayloadId cardinality}
    {thirdContract : Contract ThirdLabel PayloadId cardinality}
    (left : Refinement (concreteModel := firstModel) (abstractModel := secondModel)
      firstContract secondContract)
    (right : Refinement (concreteModel := secondModel) (abstractModel := thirdModel)
      secondContract thirdContract) :
    Refinement (concreteModel := firstModel) (abstractModel := thirdModel)
      firstContract thirdContract where
  behavior := NMLT.Behavior.Refinement.compose left.behavior right.behavior
  mapLabel := right.mapLabel ∘ left.mapLabel
  labelInjective := fun {_ _} equality =>
    left.labelInjective (right.labelInjective equality)
  labelSurjective := by
    intro thirdLabel
    obtain ⟨secondLabel, secondMaps⟩ := right.labelSurjective thirdLabel
    obtain ⟨firstLabel, firstMaps⟩ := left.labelSurjective secondLabel
    exact ⟨firstLabel, congrArg right.mapLabel firstMaps |>.trans secondMaps⟩
  direction := fun label =>
    (left.direction label).trans (right.direction (left.mapLabel label))
  payloadType := fun label =>
    (left.payloadType label).trans (right.payloadType (left.mapLabel label))
  assumption := fun label =>
    PredicateSubset.trans
      (right.assumption (left.mapLabel label))
      (left.assumption label)
  guarantee := fun label =>
    PredicateSubset.trans
      (left.guarantee label)
      (right.guarantee (left.mapLabel label))

@[simp] theorem identity_mapLabel {modelId cardinality : Nat}
    {Label PayloadId : Type} {model : Model modelId}
    (contract : Contract Label PayloadId cardinality) (label : Label) :
    (identity contract :
      Refinement (concreteModel := model) (abstractModel := model)
        contract contract).mapLabel label = label := rfl

@[simp] theorem compose_mapLabel
    {firstId secondId thirdId cardinality : Nat}
    {FirstLabel SecondLabel ThirdLabel PayloadId : Type}
    {firstModel : Model firstId} {secondModel : Model secondId}
    {thirdModel : Model thirdId}
    {firstContract : Contract FirstLabel PayloadId cardinality}
    {secondContract : Contract SecondLabel PayloadId cardinality}
    {thirdContract : Contract ThirdLabel PayloadId cardinality}
    (left : Refinement (concreteModel := firstModel) (abstractModel := secondModel)
      firstContract secondContract)
    (right : Refinement (concreteModel := secondModel) (abstractModel := thirdModel)
      secondContract thirdContract)
    (label : FirstLabel) :
    (compose left right).mapLabel label = right.mapLabel (left.mapLabel label) := rfl

/-- Direct projection used by the payload-substitution negative control. -/
theorem exactPayloadIdentity
    {concreteId abstractId cardinality : Nat}
    {ConcreteLabel AbstractLabel PayloadId : Type}
    {concreteModel : Model concreteId} {abstractModel : Model abstractId}
    {concreteContract : Contract ConcreteLabel PayloadId cardinality}
    {abstractContract : Contract AbstractLabel PayloadId cardinality}
    (refinement : Refinement (concreteModel := concreteModel)
      (abstractModel := abstractModel) concreteContract abstractContract)
    (label : ConcreteLabel) :
    concreteContract.payloadType label =
      abstractContract.payloadType (refinement.mapLabel label) :=
  refinement.payloadType label

/-- Direct projection used by the assumption-strengthening negative control. -/
theorem abstractAssumptionIncluded
    {concreteId abstractId cardinality : Nat}
    {ConcreteLabel AbstractLabel PayloadId : Type}
    {concreteModel : Model concreteId} {abstractModel : Model abstractId}
    {concreteContract : Contract ConcreteLabel PayloadId cardinality}
    {abstractContract : Contract AbstractLabel PayloadId cardinality}
    (refinement : Refinement (concreteModel := concreteModel)
      (abstractModel := abstractModel) concreteContract abstractContract)
    (label : ConcreteLabel) :
    PredicateSubset
      (abstractContract.assumption (refinement.mapLabel label))
      (concreteContract.assumption label) :=
  refinement.assumption label

/-- Direct projection used by the guarantee-weakening negative control. -/
theorem concreteGuaranteeIncluded
    {concreteId abstractId cardinality : Nat}
    {ConcreteLabel AbstractLabel PayloadId : Type}
    {concreteModel : Model concreteId} {abstractModel : Model abstractId}
    {concreteContract : Contract ConcreteLabel PayloadId cardinality}
    {abstractContract : Contract AbstractLabel PayloadId cardinality}
    (refinement : Refinement (concreteModel := concreteModel)
      (abstractModel := abstractModel) concreteContract abstractContract)
    (label : ConcreteLabel) :
    PredicateSubset
      (concreteContract.guarantee label)
      (abstractContract.guarantee (refinement.mapLabel label)) :=
  refinement.guarantee label

end Refinement

#print axioms PredicateSubset.refl
#print axioms PredicateSubset.trans
#print axioms Refinement.identity
#print axioms Refinement.compose
#print axioms Refinement.exactPayloadIdentity
#print axioms Refinement.abstractAssumptionIncluded
#print axioms Refinement.concreteGuaranteeIncluded

end NMLT.Behavior.OpenRefinement
