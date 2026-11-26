import NMLT.Counterexamples.WiringPremise

/-! A fixed premise experiment over the existing static behavior definitions.
The artifact checker's product-formation policy is unchanged. -/
namespace NMLT.Discovery.CompositionPremises

open NMLT.Behavior.ResourceBehavior

/-- The existing lifting construction needs visibility of connected left actions,
not the other formation fields. This is a research lemma, not a decoder policy. -/
def liftWithVisibleConnections
    {Action PeerAction Capability Fact GradeAtom CObs AObs PObs : Type}
    {concrete : Behavior Action Capability Fact GradeAtom CObs}
    {abstract : Behavior Action Capability Fact GradeAtom AObs}
    {peer : Behavior PeerAction Capability Fact GradeAtom PObs}
    {concreteConnection abstractConnection : Action → PeerAction → Prop}
    (refinement : ResourceWeakRefinement concrete abstract)
    (wiring : WiringEquivalent concreteConnection abstractConnection)
    (visibleConnections : ∀ {a b}, concreteConnection a b → ¬ concrete.hidden a) :
    ResourceWeakRefinement
      (parallel concrete peer concreteConnection)
      (parallel abstract peer abstractConnection) where
  mapState := fun state => (refinement.mapState state.1, state.2)
  init := fun initial => ⟨refinement.init initial.1, initial.2⟩
  observe := fun observation => (refinement.observe observation.1, observation.2)
  observeState := fun state => Prod.ext (refinement.observeState state.1) rfl
  hiddenPreserved := by
    intro action
    cases action with
    | left action => exact refinement.hiddenPreserved action
    | right _ => exact Iff.rfl
    | sync _ _ => exact Iff.rfl
  hiddenStep := by
    intro before action after step hidden
    cases step with
    | fromLeft leftStep _ => exact Prod.ext (refinement.hiddenStep leftStep hidden) rfl
    | fromRight _ _ => exact False.elim hidden
    | synchronize _ _ _ => exact False.elim hidden
  visibleStep := by
    intro before action after step visible
    cases step with
    | fromLeft leftStep isolated =>
      exact ProductStep.fromLeft (refinement.visibleStep leftStep visible)
        (fun b connected => isolated b ((wiring.connected _ _).mpr connected))
    | fromRight rightStep isolated =>
      exact ProductStep.fromRight rightStep
        (fun a connected => isolated a ((wiring.connected _ _).mpr connected))
    | synchronize connected leftStep rightStep =>
      exact ProductStep.synchronize ((wiring.connected _ _).mp connected)
        (refinement.visibleStep leftStep (visibleConnections connected)) rightStep
  resources := by
    intro action
    cases action with
    | left action => exact refinement.resources action
    | right _ => exact ResourceRefines.refl _
    | sync a _ => exact ResourceRefines.parallel (refinement.resources a) (ResourceRefines.refl _)
  hiddenResources := by
    intro action hidden
    cases action with
    | left action => exact refinement.hiddenResources action hidden
    | right _ => exact False.elim hidden
    | sync _ _ => exact False.elim hidden

structure Problem where
  Action : Type
  PeerAction : Type
  Capability : Type
  Fact : Type
  GradeAtom : Type
  CObs : Type
  AObs : Type
  PObs : Type
  concrete : Behavior Action Capability Fact GradeAtom CObs
  abstract : Behavior Action Capability Fact GradeAtom AObs
  peer : Behavior PeerAction Capability Fact GradeAtom PObs
  concreteConnection : Action → PeerAction → Prop
  abstractConnection : Action → PeerAction → Prop

inductive Premise where
  | refinement | wiring | concreteFormation | abstractFormation
  deriving DecidableEq, Repr

def Problem.holds (p : Problem) : Premise → Prop
  | .refinement => Nonempty (ResourceWeakRefinement p.concrete p.abstract)
  | .wiring => WiringEquivalent p.concreteConnection p.abstractConnection
  | .concreteFormation => Composable p.concrete p.peer p.concreteConnection
  | .abstractFormation => Composable p.abstract p.peer p.abstractConnection

def Problem.conclusion (p : Problem) : Prop :=
  Nonempty (ResourceWeakRefinement
    (parallel p.concrete p.peer p.concreteConnection)
    (parallel p.abstract p.peer p.abstractConnection))

/-- The conclusion and model class are fixed across every subset. -/
def Sufficient (selected : List Premise) : Prop :=
  ∀ p : Problem, (∀ premise ∈ selected, p.holds premise) → p.conclusion

theorem sufficient_concrete (selected : List Premise)
    (r : .refinement ∈ selected) (w : .wiring ∈ selected)
    (c : .concreteFormation ∈ selected) : Sufficient selected := by
  intro p retained
  obtain ⟨refinement⟩ := retained .refinement r
  exact ⟨liftWithVisibleConnections refinement (retained .wiring w)
    (retained .concreteFormation c).hiddenLeft⟩

