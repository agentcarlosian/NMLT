import NMLT.Behavior.OpenMappedCongruence
import NMLT.Grades.Algebra

namespace NMLT.Behavior.OpenResourceCongruence

open NMLT.Grades
open NMLT.Behavior.OpenComposition
open NMLT.Behavior.OpenMappedCongruence

/-- Per-action authority, quantitative, and rely/guarantee data. Capability and
    fact identities are nominal and shared across a refinement; no relabeling
    can silently change authority or environmental facts. -/
structure ActionResources (Capability Fact : Type) where
  requires : Capability → Prop
  consumes : Capability → Prop
  transfers : Capability → Prop
  receives : Capability → Prop
  grade : Grade
  rely : Fact → Prop
  guarantees : Fact → Prop

/-- System-level resource data. `owned` is the affine authority partition at
    the component boundary; actions are indexed by the semantic action type. -/
structure SystemResources (Action Capability Fact : Type) where
  owned : Capability → Prop
  action : Action → ActionResources Capability Fact

/-- Resource refinement cannot widen authority or environmental assumptions.
    Consumption and transfer are exact, quantitative grades improve, and every
    abstract guaranteed fact remains guaranteed concretely. -/
structure ResourceRefinement
    {ConcreteAction AbstractAction Capability Fact : Type}
    (concrete : SystemResources ConcreteAction Capability Fact)
    (abstract : SystemResources AbstractAction Capability Fact)
    (mapAction : ConcreteAction → AbstractAction) : Prop where
  owned : ∀ capability, concrete.owned capability → abstract.owned capability
  requires : ∀ action capability,
    (concrete.action action).requires capability →
      (abstract.action (mapAction action)).requires capability
  consumes : ∀ action capability,
    (concrete.action action).consumes capability ↔
      (abstract.action (mapAction action)).consumes capability
  transfers : ∀ action capability,
    (concrete.action action).transfers capability ↔
      (abstract.action (mapAction action)).transfers capability
  receives : ∀ action capability,
    (concrete.action action).receives capability ↔
      (abstract.action (mapAction action)).receives capability
  grade : ∀ action,
    Le (concrete.action action).grade
      (abstract.action (mapAction action)).grade
  rely : ∀ action fact,
    (concrete.action action).rely fact →
      (abstract.action (mapAction action)).rely fact
  guarantees : ∀ action fact,
    (abstract.action (mapAction action)).guarantees fact →
      (concrete.action action).guarantees fact

/-- The per-action projection of `ResourceRefinement`. Keeping this as a
    first-class structure lets the product proof handle interleavings,
    exposed actions, and synchronizations uniformly. -/
structure ActionResourceRefinement {Capability Fact : Type}
    (concrete abstract : ActionResources Capability Fact) : Prop where
  requires : ∀ capability, concrete.requires capability → abstract.requires capability
  consumes : ∀ capability, concrete.consumes capability ↔ abstract.consumes capability
  transfers : ∀ capability, concrete.transfers capability ↔ abstract.transfers capability
  receives : ∀ capability, concrete.receives capability ↔ abstract.receives capability
  grade : Le concrete.grade abstract.grade
  rely : ∀ fact, concrete.rely fact → abstract.rely fact
  guarantees : ∀ fact, abstract.guarantees fact → concrete.guarantees fact

def ResourceRefinement.actionRefinement
    {ConcreteAction AbstractAction Capability Fact : Type}
    {concrete : SystemResources ConcreteAction Capability Fact}
    {abstract : SystemResources AbstractAction Capability Fact}
    {mapAction : ConcreteAction → AbstractAction}
    (refinement : ResourceRefinement concrete abstract mapAction)
    (action : ConcreteAction) :
    ActionResourceRefinement (concrete.action action)
      (abstract.action (mapAction action)) where
  requires := refinement.requires action
  consumes := refinement.consumes action
  transfers := refinement.transfers action
  receives := refinement.receives action
  grade := refinement.grade action
  rely := refinement.rely action
  guarantees := refinement.guarantees action

def CapabilityPartition {Capability : Type}
    (left right : Capability → Prop) : Prop :=
  ∀ capability, left capability → right capability → False

/-- A synchronization moves authority rather than copying it and discharges
    both sides' rely facts. Requiring both directions is conservative when one
    side has an empty rely/transfer relation. -/
