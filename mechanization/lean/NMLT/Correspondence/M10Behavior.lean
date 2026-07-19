import NMLT.Correspondence.M9Kernel
import NMLT.Behavior.Coinductive

namespace NMLT.M10

open NMLT
open NMLT.Behavior

def unitGrades : GradeAlgebra where
  Carrier := Unit
  one := ()
  tensor := fun _ _ => ()
  le := fun _ _ => True
  tensor_assoc := by intros; rfl
  one_tensor := by intros; rfl
  tensor_one := by intros; rfl
  le_refl := by intros; trivial
  le_trans := by intros; trivial
  tensor_mono := by intros; trivial

/-- The M9 core action semantics embedded as a behavior. The identity is an
    explicit input because the Lean correspondence model abstracts canonical
    byte hashing as a trusted boundary. -/
def behaviorOfCore (identity : Nat) (raw : M9.RawCore) : Model identity where
  State := M9.Store
  Input := Nat
  Output := M9.Store
  Label := Nat
  grades := unitGrades
  init := fun _ => True
  step := fun before label input after output grade =>
    ∃ action, raw.actions[label]? = some action ∧ input = label ∧
      M9.CoreSteps action before after ∧ output = after ∧ grade = ()
  observe := id
  silent := fun _ => none

def actionRelation {identity : Nat} {raw : M9.RawCore}
    (index : Nat) (action : M9.RawAction) :
    (behaviorOfCore identity raw).State →
      (behaviorOfCore identity raw).Input →
      (behaviorOfCore identity raw).State →
      (behaviorOfCore identity raw).Output →
      (behaviorOfCore identity raw).grades.Carrier → Prop :=
  fun before input after output grade =>
    input = index ∧ M9.CoreSteps action before after ∧ output = after ∧ grade = ()

/-- Every action selected from an independently checked M9 core has a typed
    behavior action at the same index. -/
def typedAction {identity : Nat} (checked : M9.CheckedCore)
    (index : Nat) (action : M9.RawAction)
    (selected : checked.raw.actions[index]? = some action) :
    TypedAction (behaviorOfCore identity checked.raw) where
  label := index
  relation := actionRelation index action
  included := by
    rintro before input after output grade ⟨inputIndex, step, outputAfter, gradeUnit⟩
    exact ⟨action, selected, inputIndex, step, outputAfter, gradeUnit⟩

/-- State predicates become properties only at the exact core-behavior
    identity supplied by the source/evidence layer. -/
def stateProperty {identity : Nat} (raw : M9.RawCore)
    (predicate : M9.Store → Prop) : TraceProperty (behaviorOfCore identity raw) :=
  .atom predicate

theorem typedActionPreservesSelection {identity : Nat} (checked : M9.CheckedCore)
    (index : Nat) (action : M9.RawAction)
    (selected : checked.raw.actions[index]? = some action) :
    (typedAction (identity := identity) checked index action selected).label = index := rfl

#print axioms typedActionPreservesSelection

end NMLT.M10