theorem sufficient_abstract (selected : List Premise)
    (r : .refinement ∈ selected) (w : .wiring ∈ selected)
    (a : .abstractFormation ∈ selected) : Sufficient selected := by
  intro p retained
  obtain ⟨refinement⟩ := retained .refinement r
  have wiring := retained .wiring w
  have formed := retained .abstractFormation a
  exact ⟨liftWithVisibleConnections refinement wiring (fun {left right} connected hidden =>
    formed.hiddenLeft ((wiring.connected left right).mp connected)
      ((refinement.hiddenPreserved left).mp hidden))⟩

inductive PeerAction where | receive | tick
  deriving DecidableEq, Repr

def left (enabled hidden : Bool) : Behavior Unit Unit Unit Unit Unit where
  State := Bool
  init := fun state => state = false
  step := fun before _ after => before = false ∧ after = true ∧ enabled = true
  observe := fun _ => ()
  hidden := fun _ => hidden = true
  direction := fun _ => .output
  payload := fun _ => "Unit"
  owns := fun _ => False
  resources := fun _ => ResourceProfile.empty

def peer : Behavior PeerAction Unit Unit Unit Unit where
  State := Bool
  init := fun state => state = false
  step := fun before _ after => before = false ∧ after = true
  observe := fun _ => ()
  hidden := fun _ => False
  direction | .receive => .input | .tick => .internal
  payload := fun _ => "Unit"
  owns := fun _ => False
  resources := fun _ => ResourceProfile.empty

def linked : Unit → PeerAction → Prop := fun _ action => action = .receive

theorem sameWiring : WiringEquivalent linked linked := ⟨fun _ _ => Iff.rfl⟩

theorem emptyCompatible : SynchronizationCompatible
    (ResourceProfile.empty : ResourceProfile Unit Unit Unit) ResourceProfile.empty where
  transfer := fun _ => Iff.rfl
  noReverseTransfer := fun _ => Iff.rfl
  senderRely := fun _ impossible => False.elim impossible
  receiverRely := fun _ impossible => False.elim impossible

theorem leftComposable (enabled : Bool) : Composable (left enabled false) peer linked where
  directions := by intro a b connected; cases connected; trivial
  payloads := by intro a b connected; rfl
  hiddenLeft := by intro a b connected hidden; cases hidden
  hiddenRight := by intro a b connected hidden; exact hidden
  capabilities := by intro capability impossible; exact False.elim impossible
  resources := by intro a b connected; exact ⟨fun _ => emptyCompatible, fun _ => emptyCompatible⟩

theorem concreteEnabled (hidden : Bool) :
    (parallel (left true hidden) peer linked).init (false, false) ∧
    (parallel (left true hidden) peer linked).step (false, false)
      (.sync () .receive) (true, true) :=
  ⟨⟨rfl, rfl⟩, ProductStep.synchronize rfl ⟨rfl, rfl, rfl⟩ ⟨rfl, rfl⟩⟩

/-- A spare peer action keeps the abstract countermodels enabled. -/
theorem abstractEnabled (hidden : Bool) :
    (parallel (left false hidden) peer linked).init (false, false) ∧
    (parallel (left false hidden) peer linked).step (false, false)
      (.right .tick) (false, true) := by
  refine ⟨⟨rfl, rfl⟩, ProductStep.fromRight ⟨rfl, rfl⟩ ?_⟩
  intro action connected
  cases connected

theorem synchronizedLeftStep
    {Action OtherAction Capability Fact GradeAtom LObs RObs : Type}
    {lhs : Behavior Action Capability Fact GradeAtom LObs}
    {rhs : Behavior OtherAction Capability Fact GradeAtom RObs}
    {connection : Action → OtherAction → Prop}
    {before after : lhs.State × rhs.State} {a : Action} {b : OtherAction}
    (step : ProductStep lhs rhs connection before (.sync a b) after) :
    lhs.step before.1 a after.1 := by
  cases before with
  | mk leftBefore rightBefore =>
    cases after with
    | mk leftAfter rightAfter =>
      cases step with
      | synchronize _ leftStep _ => exact leftStep

theorem noAbstractSync (hidden : Bool) (before after : Bool × Bool) :
    ¬ (parallel (left false hidden) peer linked).step before (.sync () .receive) after := by
  intro step
  have impossible := (synchronizedLeftStep step).2.2
  cases impossible

theorem noLift (hidden : Bool) :
    ¬ Nonempty (ResourceWeakRefinement
      (parallel (left true hidden) peer linked)
      (parallel (left false hidden) peer linked)) := by
  intro ⟨refinement⟩
  exact noAbstractSync hidden _ _
    (refinement.visibleStep (concreteEnabled hidden).2 (fun impossible => impossible))