structure SynchronizationCompatible {Capability Fact : Type}
    (left right : ActionResources Capability Fact) : Prop where
  leftTransfer : ∀ capability, left.transfers capability ↔ right.receives capability
  rightTransfer : ∀ capability, right.transfers capability ↔ left.receives capability
  leftRely : ∀ fact, left.rely fact → right.guarantees fact
  rightRely : ∀ fact, right.rely fact → left.guarantees fact

/-- Resources of one synchronized product action. Transfers and receives are
    internalized by the atomic synchronization; the product owns the unique
    capability identity before and after the ownership move. -/
def parallelAction {Capability Fact : Type}
    (left right : ActionResources Capability Fact) : ActionResources Capability Fact where
  requires := fun capability => left.requires capability ∨ right.requires capability
  consumes := fun capability => left.consumes capability ∨ right.consumes capability
  transfers := fun _ => False
  receives := fun _ => False
  grade := NMLT.Grades.parallel left.grade right.grade
  rely := fun fact =>
    (left.rely fact ∧ ¬ right.guarantees fact) ∨
      (right.rely fact ∧ ¬ left.guarantees fact)
  guarantees := fun fact => left.guarantees fact ∨ right.guarantees fact

def parallelSystemResources
    {LeftAction RightAction Capability Fact : Type}
    (left : SystemResources LeftAction Capability Fact)
    (right : SystemResources RightAction Capability Fact) :
    SystemResources (LeftAction × RightAction) Capability Fact where
  owned := fun capability => left.owned capability ∨ right.owned capability
  action := fun actions =>
    parallelAction (left.action actions.1) (right.action actions.2)

/-- Resource data for every action constructor of `OpenComposition.parallel`.
    Interleavings and exposed actions retain the originating component's
    profile. Synchronizations combine both profiles and internalize the
    transfer/receive boundary. -/
def openProductActionResources
    {LeftInternal LeftPort RightInternal RightPort Message Capability Fact : Type}
    (left : SystemResources
      (OpenAction LeftInternal LeftPort Message) Capability Fact)
    (right : SystemResources
      (OpenAction RightInternal RightPort Message) Capability Fact) :
    OpenAction
        (CompositeInternal LeftInternal LeftPort RightInternal RightPort Message)
        (Sum LeftPort RightPort) Message → ActionResources Capability Fact
  | .internal (.fromLeft name) => left.action (.internal name)
  | .internal (.fromRight name) => right.action (.internal name)
  | .internal (.leftToRight leftPort rightPort message) =>
      parallelAction (left.action (.output leftPort message))
        (right.action (.input rightPort message))
  | .internal (.rightToLeft rightPort leftPort message) =>
      parallelAction (right.action (.output rightPort message))
        (left.action (.input leftPort message))
  | .input (.inl port) message => left.action (.input port message)
  | .input (.inr port) message => right.action (.input port message)
  | .output (.inl port) message => left.action (.output port message)
  | .output (.inr port) message => right.action (.output port message)

def openProductSystemResources
    {LeftInternal LeftPort RightInternal RightPort Message Capability Fact : Type}
    (left : SystemResources
      (OpenAction LeftInternal LeftPort Message) Capability Fact)
    (right : SystemResources
      (OpenAction RightInternal RightPort Message) Capability Fact) :
    SystemResources
      (OpenAction
        (CompositeInternal LeftInternal LeftPort RightInternal RightPort Message)
        (Sum LeftPort RightPort) Message) Capability Fact where
  owned := fun capability => left.owned capability ∨ right.owned capability
  action := openProductActionResources left right

/-- Per-action resource refinement is closed under synchronization. Grade
    monotonicity follows from the grade algebra rather than being supplied as
    an extra premise. -/
