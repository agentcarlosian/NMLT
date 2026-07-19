import NMLT.Behavior.TemporalTyping

namespace NMLT.Behavior

/-- Reflexive-transitive closure used to allow stuttering and finite abstract
    step sequences without pretending that observation equality is stutter. -/
inductive Reaches {State : Type} (step : State → State → Prop) : State → State → Prop
  | refl (state) : Reaches step state state
  | tail {source middle target} : step source middle → Reaches step middle target →
      Reaches step source target

def Model.stateStep {identity : Nat} (behavior : Model identity)
    (before after : behavior.State) : Prop :=
  ∃ label input output grade, behavior.step before label input after output grade

theorem Reaches.trans {State : Type} {step : State → State → Prop}
    {source middle target : State} :
    Reaches step source middle → Reaches step middle target → Reaches step source target := by
  intro first second
  induction first with
  | refl => exact second
  | tail edge rest inductionHypothesis => exact .tail edge (inductionHypothesis second)

/-- A proof-relevant directed refinement. One concrete step maps to zero or
    more abstract steps; zero steps are genuine equality in abstract state. -/
structure Refinement {concreteId abstractId : Nat}
    (concrete : Model concreteId) (abstract : Model abstractId) where
  mapState : concrete.State → abstract.State
  mapOutput : concrete.Output → abstract.Output
  initial : ∀ {state}, concrete.init state → abstract.init (mapState state)
  observation : ∀ state, mapOutput (concrete.observe state) = abstract.observe (mapState state)
  step : ∀ {before after}, concrete.stateStep before after →
    Reaches abstract.stateStep (mapState before) (mapState after)

namespace Refinement

def identity {identity : Nat} (behavior : Model identity) : Refinement behavior behavior where
  mapState := id
  mapOutput := id
  initial := id
  observation := fun _ => rfl
  step := fun concreteStep => .tail concreteStep (.refl _)

theorem mapReaches {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (refinement : Refinement concrete abstract) {before after : concrete.State} :
    Reaches concrete.stateStep before after →
      Reaches abstract.stateStep (refinement.mapState before) (refinement.mapState after) := by
  intro path
  induction path with
  | refl => exact .refl _
  | tail edge rest inductionHypothesis =>
      exact Reaches.trans (refinement.step edge) inductionHypothesis

def compose {firstId secondId thirdId : Nat}
    {first : Model firstId} {second : Model secondId} {third : Model thirdId}
    (left : Refinement first second) (right : Refinement second third) :
    Refinement first third where
  mapState := right.mapState ∘ left.mapState
  mapOutput := right.mapOutput ∘ left.mapOutput
  initial := fun initial => right.initial (left.initial initial)
  observation := fun state =>
    (congrArg right.mapOutput (left.observation state)).trans
      (right.observation (left.mapState state))
  step := fun concreteStep => right.mapReaches (left.step concreteStep)

@[simp] theorem identity_then_map {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (refinement : Refinement concrete abstract) (state : concrete.State) :
    (compose (identity concrete) refinement).mapState state = refinement.mapState state := rfl

@[simp] theorem then_identity_map {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (refinement : Refinement concrete abstract) (state : concrete.State) :
    (compose refinement (identity abstract)).mapState state = refinement.mapState state := rfl

@[simp] theorem then_assoc_map {firstId secondId thirdId fourthId : Nat}
    {first : Model firstId} {second : Model secondId}
    {third : Model thirdId} {fourth : Model fourthId}
    (one : Refinement first second) (two : Refinement second third)
    (three : Refinement third fourth) (state : first.State) :
    (compose (compose one two) three).mapState state =
      (compose one (compose two three)).mapState state := rfl

end Refinement

def InitiallyReachable {identity : Nat} (behavior : Model identity)
    (state : behavior.State) : Prop :=
  ∃ initial, behavior.init initial ∧ Reaches behavior.stateStep initial state

def Invariant {identity : Nat} (behavior : Model identity)
    (predicate : behavior.State → Prop) : Prop :=
  ∀ state, InitiallyReachable behavior state → predicate state

theorem Refinement.reachable {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (refinement : Refinement concrete abstract) {state : concrete.State} :
    InitiallyReachable concrete state →
      InitiallyReachable abstract (refinement.mapState state) := by
  rintro ⟨initial, starts, path⟩
  exact ⟨refinement.mapState initial, refinement.initial starts, refinement.mapReaches path⟩

/-- State invariants transport contravariantly along a checked directed
    refinement. Liveness transport is deliberately absent until fairness and
    divergence obligations are represented. -/
theorem Refinement.transportInvariant {concreteId abstractId : Nat}
    {concrete : Model concreteId} {abstract : Model abstractId}
    (refinement : Refinement concrete abstract)
    (predicate : abstract.State → Prop) (holds : Invariant abstract predicate) :
    Invariant concrete (predicate ∘ refinement.mapState) := by
  intro state reachable
  exact holds (refinement.mapState state) (refinement.reachable reachable)

#print axioms Reaches.trans
#print axioms Refinement.mapReaches
#print axioms Refinement.identity_then_map
#print axioms Refinement.then_identity_map
#print axioms Refinement.then_assoc_map
#print axioms Refinement.transportInvariant

end NMLT.Behavior