def hiddenRefinement : ResourceWeakRefinement (left true true) (left false true) where
  mapState := fun _ => false
  init := fun _ => rfl
  observe := id
  observeState := fun _ => rfl
  hiddenPreserved := fun _ => Iff.rfl
  hiddenStep := fun _ _ => rfl
  visibleStep := fun _ visible => False.elim (visible rfl)
  resources := fun _ => ResourceRefines.refl _
  hiddenResources := fun _ _ => ResourceRefines.refl _

def basicProblem (hidden : Bool) : Problem where
  Action := Unit
  PeerAction := PeerAction
  Capability := Unit
  Fact := Unit
  GradeAtom := Unit
  CObs := Unit
  AObs := Unit
  PObs := Unit
  concrete := left true hidden
  abstract := left false hidden
  peer := peer
  concreteConnection := linked
  abstractConnection := linked

open NMLT.Examples.ResourceWorldTransfer in
def wiringProblem : Problem where
  Action := SenderAction
  PeerAction := ReceiverAction
  Capability := Capability
  Fact := ContractFact
  GradeAtom := GradeAtom
  CObs := Bool
  AObs := Bool
  PObs := Bool
  concrete := sender
  abstract := abstractSender
  peer := receiver
  concreteConnection := connection
  abstractConnection := NMLT.Counterexamples.WiringPremise.disconnected

theorem refuted_without_refinement (selected : List Premise)
    (missing : .refinement ∉ selected) : ¬ Sufficient selected := by
  intro sufficient
  apply noLift false
  apply sufficient (basicProblem false)
  intro premise member
  cases premise with
  | refinement => exact False.elim (missing member)
  | wiring => exact sameWiring
  | concreteFormation => exact leftComposable true
  | abstractFormation => exact leftComposable false

theorem refuted_without_wiring (selected : List Premise)
    (missing : .wiring ∉ selected) : ¬ Sufficient selected := by
  intro sufficient
  apply NMLT.Counterexamples.WiringPremise.disconnectedWiring_blocksLift
  apply sufficient wiringProblem
  intro premise member
  cases premise with
  | refinement => exact ⟨NMLT.Examples.ResourceWorldTransfer.senderRefinement⟩
  | wiring => exact False.elim (missing member)
  | concreteFormation => exact NMLT.Examples.ResourceWorldTransfer.senderReceiverComposable
  | abstractFormation => exact NMLT.Counterexamples.WiringPremise.disconnectedComposable

theorem refuted_without_formation (selected : List Premise)
    (missingC : .concreteFormation ∉ selected)
    (missingA : .abstractFormation ∉ selected) : ¬ Sufficient selected := by
  intro sufficient
  apply noLift true
  apply sufficient (basicProblem true)
  intro premise member
  cases premise with
  | refinement => exact ⟨hiddenRefinement⟩
  | wiring => exact sameWiring
  | concreteFormation => exact False.elim (missingC member)
  | abstractFormation => exact False.elim (missingA member)

/-- All 16 subsets, with bit order refinement, wiring, concrete, abstract. -/
theorem case00 : ¬ Sufficient [] := refuted_without_refinement _ (by decide)
theorem case01 : ¬ Sufficient [.refinement] := refuted_without_wiring _ (by decide)
theorem case02 : ¬ Sufficient [.wiring] := refuted_without_refinement _ (by decide)
theorem case03 : ¬ Sufficient [.refinement, .wiring] :=
  refuted_without_formation _ (by decide) (by decide)
theorem case04 : ¬ Sufficient [.concreteFormation] := refuted_without_refinement _ (by decide)
theorem case05 : ¬ Sufficient [.refinement, .concreteFormation] :=
  refuted_without_wiring _ (by decide)
theorem case06 : ¬ Sufficient [.wiring, .concreteFormation] :=
  refuted_without_refinement _ (by decide)
theorem case07 : Sufficient [.refinement, .wiring, .concreteFormation] :=
  sufficient_concrete _ (by decide) (by decide) (by decide)
theorem case08 : ¬ Sufficient [.abstractFormation] := refuted_without_refinement _ (by decide)
theorem case09 : ¬ Sufficient [.refinement, .abstractFormation] := refuted_without_wiring _ (by decide)
theorem case10 : ¬ Sufficient [.wiring, .abstractFormation] := refuted_without_refinement _ (by decide)
theorem case11 : Sufficient [.refinement, .wiring, .abstractFormation] :=
  sufficient_abstract _ (by decide) (by decide) (by decide)
theorem case12 : ¬ Sufficient [.concreteFormation, .abstractFormation] :=
  refuted_without_refinement _ (by decide)
theorem case13 : ¬ Sufficient [.refinement, .concreteFormation, .abstractFormation] :=
  refuted_without_wiring _ (by decide)
theorem case14 : ¬ Sufficient [.wiring, .concreteFormation, .abstractFormation] :=
  refuted_without_refinement _ (by decide)
theorem case15 : Sufficient [.refinement, .wiring, .concreteFormation, .abstractFormation] :=
  sufficient_concrete _ (by decide) (by decide) (by decide)

end NMLT.Discovery.CompositionPremises
