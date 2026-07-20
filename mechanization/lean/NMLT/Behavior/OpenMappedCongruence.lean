import NMLT.Behavior.OpenComposition

namespace NMLT.Behavior.OpenMappedCongruence

open NMLT.Behavior.OpenComposition

def mapOpenAction
    {ConcreteInternal AbstractInternal ConcretePort AbstractPort Message : Type}
    (mapInternal : ConcreteInternal → AbstractInternal)
    (mapPort : ConcretePort → AbstractPort) :
    OpenAction ConcreteInternal ConcretePort Message →
      OpenAction AbstractInternal AbstractPort Message
  | .internal name => .internal (mapInternal name)
  | .input port message => .input (mapPort port) message
  | .output port message => .output (mapPort port) message

/-- Label-mapped exact-action refinement. It combines operational simulation
    with the M11-001b boundary obligations: complete injective port renaming,
    direction preservation, contravariant assumptions, and covariant
    guarantees. A common `Message` type represents exact payload identity; no
    payload conversion or subtyping is admitted. -/
structure MappedRefinement
    {ConcreteInternal AbstractInternal ConcretePort AbstractPort Message Observation : Type}
    {concreteInterface : Interface ConcretePort Message}
    {abstractInterface : Interface AbstractPort Message}
    (concrete : System ConcreteInternal ConcretePort Message Observation concreteInterface)
    (abstract : System AbstractInternal AbstractPort Message Observation abstractInterface) where
  mapState : concrete.State → abstract.State
  stateSurjective : Function.Surjective mapState
  mapInternal : ConcreteInternal → AbstractInternal
  mapPort : ConcretePort → AbstractPort
  portInjective : Function.Injective mapPort
  portSurjective : Function.Surjective mapPort
  initial : ∀ {state}, concrete.init state → abstract.init (mapState state)
  observation : ∀ state, concrete.observe state = abstract.observe (mapState state)
  accepts : ∀ port,
    concreteInterface.accepts port ↔ abstractInterface.accepts (mapPort port)
  emits : ∀ port,
    concreteInterface.emits port ↔ abstractInterface.emits (mapPort port)
  assumption : ∀ port message,
    abstractInterface.assumption (mapPort port) message →
      concreteInterface.assumption port message
  guarantee : ∀ port message,
    concreteInterface.guarantee port message →
      abstractInterface.guarantee (mapPort port) message
  step : ∀ {before action after}, concrete.step before action after →
    abstract.step (mapState before) (mapOpenAction mapInternal mapPort action) (mapState after)

/-- Complete wiring correspondence after applying both boundary label maps. -/
structure MappedWiringEquivalent
    {ConcreteLeftPort AbstractLeftPort ConcreteRightPort AbstractRightPort : Type}
    (concrete : Wiring ConcreteLeftPort ConcreteRightPort)
    (abstract : Wiring AbstractLeftPort AbstractRightPort)
    (mapLeft : ConcreteLeftPort → AbstractLeftPort)
    (mapRight : ConcreteRightPort → AbstractRightPort) : Prop where
  leftToRight : ∀ leftPort rightPort,
    concrete.leftToRight leftPort rightPort ↔
      abstract.leftToRight (mapLeft leftPort) (mapRight rightPort)
  rightToLeft : ∀ rightPort leftPort,
    concrete.rightToLeft rightPort leftPort ↔
      abstract.rightToLeft (mapRight rightPort) (mapLeft leftPort)

namespace MappedWiringEquivalent