def ActionResourceRefinement.parallel
    {Capability Fact : Type}
    {concreteLeft abstractLeft concreteRight abstractRight :
      ActionResources Capability Fact}
    (left : ActionResourceRefinement concreteLeft abstractLeft)
    (right : ActionResourceRefinement concreteRight abstractRight) :
    ActionResourceRefinement (parallelAction concreteLeft concreteRight)
      (parallelAction abstractLeft abstractRight) where
  requires := by
    intro capability required
    cases required with
    | inl required => exact Or.inl (left.requires capability required)
    | inr required => exact Or.inr (right.requires capability required)
  consumes := by
    intro capability
    exact or_congr (left.consumes capability) (right.consumes capability)
  transfers := by simp [parallelAction]
  receives := by simp [parallelAction]
  grade := NMLT.Grades.sequential_mono left.grade right.grade
  rely := by
    intro fact relied
    cases relied with
    | inl leftRelied =>
        left
        exact ⟨left.rely fact leftRelied.1,
          fun abstractGuaranteed =>
            leftRelied.2 (right.guarantees fact abstractGuaranteed)⟩
    | inr rightRelied =>
        right
        exact ⟨right.rely fact rightRelied.1,
          fun abstractGuaranteed =>
            rightRelied.2 (left.guarantees fact abstractGuaranteed)⟩
  guarantees := by
    intro fact guaranteed
    cases guaranteed with
    | inl guaranteed => exact Or.inl (left.guarantees fact guaranteed)
    | inr guaranteed => exact Or.inr (right.guarantees fact guaranteed)

/-- Every structural product-action constructor has a corresponding
    per-action resource refinement. -/
def liftOpenProductActionResources
    {ConcreteLeftInternal AbstractLeftInternal ConcreteLeftPort AbstractLeftPort : Type}
    {ConcreteRightInternal AbstractRightInternal ConcreteRightPort AbstractRightPort : Type}
    {Message Capability Fact : Type}
    {concreteLeft : SystemResources
      (OpenAction ConcreteLeftInternal ConcreteLeftPort Message) Capability Fact}
    {abstractLeft : SystemResources
      (OpenAction AbstractLeftInternal AbstractLeftPort Message) Capability Fact}
    {concreteRight : SystemResources
      (OpenAction ConcreteRightInternal ConcreteRightPort Message) Capability Fact}
    {abstractRight : SystemResources
      (OpenAction AbstractRightInternal AbstractRightPort Message) Capability Fact}
    (mapLeftInternal : ConcreteLeftInternal → AbstractLeftInternal)
    (mapLeftPort : ConcreteLeftPort → AbstractLeftPort)
    (mapRightInternal : ConcreteRightInternal → AbstractRightInternal)
    (mapRightPort : ConcreteRightPort → AbstractRightPort)
    (left : ResourceRefinement concreteLeft abstractLeft
      (mapOpenAction mapLeftInternal mapLeftPort))
    (right : ResourceRefinement concreteRight abstractRight
      (mapOpenAction mapRightInternal mapRightPort))
    (action : OpenAction
      (CompositeInternal ConcreteLeftInternal ConcreteLeftPort
        ConcreteRightInternal ConcreteRightPort Message)
      (Sum ConcreteLeftPort ConcreteRightPort) Message) :
    ActionResourceRefinement
      (openProductActionResources concreteLeft concreteRight action)
      (openProductActionResources abstractLeft abstractRight
        (mapOpenAction
          (MappedRefinement.mapCompositeInternal mapLeftInternal mapLeftPort
            mapRightInternal mapRightPort)
          (Sum.map mapLeftPort mapRightPort) action)) := by
  cases action with
  | internal action =>
      cases action with
      | fromLeft name => exact left.actionRefinement (.internal name)
      | fromRight name => exact right.actionRefinement (.internal name)
      | leftToRight leftPort rightPort message =>
          exact ActionResourceRefinement.parallel
            (left.actionRefinement (.output leftPort message))
            (right.actionRefinement (.input rightPort message))
      | rightToLeft rightPort leftPort message =>
          exact ActionResourceRefinement.parallel
            (right.actionRefinement (.output rightPort message))
            (left.actionRefinement (.input leftPort message))
  | input port message =>
      cases port with
      | inl port => exact left.actionRefinement (.input port message)
      | inr port => exact right.actionRefinement (.input port message)
  | output port message =>
      cases port with
      | inl port => exact left.actionRefinement (.output port message)
      | inr port => exact right.actionRefinement (.output port message)

/-- The resource counterpart of `MappedRefinement.liftParallel`, covering all
    eight constructors of `ParallelStep`: two component-internal cases, four
    exposed input/output cases, and both synchronization directions. -/
