namespace NMLT.Behavior.ResourceBehavior

structure Grade (Atom : Type) where
  use : Atom → Nat

namespace Grade

def zero {Atom : Type} : Grade Atom := ⟨fun _ => 0⟩

def add {Atom : Type} (left right : Grade Atom) : Grade Atom :=
  ⟨fun atom => left.use atom + right.use atom⟩

def Le {Atom : Type} (left right : Grade Atom) : Prop :=
  ∀ atom, left.use atom ≤ right.use atom

theorem le_refl {Atom : Type} (grade : Grade Atom) : Le grade grade :=
  fun _ => Nat.le_refl _

theorem add_le_add {Atom : Type} {a b c d : Grade Atom}
    (left : Le a b) (right : Le c d) : Le (add a c) (add b d) := by
  intro atom
  exact Nat.add_le_add (left atom) (right atom)

end Grade

structure ResourceProfile (Capability Fact GradeAtom : Type) where
  requires : Capability → Prop
  consumes : Capability → Prop
  transfers : Capability → Prop
  receives : Capability → Prop
  grade : Grade GradeAtom
  relies : Fact → Prop
  guarantees : Fact → Prop

def ResourceProfile.empty {Capability Fact GradeAtom : Type} :
    ResourceProfile Capability Fact GradeAtom where
  requires := fun _ => False
  consumes := fun _ => False
  transfers := fun _ => False
  receives := fun _ => False
  grade := Grade.zero
  relies := fun _ => False
  guarantees := fun _ => False

def ResourceProfile.parallel {Capability Fact GradeAtom : Type}
    (left right : ResourceProfile Capability Fact GradeAtom) :
    ResourceProfile Capability Fact GradeAtom where
  requires := fun capability => left.requires capability ∨ right.requires capability
  consumes := fun capability => left.consumes capability ∨ right.consumes capability
  transfers := fun _ => False
  receives := fun _ => False
  grade := Grade.add left.grade right.grade
  relies := fun fact =>
    (left.relies fact ∧ ¬ right.guarantees fact) ∨
      (right.relies fact ∧ ¬ left.guarantees fact)
  guarantees := fun fact => left.guarantees fact ∨ right.guarantees fact

structure ResourceRefines {Capability Fact GradeAtom : Type}
    (concrete abstract : ResourceProfile Capability Fact GradeAtom) : Prop where
  requires : ∀ capability, concrete.requires capability → abstract.requires capability
  consumes : ∀ capability, concrete.consumes capability ↔ abstract.consumes capability
  transfers : ∀ capability, concrete.transfers capability ↔ abstract.transfers capability
  receives : ∀ capability, concrete.receives capability ↔ abstract.receives capability
  grade : Grade.Le concrete.grade abstract.grade
  relies : ∀ fact, concrete.relies fact → abstract.relies fact
  guarantees : ∀ fact, abstract.guarantees fact → concrete.guarantees fact

namespace ResourceRefines

theorem refl {Capability Fact GradeAtom : Type}
    (profile : ResourceProfile Capability Fact GradeAtom) :
    ResourceRefines profile profile where
  requires := fun _ => id
  consumes := fun _ => Iff.rfl
  transfers := fun _ => Iff.rfl
  receives := fun _ => Iff.rfl
  grade := Grade.le_refl _
  relies := fun _ => id
  guarantees := fun _ => id