theorem leftConnected_iff
    {ConcreteLeftPort AbstractLeftPort ConcreteRightPort AbstractRightPort : Type}
    {concrete : Wiring ConcreteLeftPort ConcreteRightPort}
    {abstract : Wiring AbstractLeftPort AbstractRightPort}
    {mapLeft : ConcreteLeftPort → AbstractLeftPort}
    {mapRight : ConcreteRightPort → AbstractRightPort}
    (equivalent : MappedWiringEquivalent concrete abstract mapLeft mapRight)
    (rightSurjective : Function.Surjective mapRight) (port : ConcreteLeftPort) :
    concrete.leftConnected port ↔ abstract.leftConnected (mapLeft port) := by
  constructor
  · intro connected
    cases connected with
    | inl edge =>
        left
        obtain ⟨rightPort, wired⟩ := edge
        exact ⟨mapRight rightPort, (equivalent.leftToRight _ _).mp wired⟩
    | inr edge =>
        right
        obtain ⟨rightPort, wired⟩ := edge
        exact ⟨mapRight rightPort, (equivalent.rightToLeft _ _).mp wired⟩
  · intro connected
    cases connected with
    | inl edge =>
        left
        obtain ⟨abstractRight, wired⟩ := edge
        obtain ⟨rightPort, rfl⟩ := rightSurjective abstractRight
        exact ⟨rightPort, (equivalent.leftToRight _ _).mpr wired⟩
    | inr edge =>
        right
        obtain ⟨abstractRight, wired⟩ := edge
        obtain ⟨rightPort, rfl⟩ := rightSurjective abstractRight
        exact ⟨rightPort, (equivalent.rightToLeft _ _).mpr wired⟩

theorem rightConnected_iff
    {ConcreteLeftPort AbstractLeftPort ConcreteRightPort AbstractRightPort : Type}
    {concrete : Wiring ConcreteLeftPort ConcreteRightPort}
    {abstract : Wiring AbstractLeftPort AbstractRightPort}
    {mapLeft : ConcreteLeftPort → AbstractLeftPort}
    {mapRight : ConcreteRightPort → AbstractRightPort}
    (equivalent : MappedWiringEquivalent concrete abstract mapLeft mapRight)
    (leftSurjective : Function.Surjective mapLeft) (port : ConcreteRightPort) :
    concrete.rightConnected port ↔ abstract.rightConnected (mapRight port) := by
  constructor
  · intro connected
    cases connected with
    | inl edge =>
        left
        obtain ⟨leftPort, wired⟩ := edge
        exact ⟨mapLeft leftPort, (equivalent.leftToRight _ _).mp wired⟩
    | inr edge =>
        right
        obtain ⟨leftPort, wired⟩ := edge
        exact ⟨mapLeft leftPort, (equivalent.rightToLeft _ _).mp wired⟩
  · intro connected
    cases connected with
    | inl edge =>
        left
        obtain ⟨abstractLeft, wired⟩ := edge
        obtain ⟨leftPort, rfl⟩ := leftSurjective abstractLeft
        exact ⟨leftPort, (equivalent.leftToRight _ _).mpr wired⟩
    | inr edge =>
        right
        obtain ⟨abstractLeft, wired⟩ := edge
        obtain ⟨leftPort, rfl⟩ := leftSurjective abstractLeft
        exact ⟨leftPort, (equivalent.rightToLeft _ _).mpr wired⟩

end MappedWiringEquivalent

namespace MappedRefinement

def mapCompositeInternal
    {ConcreteLeftInternal AbstractLeftInternal ConcreteLeftPort AbstractLeftPort : Type}
    {ConcreteRightInternal AbstractRightInternal ConcreteRightPort AbstractRightPort Message : Type}
    (mapLeftInternal : ConcreteLeftInternal → AbstractLeftInternal)
    (mapLeftPort : ConcreteLeftPort → AbstractLeftPort)
    (mapRightInternal : ConcreteRightInternal → AbstractRightInternal)
    (mapRightPort : ConcreteRightPort → AbstractRightPort) :
    CompositeInternal ConcreteLeftInternal ConcreteLeftPort
        ConcreteRightInternal ConcreteRightPort Message →
      CompositeInternal AbstractLeftInternal AbstractLeftPort
        AbstractRightInternal AbstractRightPort Message
  | .fromLeft name => .fromLeft (mapLeftInternal name)
  | .fromRight name => .fromRight (mapRightInternal name)
  | .leftToRight leftPort rightPort message =>
      .leftToRight (mapLeftPort leftPort) (mapRightPort rightPort) message
  | .rightToLeft rightPort leftPort message =>
      .rightToLeft (mapRightPort rightPort) (mapLeftPort leftPort) message