def liftOpenProductResources
    {ConcreteLeftInternal AbstractLeftInternal ConcreteLeftPort AbstractLeftPort : Type}
    {ConcreteRightInternal AbstractRightInternal ConcreteRightPort AbstractRightPort : Type}
    {Message Capability Fact : Type}
    {concreteLeft : SystemResources
      (OpenAction ConcreteLeftInternal ConcreteLeftPort Message) Capability Fact}
    {abstractLeft : SystemResources
      (OpenAction AbstractLeftInternal AbstractLeftPort Message) Capability Fact}
    {concreteRight : SystemResources
      (OpenAction ConcreteRightInternal ConcreteRightPort Message) Capability Fact}
    {abstractRight : SystemResources
      (OpenAction AbstractRightInternal AbstractRightPort Message) Capability Fact}
    (mapLeftInternal : ConcreteLeftInternal → AbstractLeftInternal)
    (mapLeftPort : ConcreteLeftPort → AbstractLeftPort)
    (mapRightInternal : ConcreteRightInternal → AbstractRightInternal)
    (mapRightPort : ConcreteRightPort → AbstractRightPort)
    (left : ResourceRefinement concreteLeft abstractLeft
      (mapOpenAction mapLeftInternal mapLeftPort))
    (right : ResourceRefinement concreteRight abstractRight
      (mapOpenAction mapRightInternal mapRightPort)) :
    ResourceRefinement
      (openProductSystemResources concreteLeft concreteRight)
      (openProductSystemResources abstractLeft abstractRight)
      (mapOpenAction
        (MappedRefinement.mapCompositeInternal mapLeftInternal mapLeftPort
          mapRightInternal mapRightPort)
        (Sum.map mapLeftPort mapRightPort)) where
  owned := by
    intro capability owned
    cases owned with
    | inl owned => exact Or.inl (left.owned capability owned)
    | inr owned => exact Or.inr (right.owned capability owned)
  requires := fun action =>
    (liftOpenProductActionResources mapLeftInternal mapLeftPort
      mapRightInternal mapRightPort left right action).requires
  consumes := fun action =>
    (liftOpenProductActionResources mapLeftInternal mapLeftPort
      mapRightInternal mapRightPort left right action).consumes
  transfers := fun action =>
    (liftOpenProductActionResources mapLeftInternal mapLeftPort
      mapRightInternal mapRightPort left right action).transfers
  receives := fun action =>
    (liftOpenProductActionResources mapLeftInternal mapLeftPort
      mapRightInternal mapRightPort left right action).receives
  grade := fun action =>
    (liftOpenProductActionResources mapLeftInternal mapLeftPort
      mapRightInternal mapRightPort left right action).grade
  rely := fun action =>
    (liftOpenProductActionResources mapLeftInternal mapLeftPort
      mapRightInternal mapRightPort left right action).rely
  guarantees := fun action =>
    (liftOpenProductActionResources mapLeftInternal mapLeftPort
      mapRightInternal mapRightPort left right action).guarantees

/-- One refinement witness bundles operational/contract simulation with the
    resource relation over the same action map. -/
structure ResourceAwareMappedRefinement
    {ConcreteInternal AbstractInternal ConcretePort AbstractPort Message Observation : Type}
    {Capability Fact : Type}
    {concreteInterface : Interface ConcretePort Message}
    {abstractInterface : Interface AbstractPort Message}
    (concrete : System ConcreteInternal ConcretePort Message Observation concreteInterface)
    (abstract : System AbstractInternal AbstractPort Message Observation abstractInterface)
    (concreteResources : SystemResources
      (OpenAction ConcreteInternal ConcretePort Message) Capability Fact)
    (abstractResources : SystemResources
      (OpenAction AbstractInternal AbstractPort Message) Capability Fact) where
  behavior : MappedRefinement concrete abstract
  resources : ResourceRefinement concreteResources abstractResources
    (mapOpenAction behavior.mapInternal behavior.mapPort)

/-- Fully bundled two-sided congruence: the same mapped product witness carries
    operational/contract simulation and resource preservation for all eight
    structural product-action cases. -/