theorem parallel {Capability Fact GradeAtom : Type}
    {concreteLeft abstractLeft concreteRight abstractRight :
      ResourceProfile Capability Fact GradeAtom}
    (left : ResourceRefines concreteLeft abstractLeft)
    (right : ResourceRefines concreteRight abstractRight) :
    ResourceRefines
      (ResourceProfile.parallel concreteLeft concreteRight)
      (ResourceProfile.parallel abstractLeft abstractRight) where
  requires := by
    intro capability required
    cases required with
    | inl required => exact Or.inl (left.requires capability required)
    | inr required => exact Or.inr (right.requires capability required)
  consumes := by
    intro capability
    constructor
    · intro consumed
      cases consumed with
      | inl consumed => exact Or.inl ((left.consumes capability).mp consumed)
      | inr consumed => exact Or.inr ((right.consumes capability).mp consumed)
    · intro consumed
      cases consumed with
      | inl consumed => exact Or.inl ((left.consumes capability).mpr consumed)
      | inr consumed => exact Or.inr ((right.consumes capability).mpr consumed)
  transfers := fun _ => Iff.rfl
  receives := fun _ => Iff.rfl
  grade := Grade.add_le_add left.grade right.grade
  relies := by
    intro fact relied
    cases relied with
    | inl relied =>
        exact Or.inl ⟨left.relies fact relied.1, fun guarantee =>
          relied.2 (right.guarantees fact guarantee)⟩
    | inr relied =>
        exact Or.inr ⟨right.relies fact relied.1, fun guarantee =>
          relied.2 (left.guarantees fact guarantee)⟩
  guarantees := by
    intro fact guaranteed
    cases guaranteed with
    | inl guaranteed => exact Or.inl (left.guarantees fact guaranteed)
    | inr guaranteed => exact Or.inr (right.guarantees fact guaranteed)

end ResourceRefines

inductive Direction where
  | internal
  | input
  | output
deriving DecidableEq

def Direction.Complementary : Direction → Direction → Prop
  | .input, .output => True
  | .output, .input => True
  | _, _ => False

structure Behavior
    (Action Capability Fact GradeAtom Observation : Type) where
  State : Type
  init : State → Prop
  step : State → Action → State → Prop
  observe : State → Observation
  hidden : Action → Prop
  direction : Action → Direction
  payload : Action → String
  owns : Capability → Prop
  resources : Action → ResourceProfile Capability Fact GradeAtom

def CapabilityPartition {Capability : Type}
    (left right : Capability → Prop) : Prop :=
  ∀ capability, left capability → right capability → False

structure SynchronizationCompatible {Capability Fact GradeAtom : Type}
    (sender receiver : ResourceProfile Capability Fact GradeAtom) : Prop where
  transfer : ∀ capability, sender.transfers capability ↔ receiver.receives capability
  noReverseTransfer : ∀ capability,
    receiver.transfers capability ↔ sender.receives capability
  senderRely : ∀ fact, sender.relies fact → receiver.guarantees fact
  receiverRely : ∀ fact, receiver.relies fact → sender.guarantees fact

structure WiringEquivalent {LeftAction RightAction : Type}
    (concrete abstract : LeftAction → RightAction → Prop) : Prop where
  connected : ∀ left right, concrete left right ↔ abstract left right

structure Composable
    {LeftAction RightAction Capability Fact GradeAtom LeftObservation RightObservation : Type}
    (left : Behavior LeftAction Capability Fact GradeAtom LeftObservation)
    (right : Behavior RightAction Capability Fact GradeAtom RightObservation)
    (connection : LeftAction → RightAction → Prop) : Prop where
  directions : ∀ {leftAction rightAction}, connection leftAction rightAction →
    Direction.Complementary (left.direction leftAction) (right.direction rightAction)
  payloads : ∀ {leftAction rightAction}, connection leftAction rightAction →
    left.payload leftAction = right.payload rightAction
  hiddenLeft : ∀ {leftAction rightAction}, connection leftAction rightAction →
    ¬ left.hidden leftAction
  hiddenRight : ∀ {leftAction rightAction}, connection leftAction rightAction →
    ¬ right.hidden rightAction
  capabilities : CapabilityPartition left.owns right.owns
  resources : ∀ {leftAction rightAction}, connection leftAction rightAction →
    (left.direction leftAction = .output →
      SynchronizationCompatible
        (left.resources leftAction) (right.resources rightAction)) ∧
    (right.direction rightAction = .output →
      SynchronizationCompatible
        (right.resources rightAction) (left.resources leftAction))