theorem sumMapInjective
    {ConcreteLeftPort AbstractLeftPort ConcreteRightPort AbstractRightPort : Type}
    {mapLeft : ConcreteLeftPort → AbstractLeftPort}
    {mapRight : ConcreteRightPort → AbstractRightPort}
    (leftInjective : Function.Injective mapLeft)
    (rightInjective : Function.Injective mapRight) :
    Function.Injective (Sum.map mapLeft mapRight) := by
  intro first second equality
  cases first with
  | inl first =>
      cases second with
      | inl second => exact congrArg Sum.inl (leftInjective (Sum.inl.inj equality))
      | inr second => contradiction
  | inr first =>
      cases second with
      | inl second => contradiction
      | inr second => exact congrArg Sum.inr (rightInjective (Sum.inr.inj equality))

theorem sumMapSurjective
    {ConcreteLeftPort AbstractLeftPort ConcreteRightPort AbstractRightPort : Type}
    {mapLeft : ConcreteLeftPort → AbstractLeftPort}
    {mapRight : ConcreteRightPort → AbstractRightPort}
    (leftSurjective : Function.Surjective mapLeft)
    (rightSurjective : Function.Surjective mapRight) :
    Function.Surjective (Sum.map mapLeft mapRight) := by
  intro target
  cases target with
  | inl target =>
      obtain ⟨source, rfl⟩ := leftSurjective target
      exact ⟨.inl source, rfl⟩
  | inr target =>
      obtain ⟨source, rfl⟩ := rightSurjective target
      exact ⟨.inr source, rfl⟩

theorem parallelStepCongruence
    {ConcreteLeftInternal AbstractLeftInternal ConcreteLeftPort AbstractLeftPort : Type}
    {ConcreteRightInternal AbstractRightInternal ConcreteRightPort AbstractRightPort : Type}
    {Message LeftObservation RightObservation : Type}
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
    {concreteWiring : Wiring ConcreteLeftPort ConcreteRightPort}
    {abstractWiring : Wiring AbstractLeftPort AbstractRightPort}
    (left : MappedRefinement concreteLeft abstractLeft)
    (right : MappedRefinement concreteRight abstractRight)
    (wiring : MappedWiringEquivalent concreteWiring abstractWiring
      left.mapPort right.mapPort)
    {before after : concreteLeft.State × concreteRight.State}
    {action : OpenAction
      (CompositeInternal ConcreteLeftInternal ConcreteLeftPort
        ConcreteRightInternal ConcreteRightPort Message)
      (Sum ConcreteLeftPort ConcreteRightPort) Message} :
    ParallelStep concreteLeft concreteRight concreteWiring before action after →
      ParallelStep abstractLeft abstractRight abstractWiring
        (left.mapState before.1, right.mapState before.2)
        (mapOpenAction
          (mapCompositeInternal left.mapInternal left.mapPort
            right.mapInternal right.mapPort)
          (Sum.map left.mapPort right.mapPort) action)
        (left.mapState after.1, right.mapState after.2) := by
  intro transition
  cases transition with
  | fromLeftInternal edge => exact .fromLeftInternal (left.step edge)
  | fromRightInternal edge => exact .fromRightInternal (right.step edge)
  | fromLeftInput isolated edge =>
      exact .fromLeftInput
        ((not_congr (wiring.leftConnected_iff right.portSurjective _)).mp isolated)
        (left.step edge)
  | fromLeftOutput isolated edge =>
      exact .fromLeftOutput
        ((not_congr (wiring.leftConnected_iff right.portSurjective _)).mp isolated)
        (left.step edge)
  | fromRightInput isolated edge =>
      exact .fromRightInput
        ((not_congr (wiring.rightConnected_iff left.portSurjective _)).mp isolated)
        (right.step edge)
  | fromRightOutput isolated edge =>
      exact .fromRightOutput
        ((not_congr (wiring.rightConnected_iff left.portSurjective _)).mp isolated)
        (right.step edge)
  | synchronizeLeftToRight connected send receive =>
      exact .synchronizeLeftToRight ((wiring.leftToRight _ _).mp connected)
        (left.step send) (right.step receive)
  | synchronizeRightToLeft connected send receive =>
      exact .synchronizeRightToLeft ((wiring.rightToLeft _ _).mp connected)
        (right.step send) (left.step receive)

/-- Two-sided label-aware product lifting. Unlike the earlier exact-interface
    theorem, this statement carries both boundary maps and the composite
    assumption/guarantee variance obligations. -/