def liftResourceAwareParallel
    {ConcreteLeftInternal AbstractLeftInternal ConcreteLeftPort AbstractLeftPort : Type}
    {ConcreteRightInternal AbstractRightInternal ConcreteRightPort AbstractRightPort : Type}
    {Message LeftObservation RightObservation Capability Fact : Type}
    {concreteLeftInterface : Interface ConcreteLeftPort Message}
    {abstractLeftInterface : Interface AbstractLeftPort Message}
    {concreteRightInterface : Interface ConcreteRightPort Message}
    {abstractRightInterface : Interface AbstractRightPort Message}
    {concreteLeft : System ConcreteLeftInternal ConcreteLeftPort Message
      LeftObservation concreteLeftInterface}
    {abstractLeft : System AbstractLeftInternal AbstractLeftPort Message
      LeftObservation abstractLeftInterface}
    {concreteRight : System ConcreteRightInternal ConcreteRightPort Message
      RightObservation concreteRightInterface}
    {abstractRight : System AbstractRightInternal AbstractRightPort Message
      RightObservation abstractRightInterface}
    {concreteLeftResources : SystemResources
      (OpenAction ConcreteLeftInternal ConcreteLeftPort Message) Capability Fact}
    {abstractLeftResources : SystemResources
      (OpenAction AbstractLeftInternal AbstractLeftPort Message) Capability Fact}
    {concreteRightResources : SystemResources
      (OpenAction ConcreteRightInternal ConcreteRightPort Message) Capability Fact}
    {abstractRightResources : SystemResources
      (OpenAction AbstractRightInternal AbstractRightPort Message) Capability Fact}
    {concreteWiring : Wiring ConcreteLeftPort ConcreteRightPort}
    {abstractWiring : Wiring AbstractLeftPort AbstractRightPort}
    (left : ResourceAwareMappedRefinement concreteLeft abstractLeft
      concreteLeftResources abstractLeftResources)
    (right : ResourceAwareMappedRefinement concreteRight abstractRight
      concreteRightResources abstractRightResources)
    (wiring : MappedWiringEquivalent concreteWiring abstractWiring
      left.behavior.mapPort right.behavior.mapPort) :
    ResourceAwareMappedRefinement
      (parallel concreteLeft concreteRight concreteWiring)
      (parallel abstractLeft abstractRight abstractWiring)
      (openProductSystemResources concreteLeftResources concreteRightResources)
      (openProductSystemResources abstractLeftResources abstractRightResources) where
  behavior := MappedRefinement.liftParallel left.behavior right.behavior wiring
  resources := liftOpenProductResources
    left.behavior.mapInternal left.behavior.mapPort
    right.behavior.mapInternal right.behavior.mapPort
    left.resources right.resources

theorem partition_preserved
    {ConcreteLeftAction AbstractLeftAction ConcreteRightAction AbstractRightAction : Type}
    {Capability Fact : Type}
    {concreteLeft : SystemResources ConcreteLeftAction Capability Fact}
    {abstractLeft : SystemResources AbstractLeftAction Capability Fact}
    {concreteRight : SystemResources ConcreteRightAction Capability Fact}
    {abstractRight : SystemResources AbstractRightAction Capability Fact}
    {mapLeft : ConcreteLeftAction → AbstractLeftAction}
    {mapRight : ConcreteRightAction → AbstractRightAction}
    (left : ResourceRefinement concreteLeft abstractLeft mapLeft)
    (right : ResourceRefinement concreteRight abstractRight mapRight)
    (abstractPartition : CapabilityPartition abstractLeft.owned abstractRight.owned) :
    CapabilityPartition concreteLeft.owned concreteRight.owned := by
  intro capability concreteLeftOwns concreteRightOwns
  exact abstractPartition capability
    (left.owned capability concreteLeftOwns)
    (right.owned capability concreteRightOwns)

/-- Two resource refinements lift through a synchronized action. This is the
    quantitative/authority/rely counterpart of the structural product theorem
    in `OpenMappedCongruence`. -/