inductive ProductAction (LeftAction RightAction : Type) where
  | left (action : LeftAction)
  | right (action : RightAction)
  | sync (left : LeftAction) (right : RightAction)

inductive ProductStep
    {LeftAction RightAction Capability Fact GradeAtom LeftObservation RightObservation : Type}
    (left : Behavior LeftAction Capability Fact GradeAtom LeftObservation)
    (right : Behavior RightAction Capability Fact GradeAtom RightObservation)
    (connection : LeftAction → RightAction → Prop) :
    (left.State × right.State) → ProductAction LeftAction RightAction →
      (left.State × right.State) → Prop where
  | fromLeft {leftBefore leftAfter rightState action} :
      left.step leftBefore action leftAfter →
      (∀ rightAction, ¬ connection action rightAction) →
      ProductStep left right connection (leftBefore, rightState)
        (.left action) (leftAfter, rightState)
  | fromRight {leftState rightBefore rightAfter action} :
      right.step rightBefore action rightAfter →
      (∀ leftAction, ¬ connection leftAction action) →
      ProductStep left right connection (leftState, rightBefore)
        (.right action) (leftState, rightAfter)
  | synchronize {leftBefore leftAfter rightBefore rightAfter leftAction rightAction} :
      connection leftAction rightAction →
      left.step leftBefore leftAction leftAfter →
      right.step rightBefore rightAction rightAfter →
      ProductStep left right connection (leftBefore, rightBefore)
        (.sync leftAction rightAction) (leftAfter, rightAfter)

def parallel
    {LeftAction RightAction Capability Fact GradeAtom LeftObservation RightObservation : Type}
    (left : Behavior LeftAction Capability Fact GradeAtom LeftObservation)
    (right : Behavior RightAction Capability Fact GradeAtom RightObservation)
    (connection : LeftAction → RightAction → Prop) :
    Behavior (ProductAction LeftAction RightAction) Capability Fact GradeAtom
      (LeftObservation × RightObservation) where
  State := left.State × right.State
  init := fun state => left.init state.1 ∧ right.init state.2
  step := ProductStep left right connection
  observe := fun state => (left.observe state.1, right.observe state.2)
  hidden
    | .left action => left.hidden action
    | .right _ => False
    | .sync _ _ => False
  direction := fun _ => .internal
  payload := fun _ => "Unit"
  owns := fun capability => left.owns capability ∨ right.owns capability
  resources
    | .left action => left.resources action
    | .right action => right.resources action
    | .sync leftAction rightAction =>
        ResourceProfile.parallel
          (left.resources leftAction) (right.resources rightAction)

structure ResourceWeakRefinement
    {Action Capability Fact GradeAtom ConcreteObservation AbstractObservation : Type}
    (concrete : Behavior Action Capability Fact GradeAtom ConcreteObservation)
    (abstract : Behavior Action Capability Fact GradeAtom AbstractObservation) where
  mapState : concrete.State → abstract.State
  init : ∀ {state}, concrete.init state → abstract.init (mapState state)
  observe : ConcreteObservation → AbstractObservation
  observeState : ∀ state, observe (concrete.observe state) =
    abstract.observe (mapState state)
  hiddenPreserved : ∀ action, concrete.hidden action ↔ abstract.hidden action
  hiddenStep : ∀ {before action after}, concrete.step before action after →
    concrete.hidden action → mapState before = mapState after
  visibleStep : ∀ {before action after}, concrete.step before action after →
    ¬ concrete.hidden action →
    abstract.step (mapState before) action (mapState after)
  resources : ∀ action,
    ResourceRefines (concrete.resources action) (abstract.resources action)
  hiddenResources : ∀ action, concrete.hidden action →
    ResourceRefines (concrete.resources action) ResourceProfile.empty