def liftParallel
    {ConcreteLeftInternal AbstractLeftInternal ConcreteLeftPort AbstractLeftPort : Type}
    {ConcreteRightInternal AbstractRightInternal ConcreteRightPort AbstractRightPort : Type}
    {Message LeftObservation RightObservation : Type}
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
    {concreteWiring : Wiring ConcreteLeftPort ConcreteRightPort}
    {abstractWiring : Wiring AbstractLeftPort AbstractRightPort}
    (left : MappedRefinement concreteLeft abstractLeft)
    (right : MappedRefinement concreteRight abstractRight)
    (wiring : MappedWiringEquivalent concreteWiring abstractWiring
      left.mapPort right.mapPort) :
    MappedRefinement
      (parallel concreteLeft concreteRight concreteWiring)
      (parallel abstractLeft abstractRight abstractWiring) where
  mapState := fun state => (left.mapState state.1, right.mapState state.2)
  stateSurjective := by
    intro state
    obtain ⟨leftState, leftMaps⟩ := left.stateSurjective state.1
    obtain ⟨rightState, rightMaps⟩ := right.stateSurjective state.2
    exact ⟨(leftState, rightState), Prod.ext leftMaps rightMaps⟩
  mapInternal := mapCompositeInternal left.mapInternal left.mapPort
    right.mapInternal right.mapPort
  mapPort := Sum.map left.mapPort right.mapPort
  portInjective := sumMapInjective left.portInjective right.portInjective
  portSurjective := sumMapSurjective left.portSurjective right.portSurjective
  initial := fun starts => ⟨left.initial starts.1, right.initial starts.2⟩
  observation := fun state => Prod.ext
    (left.observation state.1) (right.observation state.2)
  accepts := by
    intro port
    cases port with
    | inl port =>
        exact and_congr (left.accepts port)
          (not_congr (wiring.leftConnected_iff right.portSurjective port))
    | inr port =>
        exact and_congr (right.accepts port)
          (not_congr (wiring.rightConnected_iff left.portSurjective port))
  emits := by
    intro port
    cases port with
    | inl port =>
        exact and_congr (left.emits port)
          (not_congr (wiring.leftConnected_iff right.portSurjective port))
    | inr port =>
        exact and_congr (right.emits port)
          (not_congr (wiring.rightConnected_iff left.portSurjective port))
  assumption := by
    intro port message assumed
    cases port with
    | inl port =>
        exact ⟨left.assumption port message assumed.1,
          (not_congr (wiring.leftConnected_iff right.portSurjective port)).mpr assumed.2⟩
    | inr port =>
        exact ⟨right.assumption port message assumed.1,
          (not_congr (wiring.rightConnected_iff left.portSurjective port)).mpr assumed.2⟩
  guarantee := by
    intro port message guaranteed
    cases port with
    | inl port =>
        exact ⟨left.guarantee port message guaranteed.1,
          (not_congr (wiring.leftConnected_iff right.portSurjective port)).mp guaranteed.2⟩
    | inr port =>
        exact ⟨right.guarantee port message guaranteed.1,
          (not_congr (wiring.rightConnected_iff left.portSurjective port)).mp guaranteed.2⟩
  step := parallelStepCongruence left right wiring

inductive Reachable
    {Internal Port Message Observation : Type}
    {interface : Interface Port Message}
    (system : System Internal Port Message Observation interface) : system.State → Prop
  | initial {state} : system.init state → Reachable system state
  | step {before action after} : Reachable system before →
      system.step before action after → Reachable system after

def Invariant
    {Internal Port Message Observation : Type}
    {interface : Interface Port Message}
    (system : System Internal Port Message Observation interface)
    (predicate : system.State → Prop) : Prop :=
  ∀ state, Reachable system state → predicate state

theorem mapReachable
    {ConcreteInternal AbstractInternal ConcretePort AbstractPort Message Observation : Type}
    {concreteInterface : Interface ConcretePort Message}
    {abstractInterface : Interface AbstractPort Message}
    {concrete : System ConcreteInternal ConcretePort Message Observation concreteInterface}
    {abstract : System AbstractInternal AbstractPort Message Observation abstractInterface}
    (refinement : MappedRefinement concrete abstract) {state : concrete.State} :
    Reachable concrete state → Reachable abstract (refinement.mapState state) := by
  intro reachable
  induction reachable with
  | initial starts => exact .initial (refinement.initial starts)
  | step previous edge inductionHypothesis =>
      exact .step inductionHypothesis (refinement.step edge)