def liftParallelAction
    {ConcreteLeftAction AbstractLeftAction ConcreteRightAction AbstractRightAction : Type}
    {Capability Fact : Type}
    {concreteLeft : SystemResources ConcreteLeftAction Capability Fact}
    {abstractLeft : SystemResources AbstractLeftAction Capability Fact}
    {concreteRight : SystemResources ConcreteRightAction Capability Fact}
    {abstractRight : SystemResources AbstractRightAction Capability Fact}
    (mapLeft : ConcreteLeftAction → AbstractLeftAction)
    (mapRight : ConcreteRightAction → AbstractRightAction)
    (left : ResourceRefinement concreteLeft abstractLeft mapLeft)
    (right : ResourceRefinement concreteRight abstractRight mapRight)
    (gradeParallel : ∀ action : ConcreteLeftAction × ConcreteRightAction,
      Le (parallelAction (concreteLeft.action action.1)
          (concreteRight.action action.2)).grade
        (parallelAction (abstractLeft.action (mapLeft action.1))
          (abstractRight.action (mapRight action.2))).grade) :
    ResourceRefinement
      (parallelSystemResources concreteLeft concreteRight)
      (parallelSystemResources abstractLeft abstractRight)
      (fun actions => (mapLeft actions.1, mapRight actions.2)) where
  owned := by
    intro capability owned
    cases owned with
    | inl owned => exact Or.inl (left.owned capability owned)
    | inr owned => exact Or.inr (right.owned capability owned)
  requires := by
    intro action capability required
    cases required with
    | inl required => exact Or.inl (left.requires action.1 capability required)
    | inr required => exact Or.inr (right.requires action.2 capability required)
  consumes := by
    intro action capability
    constructor
    · intro consumed
      cases consumed with
      | inl consumed => exact Or.inl ((left.consumes action.1 capability).mp consumed)
      | inr consumed => exact Or.inr ((right.consumes action.2 capability).mp consumed)
    · intro consumed
      cases consumed with
      | inl consumed => exact Or.inl ((left.consumes action.1 capability).mpr consumed)
      | inr consumed => exact Or.inr ((right.consumes action.2 capability).mpr consumed)
  transfers := by
    intro _ _
    rfl
  receives := by
    intro _ _
    rfl
  grade := by
    intro action
    exact gradeParallel action
  rely := by
    intro action fact relied
    cases relied with
    | inl leftRelied =>
        left
        exact ⟨left.rely action.1 fact leftRelied.1,
          fun abstractGuaranteed =>
            leftRelied.2 (right.guarantees action.2 fact abstractGuaranteed)⟩
    | inr rightRelied =>
        right
        exact ⟨right.rely action.2 fact rightRelied.1,
          fun abstractGuaranteed =>
            rightRelied.2 (left.guarantees action.1 fact abstractGuaranteed)⟩
  guarantees := by
    intro action fact guaranteed
    cases guaranteed with
    | inl guaranteed => exact Or.inl (left.guarantees action.1 fact guaranteed)
    | inr guaranteed => exact Or.inr (right.guarantees action.2 fact guaranteed)

theorem synchronized_rely_discharged
    {Capability Fact : Type}
    {left right : ActionResources Capability Fact}
    (compatible : SynchronizationCompatible left right) (fact : Fact) :
    ¬ (parallelAction left right).rely fact := by
  change ¬ ((left.rely fact ∧ ¬ right.guarantees fact) ∨
    (right.rely fact ∧ ¬ left.guarantees fact))
  intro residual
  cases residual with
  | inl residual => exact residual.2 (compatible.leftRely fact residual.1)
  | inr residual => exact residual.2 (compatible.rightRely fact residual.1)

theorem synchronized_transfer_exact
    {Capability Fact : Type}
    {left right : ActionResources Capability Fact}
    (compatible : SynchronizationCompatible left right) (capability : Capability) :
    (left.transfers capability ↔ right.receives capability) ∧
      (right.transfers capability ↔ left.receives capability) :=
  ⟨compatible.leftTransfer capability, compatible.rightTransfer capability⟩

theorem concreteGradeIncluded
    {ConcreteAction AbstractAction Capability Fact : Type}
    {concrete : SystemResources ConcreteAction Capability Fact}
    {abstract : SystemResources AbstractAction Capability Fact}
    {mapAction : ConcreteAction → AbstractAction}
    (refinement : ResourceRefinement concrete abstract mapAction)
    (action : ConcreteAction) :
    Le (concrete.action action).grade (abstract.action (mapAction action)).grade :=
  refinement.grade action

theorem concreteRelyIncluded
    {ConcreteAction AbstractAction Capability Fact : Type}
    {concrete : SystemResources ConcreteAction Capability Fact}
    {abstract : SystemResources AbstractAction Capability Fact}
    {mapAction : ConcreteAction → AbstractAction}
    (refinement : ResourceRefinement concrete abstract mapAction)
    (action : ConcreteAction) (fact : Fact) :
    (concrete.action action).rely fact →
      (abstract.action (mapAction action)).rely fact :=
  refinement.rely action fact

#print axioms partition_preserved
#print axioms ActionResourceRefinement.parallel
#print axioms liftOpenProductActionResources
#print axioms liftOpenProductResources
#print axioms liftResourceAwareParallel
#print axioms liftParallelAction
#print axioms synchronized_rely_discharged
#print axioms synchronized_transfer_exact
#print axioms concreteGradeIncluded
#print axioms concreteRelyIncluded

end NMLT.Behavior.OpenResourceCongruence