def liftParallelWitness
    {Action PeerAction Capability Fact GradeAtom
      ConcreteObservation AbstractObservation PeerObservation : Type}
    {concrete : Behavior Action Capability Fact GradeAtom ConcreteObservation}
    {abstract : Behavior Action Capability Fact GradeAtom AbstractObservation}
    {peer : Behavior PeerAction Capability Fact GradeAtom PeerObservation}
    {concreteConnection abstractConnection : Action → PeerAction → Prop}
    (refinement : ResourceWeakRefinement concrete abstract)
    (wiring : WiringEquivalent concreteConnection abstractConnection)
    (concreteComposable : Composable concrete peer concreteConnection)
    (_abstractComposable : Composable abstract peer abstractConnection) :
    ResourceWeakRefinement
      (parallel concrete peer concreteConnection)
      (parallel abstract peer abstractConnection) where
  mapState := fun state => (refinement.mapState state.1, state.2)
  init := by
    intro state initial
    exact ⟨refinement.init initial.1, initial.2⟩
  observe := fun observation => (refinement.observe observation.1, observation.2)
  observeState := by
    intro state
    exact Prod.ext (refinement.observeState state.1) rfl
  hiddenPreserved := by
    intro action
    cases action with
    | left action => exact refinement.hiddenPreserved action
    | right action => exact Iff.rfl
    | sync leftAction rightAction => exact Iff.rfl
  hiddenStep := by
    intro before action after step hidden
    cases step with
    | fromLeft leftStep isolated =>
        exact Prod.ext (refinement.hiddenStep leftStep hidden) rfl
    | fromRight rightStep isolated => exact False.elim hidden
    | synchronize connected leftStep rightStep => exact False.elim hidden
  visibleStep := by
    intro before action after step visible
    cases step with
    | fromLeft leftStep isolated =>
        exact ProductStep.fromLeft
          (refinement.visibleStep leftStep visible)
          (fun rightAction abstractConnected =>
            isolated rightAction ((wiring.connected _ _).mpr abstractConnected))
    | fromRight rightStep isolated =>
        exact ProductStep.fromRight rightStep
          (fun leftAction abstractConnected =>
            isolated leftAction ((wiring.connected _ _).mpr abstractConnected))
    | synchronize connected leftStep rightStep =>
        exact ProductStep.synchronize
          ((wiring.connected _ _).mp connected)
          (refinement.visibleStep leftStep
            (concreteComposable.hiddenLeft connected))
          rightStep
  resources := by
    intro action
    cases action with
    | left action => exact refinement.resources action
    | right action => exact ResourceRefines.refl _
    | sync leftAction rightAction =>
        exact ResourceRefines.parallel
          (refinement.resources leftAction) (ResourceRefines.refl _)
  hiddenResources := by
    intro action hidden
    cases action with
    | left action => exact refinement.hiddenResources action hidden
    | right action => exact False.elim hidden
    | sync leftAction rightAction => exact False.elim hidden

theorem liftParallel
    {Action PeerAction Capability Fact GradeAtom
      ConcreteObservation AbstractObservation PeerObservation : Type}
    {concrete : Behavior Action Capability Fact GradeAtom ConcreteObservation}
    {abstract : Behavior Action Capability Fact GradeAtom AbstractObservation}
    {peer : Behavior PeerAction Capability Fact GradeAtom PeerObservation}
    {concreteConnection abstractConnection : Action → PeerAction → Prop}
    (refinement : ResourceWeakRefinement concrete abstract)
    (wiring : WiringEquivalent concreteConnection abstractConnection)
    (concreteComposable : Composable concrete peer concreteConnection)
    (abstractComposable : Composable abstract peer abstractConnection) :
    Nonempty
      (ResourceWeakRefinement
        (parallel concrete peer concreteConnection)
        (parallel abstract peer abstractConnection)) :=
  ⟨liftParallelWitness refinement wiring concreteComposable abstractComposable⟩

end NMLT.Behavior.ResourceBehavior