theorem transportInvariant
    {ConcreteInternal AbstractInternal ConcretePort AbstractPort Message Observation : Type}
    {concreteInterface : Interface ConcretePort Message}
    {abstractInterface : Interface AbstractPort Message}
    {concrete : System ConcreteInternal ConcretePort Message Observation concreteInterface}
    {abstract : System AbstractInternal AbstractPort Message Observation abstractInterface}
    (refinement : MappedRefinement concrete abstract)
    (predicate : abstract.State → Prop) (holds : Invariant abstract predicate) :
    Invariant concrete (predicate ∘ refinement.mapState) := by
  intro state reachable
  exact holds (refinement.mapState state) (refinement.mapReachable reachable)

theorem abstractPortCovered
    {ConcreteInternal AbstractInternal ConcretePort AbstractPort Message Observation : Type}
    {concreteInterface : Interface ConcretePort Message}
    {abstractInterface : Interface AbstractPort Message}
    {concrete : System ConcreteInternal ConcretePort Message Observation concreteInterface}
    {abstract : System AbstractInternal AbstractPort Message Observation abstractInterface}
    (refinement : MappedRefinement concrete abstract) (port : AbstractPort) :
    ∃ concretePort, refinement.mapPort concretePort = port :=
  refinement.portSurjective port

theorem abstractAssumptionIncluded
    {ConcreteInternal AbstractInternal ConcretePort AbstractPort Message Observation : Type}
    {concreteInterface : Interface ConcretePort Message}
    {abstractInterface : Interface AbstractPort Message}
    {concrete : System ConcreteInternal ConcretePort Message Observation concreteInterface}
    {abstract : System AbstractInternal AbstractPort Message Observation abstractInterface}
    (refinement : MappedRefinement concrete abstract) (port : ConcretePort) (message : Message) :
    abstractInterface.assumption (refinement.mapPort port) message →
      concreteInterface.assumption port message :=
  refinement.assumption port message

theorem concreteGuaranteeIncluded
    {ConcreteInternal AbstractInternal ConcretePort AbstractPort Message Observation : Type}
    {concreteInterface : Interface ConcretePort Message}
    {abstractInterface : Interface AbstractPort Message}
    {concrete : System ConcreteInternal ConcretePort Message Observation concreteInterface}
    {abstract : System AbstractInternal AbstractPort Message Observation abstractInterface}
    (refinement : MappedRefinement concrete abstract) (port : ConcretePort) (message : Message) :
    concreteInterface.guarantee port message →
      abstractInterface.guarantee (refinement.mapPort port) message :=
  refinement.guarantee port message

#print axioms MappedWiringEquivalent.leftConnected_iff
#print axioms MappedWiringEquivalent.rightConnected_iff
#print axioms MappedRefinement.parallelStepCongruence
#print axioms MappedRefinement.liftParallel
#print axioms MappedRefinement.transportInvariant
#print axioms MappedRefinement.abstractPortCovered
#print axioms MappedRefinement.abstractAssumptionIncluded
#print axioms MappedRefinement.concreteGuaranteeIncluded

end MappedRefinement

namespace Examples

inductive ConcreteLeftPort where | send
inductive AbstractLeftPort where | commit
inductive ConcreteRightPort where | receive
inductive AbstractRightPort where | accept

def outputInterface {Port : Type} : Interface Port Unit where
  accepts := fun _ => False
  emits := fun _ => True
  assumption := fun _ _ => False
  guarantee := fun _ _ => True
  assumption_is_input := fun impossible => impossible.elim
  guarantee_is_output := fun _ => True.intro
  directions_disjoint := fun _ impossible _ => impossible.elim

def inputInterface {Port : Type} : Interface Port Unit where
  accepts := fun _ => True
  emits := fun _ => False
  assumption := fun _ _ => True
  guarantee := fun _ _ => False
  assumption_is_input := fun _ => True.intro
  guarantee_is_output := fun impossible => impossible.elim
  directions_disjoint := fun _ _ impossible => impossible.elim

inductive ConcreteLeftStep : Unit → OpenAction Unit ConcreteLeftPort Unit → Unit → Prop
  | send : ConcreteLeftStep () (.output .send ()) ()

inductive AbstractLeftStep : Unit → OpenAction Unit AbstractLeftPort Unit → Unit → Prop
  | commit : AbstractLeftStep () (.output .commit ()) ()

inductive ConcreteRightStep : Unit → OpenAction Unit ConcreteRightPort Unit → Unit → Prop
  | receive : ConcreteRightStep () (.input .receive ()) ()

inductive AbstractRightStep : Unit → OpenAction Unit AbstractRightPort Unit → Unit → Prop
  | accept : AbstractRightStep () (.input .accept ()) ()

def concreteLeft : System Unit ConcreteLeftPort Unit Unit outputInterface where
  State := Unit
  init := fun _ => True
  step := ConcreteLeftStep
  observe := id
  step_admitted := by intro _ _ _ edge; cases edge; trivial

def abstractLeft : System Unit AbstractLeftPort Unit Unit outputInterface where
  State := Unit
  init := fun _ => True
  step := AbstractLeftStep
  observe := id
  step_admitted := by intro _ _ _ edge; cases edge; trivial

def concreteRight : System Unit ConcreteRightPort Unit Unit inputInterface where
  State := Unit
  init := fun _ => True
  step := ConcreteRightStep
  observe := id
  step_admitted := by intro _ _ _ edge; cases edge; trivial

def abstractRight : System Unit AbstractRightPort Unit Unit inputInterface where
  State := Unit
  init := fun _ => True
  step := AbstractRightStep
  observe := id
  step_admitted := by intro _ _ _ edge; cases edge; trivial

def leftRefinement : MappedRefinement concreteLeft abstractLeft where
  mapState := id
  stateSurjective := fun state => ⟨state, rfl⟩
  mapInternal := id
  mapPort := fun | .send => .commit
  portInjective := by intro first second _equality; cases first; cases second; rfl
  portSurjective := by intro port; cases port; exact ⟨.send, rfl⟩
  initial := id
  observation := fun _ => rfl
  accepts := by intro port; cases port; exact Iff.rfl
  emits := by intro port; cases port; exact Iff.rfl
  assumption := by intro port _ impossible; exact impossible.elim
  guarantee := by intro _ _ _; trivial
  step := by intro _ _ _ edge; cases edge; exact .commit

def rightRefinement : MappedRefinement concreteRight abstractRight where
  mapState := id
  stateSurjective := fun state => ⟨state, rfl⟩
  mapInternal := id
  mapPort := fun | .receive => .accept
  portInjective := by intro first second _equality; cases first; cases second; rfl
  portSurjective := by intro port; cases port; exact ⟨.receive, rfl⟩
  initial := id
  observation := fun _ => rfl
  accepts := by intro port; cases port; exact Iff.rfl
  emits := by intro port; cases port; exact Iff.rfl
  assumption := by intro _ _ _; trivial
  guarantee := by intro port _ impossible; exact impossible.elim
  step := by intro _ _ _ edge; cases edge; exact .accept

def concreteWiring : Wiring ConcreteLeftPort ConcreteRightPort where
  leftToRight := fun _ _ => True
  rightToLeft := fun _ _ => False

def abstractWiring : Wiring AbstractLeftPort AbstractRightPort where
  leftToRight := fun _ _ => True
  rightToLeft := fun _ _ => False

theorem wiringCorresponds : MappedWiringEquivalent concreteWiring abstractWiring
    leftRefinement.mapPort rightRefinement.mapPort where
  leftToRight := fun _ _ => Iff.rfl
  rightToLeft := fun _ _ => Iff.rfl

/-- Positive correspondence control with genuinely different concrete and
    abstract boundary-label types on both sides and a real synchronization. -/
theorem positiveMappedProduct : Nonempty (MappedRefinement
    (parallel concreteLeft concreteRight concreteWiring)
    (parallel abstractLeft abstractRight abstractWiring)) := by
  exact ⟨MappedRefinement.liftParallel leftRefinement rightRefinement wiringCorresponds⟩

theorem positiveConcreteSynchronization :
    ParallelStep concreteLeft concreteRight concreteWiring ((), ())
      (.internal (.leftToRight .send .receive ())) ((), ()) := by
  exact .synchronizeLeftToRight True.intro .send .receive

#print axioms positiveMappedProduct
#print axioms positiveConcreteSynchronization

end Examples

end NMLT.Behavior.OpenMappedCongruence
