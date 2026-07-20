import NMLT.Behavior.Refinement

namespace NMLT.Behavior.OpenComposition

/-! The companion translation-validation vectors are
    `mechanization/vectors/m11-open-congruence-v1.json`, schema
    `nmlt-m11-open-congruence-v1`. They bind the theorem handles and Rust
    controls but do not constitute a proof about the Rust compiler or runtime. -/

/-- The polarity of a boundary port. Internal actions have no port and are
    therefore not candidates for synchronization. -/
inductive Direction where
  | input
  | output
deriving DecidableEq

/-- An open action is either internal, supplied by the environment, or
    emitted to the environment. Keeping `internal` separate from port actions
    makes RFC 0008's no-hidden-boundary premise true by construction. -/
inductive OpenAction (Internal Port Message : Type) where
  | internal (name : Internal)
  | input (port : Port) (message : Message)
  | output (port : Port) (message : Message)

/-- A small assume/guarantee interface. `assumption p m` says which input
    messages the environment may provide; `guarantee p m` says which output
    messages the component may emit. The classification fields prevent a
    contract from assigning messages to ports of the wrong polarity. -/
structure Interface (Port Message : Type) where
  accepts : Port → Prop
  emits : Port → Prop
  assumption : Port → Message → Prop
  guarantee : Port → Message → Prop
  assumption_is_input : ∀ {port message}, assumption port message → accepts port
  guarantee_is_output : ∀ {port message}, guarantee port message → emits port
  directions_disjoint : ∀ port, accepts port → emits port → False

def Interface.Admits {Internal Port Message : Type}
    (interface : Interface Port Message) : OpenAction Internal Port Message → Prop
  | .internal _ => True
  | .input port message => interface.assumption port message
  | .output port message => interface.guarantee port message

/-- A deliberately small open transition system. It records neither grades nor
    fairness; those require additional preservation obligations before any
    liveness theorem can be stated. -/
structure System (Internal Port Message Observation : Type)
    (interface : Interface Port Message) where
  State : Type
  init : State → Prop
  step : State → OpenAction Internal Port Message → State → Prop
  observe : State → Observation
  step_admitted : ∀ {before action after}, step before action after → interface.Admits action

/-- An open system is receptive when every message on every declared input
    port is enabled at every state. This is the intentionally strong,
    input-enabled fragment used by the first M11 composition theorem. A
    system whose assumption excludes a message on an accepted port therefore
    cannot establish this property, because every step must be admitted. -/
def InputReceptive {Internal Port Message Observation : Type}
    {interface : Interface Port Message}
    (system : System Internal Port Message Observation interface) : Prop :=
  ∀ state port message, interface.accepts port →
    ∃ after, system.step state (.input port message) after

/-- A bidirectional wiring relation. The two fields distinguish which side is
    the sender, avoiding an implicit symmetry assumption. -/
structure Wiring (LeftPort RightPort : Type) where
  leftToRight : LeftPort → RightPort → Prop
  rightToLeft : RightPort → LeftPort → Prop

def Wiring.leftConnected {LeftPort RightPort : Type}
    (wiring : Wiring LeftPort RightPort) (port : LeftPort) : Prop :=
  (∃ peer, wiring.leftToRight port peer) ∨
    (∃ peer, wiring.rightToLeft peer port)

def Wiring.rightConnected {LeftPort RightPort : Type}
    (wiring : Wiring LeftPort RightPort) (port : RightPort) : Prop :=
  (∃ peer, wiring.leftToRight peer port) ∨
    (∃ peer, wiring.rightToLeft port peer)

/-- The bounded theorem uses exact connection coverage. This is stronger than
    a pointwise condition over mapped labels: it rules out an additional
    abstract connection that could block a concrete peer-only interleaving. -/
structure WiringEquivalent {LeftPort RightPort : Type}
    (concrete abstract : Wiring LeftPort RightPort) : Prop where
  leftToRight : ∀ leftPort rightPort,
    concrete.leftToRight leftPort rightPort ↔ abstract.leftToRight leftPort rightPort
  rightToLeft : ∀ rightPort leftPort,
    concrete.rightToLeft rightPort leftPort ↔ abstract.rightToLeft rightPort leftPort

namespace WiringEquivalent

theorem reflectLeftIsolation {LeftPort RightPort : Type}
    {concrete abstract : Wiring LeftPort RightPort}
    (equivalent : WiringEquivalent concrete abstract) {port : LeftPort} :
    ¬ concrete.leftConnected port → ¬ abstract.leftConnected port := by
  intro concreteOpen abstractConnected
  apply concreteOpen
  cases abstractConnected with
  | inl connected =>
      left
      obtain ⟨rightPort, edge⟩ := connected
      exact ⟨rightPort, (equivalent.leftToRight port rightPort).mpr edge⟩
  | inr connected =>
      right
      obtain ⟨rightPort, edge⟩ := connected
      exact ⟨rightPort, (equivalent.rightToLeft rightPort port).mpr edge⟩

/-- Global peer-isolation reflection. Without this property an abstract wire
    outside the concrete connection image can block a peer step that was
    independent in the concrete product. -/
theorem reflectRightIsolation {LeftPort RightPort : Type}
    {concrete abstract : Wiring LeftPort RightPort}
    (equivalent : WiringEquivalent concrete abstract) {port : RightPort} :
    ¬ concrete.rightConnected port → ¬ abstract.rightConnected port := by
  intro concreteOpen abstractConnected
  apply concreteOpen
  cases abstractConnected with
  | inl connected =>
      left
      obtain ⟨leftPort, edge⟩ := connected
      exact ⟨leftPort, (equivalent.leftToRight leftPort port).mpr edge⟩
  | inr connected =>
      right
      obtain ⟨leftPort, edge⟩ := connected
      exact ⟨leftPort, (equivalent.rightToLeft port leftPort).mpr edge⟩

end WiringEquivalent

/-- Static compatibility requires each wire to connect output to input and
    requires every sender guarantee to discharge the receiver assumption. -/
structure InterfaceCompatible {LeftPort RightPort Message : Type}
    (left : Interface LeftPort Message) (right : Interface RightPort Message)
    (wiring : Wiring LeftPort RightPort) : Prop where
  leftToRightPorts : ∀ {leftPort rightPort}, wiring.leftToRight leftPort rightPort →
    left.emits leftPort ∧ right.accepts rightPort
  rightToLeftPorts : ∀ {rightPort leftPort}, wiring.rightToLeft rightPort leftPort →
    right.emits rightPort ∧ left.accepts leftPort
  leftToRightContract : ∀ {leftPort rightPort message},
    wiring.leftToRight leftPort rightPort → left.guarantee leftPort message →
      right.assumption rightPort message
  rightToLeftContract : ∀ {rightPort leftPort message},
    wiring.rightToLeft rightPort leftPort → right.guarantee rightPort message →
      left.assumption leftPort message

/-- A composition is well formed when its interfaces are compatible and both
    components are input receptive. -/
structure Composable
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    (left : System LeftInternal LeftPort Message LeftObservation leftInterface)
    (right : System RightInternal RightPort Message RightObservation rightInterface)
    (wiring : Wiring LeftPort RightPort) : Prop where
  interfaces : InterfaceCompatible leftInterface rightInterface wiring
  leftReceptive : InputReceptive left
  rightReceptive : InputReceptive right

/-- Internal names in the product remember whether a component stepped alone
    or the two components synchronized. -/
inductive CompositeInternal
    (LeftInternal LeftPort RightInternal RightPort Message : Type) where
  | fromLeft (name : LeftInternal)
  | fromRight (name : RightInternal)
  | leftToRight (leftPort : LeftPort) (rightPort : RightPort) (message : Message)
  | rightToLeft (rightPort : RightPort) (leftPort : LeftPort) (message : Message)

def parallelInterface {LeftPort RightPort Message : Type}
    (left : Interface LeftPort Message) (right : Interface RightPort Message)
    (wiring : Wiring LeftPort RightPort) : Interface (Sum LeftPort RightPort) Message where
  accepts
    | .inl port => left.accepts port ∧ ¬ wiring.leftConnected port
    | .inr port => right.accepts port ∧ ¬ wiring.rightConnected port
  emits
    | .inl port => left.emits port ∧ ¬ wiring.leftConnected port
    | .inr port => right.emits port ∧ ¬ wiring.rightConnected port
  assumption
    | .inl port, message => left.assumption port message ∧ ¬ wiring.leftConnected port
    | .inr port, message => right.assumption port message ∧ ¬ wiring.rightConnected port
  guarantee
    | .inl port, message => left.guarantee port message ∧ ¬ wiring.leftConnected port
    | .inr port, message => right.guarantee port message ∧ ¬ wiring.rightConnected port
  assumption_is_input := by
    intro port message assumed
    cases port with
    | inl port => exact ⟨left.assumption_is_input assumed.1, assumed.2⟩
    | inr port => exact ⟨right.assumption_is_input assumed.1, assumed.2⟩
  guarantee_is_output := by
    intro port message guaranteed
    cases port with
    | inl port => exact ⟨left.guarantee_is_output guaranteed.1, guaranteed.2⟩
    | inr port => exact ⟨right.guarantee_is_output guaranteed.1, guaranteed.2⟩
  directions_disjoint := by
    intro port accepted emitted
    cases port with
    | inl port => exact left.directions_disjoint port accepted.1 emitted.1
    | inr port => exact right.directions_disjoint port accepted.1 emitted.1

/-- Operational rules for the supported synchronous product. Connected
    boundary actions must synchronize; unconnected actions remain exposed.
    Internal actions can only interleave and can never use a wire. -/
inductive ParallelStep
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    (left : System LeftInternal LeftPort Message LeftObservation leftInterface)
    (right : System RightInternal RightPort Message RightObservation rightInterface)
    (wiring : Wiring LeftPort RightPort) :
    (left.State × right.State) →
      OpenAction
        (CompositeInternal LeftInternal LeftPort RightInternal RightPort Message)
        (Sum LeftPort RightPort) Message →
      (left.State × right.State) → Prop where
  | fromLeftInternal {leftBefore leftAfter rightState name} :
      left.step leftBefore (.internal name) leftAfter →
      ParallelStep left right wiring (leftBefore, rightState)
        (.internal (.fromLeft name)) (leftAfter, rightState)
  | fromRightInternal {leftState rightBefore rightAfter name} :
      right.step rightBefore (.internal name) rightAfter →
      ParallelStep left right wiring (leftState, rightBefore)
        (.internal (.fromRight name)) (leftState, rightAfter)
  | fromLeftInput {leftBefore leftAfter rightState port message} :
      ¬ wiring.leftConnected port →
      left.step leftBefore (.input port message) leftAfter →
      ParallelStep left right wiring (leftBefore, rightState)
        (.input (.inl port) message) (leftAfter, rightState)
  | fromLeftOutput {leftBefore leftAfter rightState port message} :
      ¬ wiring.leftConnected port →
      left.step leftBefore (.output port message) leftAfter →
      ParallelStep left right wiring (leftBefore, rightState)
        (.output (.inl port) message) (leftAfter, rightState)
  | fromRightInput {leftState rightBefore rightAfter port message} :
      ¬ wiring.rightConnected port →
      right.step rightBefore (.input port message) rightAfter →
      ParallelStep left right wiring (leftState, rightBefore)
        (.input (.inr port) message) (leftState, rightAfter)
  | fromRightOutput {leftState rightBefore rightAfter port message} :
      ¬ wiring.rightConnected port →
      right.step rightBefore (.output port message) rightAfter →
      ParallelStep left right wiring (leftState, rightBefore)
        (.output (.inr port) message) (leftState, rightAfter)
  | synchronizeLeftToRight
      {leftBefore leftAfter rightBefore rightAfter leftPort rightPort message} :
      wiring.leftToRight leftPort rightPort →
      left.step leftBefore (.output leftPort message) leftAfter →
      right.step rightBefore (.input rightPort message) rightAfter →
      ParallelStep left right wiring (leftBefore, rightBefore)
        (.internal (.leftToRight leftPort rightPort message)) (leftAfter, rightAfter)
  | synchronizeRightToLeft
      {leftBefore leftAfter rightBefore rightAfter rightPort leftPort message} :
      wiring.rightToLeft rightPort leftPort →
      right.step rightBefore (.output rightPort message) rightAfter →
      left.step leftBefore (.input leftPort message) leftAfter →
      ParallelStep left right wiring (leftBefore, rightBefore)
        (.internal (.rightToLeft rightPort leftPort message)) (leftAfter, rightAfter)

/-- The synchronous product as another open system. Its remaining boundary is
    exactly the disjoint sum of unconnected component ports. -/
def parallel
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    (left : System LeftInternal LeftPort Message LeftObservation leftInterface)
    (right : System RightInternal RightPort Message RightObservation rightInterface)
    (wiring : Wiring LeftPort RightPort) :
    System
      (CompositeInternal LeftInternal LeftPort RightInternal RightPort Message)
      (Sum LeftPort RightPort) Message (LeftObservation × RightObservation)
      (parallelInterface leftInterface rightInterface wiring) where
  State := left.State × right.State
  init := fun state => left.init state.1 ∧ right.init state.2
  step := ParallelStep left right wiring
  observe := fun state => (left.observe state.1, right.observe state.2)
  step_admitted := by
    intro before action after transition
    cases transition with
    | fromLeftInternal edge => trivial
    | fromRightInternal edge => trivial
    | fromLeftInput isolated edge => exact ⟨left.step_admitted edge, isolated⟩
    | fromLeftOutput isolated edge => exact ⟨left.step_admitted edge, isolated⟩
    | fromRightInput isolated edge => exact ⟨right.step_admitted edge, isolated⟩
    | fromRightOutput isolated edge => exact ⟨right.step_admitted edge, isolated⟩
    | synchronizeLeftToRight connected send receive => trivial
    | synchronizeRightToLeft connected send receive => trivial

/-- A well-formed product is receptive on every input that remains exposed.
    Connected inputs are absent from the product interface and are handled by
    the synchronization rules instead. -/
theorem parallelInputReceptive
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    {left : System LeftInternal LeftPort Message LeftObservation leftInterface}
    {right : System RightInternal RightPort Message RightObservation rightInterface}
    {wiring : Wiring LeftPort RightPort}
    (ready : Composable left right wiring) : InputReceptive (parallel left right wiring) := by
  intro state port message accepted
  cases port with
  | inl leftPort =>
      obtain ⟨leftAfter, step⟩ :=
        ready.leftReceptive state.1 leftPort message accepted.1
      exact ⟨(leftAfter, state.2), .fromLeftInput accepted.2 step⟩
  | inr rightPort =>
      obtain ⟨rightAfter, step⟩ :=
        ready.rightReceptive state.2 rightPort message accepted.1
      exact ⟨(state.1, rightAfter), .fromRightInput accepted.2 step⟩

/-- Exact-action, state-surjective forward refinement for the supported
    open-system fragment. The composition theorem below instantiates concrete
    and abstract components with the same interface contract. Surjectivity is
    needed to transport the deliberately global input-receptiveness property.
    This is intentionally stronger than `Behavior.Refinement`: it neither
    hides a concrete action nor expands it into multiple abstract steps. -/
structure StrongRefinement
    {Internal Port Message Observation : Type}
    {concreteInterface abstractInterface : Interface Port Message}
    (concrete : System Internal Port Message Observation concreteInterface)
    (abstract : System Internal Port Message Observation abstractInterface) where
  mapState : concrete.State → abstract.State
  stateSurjective : Function.Surjective mapState
  initial : ∀ {state}, concrete.init state → abstract.init (mapState state)
  observation : ∀ state, concrete.observe state = abstract.observe (mapState state)
  step : ∀ {before action after}, concrete.step before action after →
    abstract.step (mapState before) action (mapState after)

namespace StrongRefinement

def identity
    {Internal Port Message Observation : Type}
    {interface : Interface Port Message}
    (system : System Internal Port Message Observation interface) :
    StrongRefinement system system where
  mapState := id
  stateSurjective := fun state => ⟨state, rfl⟩
  initial := id
  observation := fun _ => rfl
  step := id

def compose
    {Internal Port Message Observation : Type}
    {firstInterface secondInterface thirdInterface : Interface Port Message}
    {first : System Internal Port Message Observation firstInterface}
    {second : System Internal Port Message Observation secondInterface}
    {third : System Internal Port Message Observation thirdInterface}
    (left : StrongRefinement first second) (right : StrongRefinement second third) :
    StrongRefinement first third where
  mapState := right.mapState ∘ left.mapState
  stateSurjective := right.stateSurjective.comp left.stateSurjective
  initial := fun starts => right.initial (left.initial starts)
  observation := fun state =>
    (left.observation state).trans (right.observation (left.mapState state))
  step := fun edge => right.step (left.step edge)

theorem preservesInputReceptive
    {Internal Port Message Observation : Type}
    {interface : Interface Port Message}
    {concrete abstract : System Internal Port Message Observation interface}
    (refinement : StrongRefinement concrete abstract)
    (receptive : InputReceptive concrete) : InputReceptive abstract := by
  intro state port message accepted
  obtain ⟨concreteState, rfl⟩ := refinement.stateSurjective state
  obtain ⟨after, step⟩ := receptive concreteState port message accepted
  exact ⟨refinement.mapState after, refinement.step step⟩

/-- Compatibility plus peer receptiveness ensures that an admitted output has
    a synchronized product step; it is not merely a static port check. -/
theorem outputCanSynchronize
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    {left : System LeftInternal LeftPort Message LeftObservation leftInterface}
    {right : System RightInternal RightPort Message RightObservation rightInterface}
    {wiring : Wiring LeftPort RightPort}
    (ready : Composable left right wiring)
    {leftBefore leftAfter : left.State} {rightBefore : right.State}
    {leftPort : LeftPort} {rightPort : RightPort} {message : Message}
    (connected : wiring.leftToRight leftPort rightPort)
    (send : left.step leftBefore (.output leftPort message) leftAfter) :
    ∃ rightAfter, ParallelStep left right wiring (leftBefore, rightBefore)
      (.internal (.leftToRight leftPort rightPort message)) (leftAfter, rightAfter) := by
  have guaranteed : leftInterface.guarantee leftPort message := left.step_admitted send
  have accepted : rightInterface.accepts rightPort :=
    (ready.interfaces.leftToRightPorts connected).2
  have _assumed : rightInterface.assumption rightPort message :=
    ready.interfaces.leftToRightContract connected guaranteed
  obtain ⟨rightAfter, receive⟩ := ready.rightReceptive rightBefore rightPort message accepted
  exact ⟨rightAfter, .synchronizeLeftToRight connected send receive⟩

/-- The symmetric enabled-synchronization result for a right-side guaranteed
    output connected to a receptive left-side input. -/
theorem rightOutputCanSynchronize
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    {left : System LeftInternal LeftPort Message LeftObservation leftInterface}
    {right : System RightInternal RightPort Message RightObservation rightInterface}
    {wiring : Wiring LeftPort RightPort}
    (ready : Composable left right wiring)
    {leftBefore : left.State} {rightBefore rightAfter : right.State}
    {rightPort : RightPort} {leftPort : LeftPort} {message : Message}
    (connected : wiring.rightToLeft rightPort leftPort)
    (send : right.step rightBefore (.output rightPort message) rightAfter) :
    ∃ leftAfter, ParallelStep left right wiring (leftBefore, rightBefore)
      (.internal (.rightToLeft rightPort leftPort message)) (leftAfter, rightAfter) := by
  have guaranteed : rightInterface.guarantee rightPort message := right.step_admitted send
  have accepted : leftInterface.accepts leftPort :=
    (ready.interfaces.rightToLeftPorts connected).2
  have _assumed : leftInterface.assumption leftPort message :=
    ready.interfaces.rightToLeftContract connected guaranteed
  obtain ⟨leftAfter, receive⟩ := ready.leftReceptive leftBefore leftPort message accepted
  exact ⟨leftAfter, .synchronizeRightToLeft connected send receive⟩

theorem preservesComposableLeft
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    {concrete abstract : System LeftInternal LeftPort Message LeftObservation leftInterface}
    {peer : System RightInternal RightPort Message RightObservation rightInterface}
    {wiring : Wiring LeftPort RightPort}
    (refinement : StrongRefinement concrete abstract)
    (ready : Composable concrete peer wiring) : Composable abstract peer wiring where
  interfaces := ready.interfaces
  leftReceptive := refinement.preservesInputReceptive ready.leftReceptive
  rightReceptive := ready.rightReceptive

theorem preservesComposableLeftUnderWiring
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    {concrete abstract : System LeftInternal LeftPort Message LeftObservation leftInterface}
    {peer : System RightInternal RightPort Message RightObservation rightInterface}
    {concreteWiring abstractWiring : Wiring LeftPort RightPort}
    (refinement : StrongRefinement concrete abstract)
    (equivalent : WiringEquivalent concreteWiring abstractWiring)
    (ready : Composable concrete peer concreteWiring) :
    Composable abstract peer abstractWiring where
  interfaces := {
    leftToRightPorts := by
      intro leftPort rightPort connected
      exact ready.interfaces.leftToRightPorts
        ((equivalent.leftToRight leftPort rightPort).mpr connected)
    rightToLeftPorts := by
      intro rightPort leftPort connected
      exact ready.interfaces.rightToLeftPorts
        ((equivalent.rightToLeft rightPort leftPort).mpr connected)
    leftToRightContract := by
      intro leftPort rightPort message connected guaranteed
      exact ready.interfaces.leftToRightContract
        ((equivalent.leftToRight leftPort rightPort).mpr connected) guaranteed
    rightToLeftContract := by
      intro rightPort leftPort message connected guaranteed
      exact ready.interfaces.rightToLeftContract
        ((equivalent.rightToLeft rightPort leftPort).mpr connected) guaranteed
  }
  leftReceptive := refinement.preservesInputReceptive ready.leftReceptive
  rightReceptive := ready.rightReceptive

/-- Every concrete product step has an exact abstract product step under the
    lifted state map. This is the operational core of congruence. -/
theorem parallelStepCongruence
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    {concrete abstract : System LeftInternal LeftPort Message LeftObservation leftInterface}
    {peer : System RightInternal RightPort Message RightObservation rightInterface}
    {concreteWiring abstractWiring : Wiring LeftPort RightPort}
    (refinement : StrongRefinement concrete abstract)
    (equivalent : WiringEquivalent concreteWiring abstractWiring)
    {before after : concrete.State × peer.State}
    {action : OpenAction
      (CompositeInternal LeftInternal LeftPort RightInternal RightPort Message)
      (Sum LeftPort RightPort) Message} :
    ParallelStep concrete peer concreteWiring before action after →
      ParallelStep abstract peer abstractWiring
        (refinement.mapState before.1, before.2) action
        (refinement.mapState after.1, after.2) := by
  intro transition
  cases transition with
  | fromLeftInternal edge => exact .fromLeftInternal (refinement.step edge)
  | fromRightInternal edge => exact .fromRightInternal edge
  | fromLeftInput isolated edge =>
      exact .fromLeftInput (equivalent.reflectLeftIsolation isolated) (refinement.step edge)
  | fromLeftOutput isolated edge =>
      exact .fromLeftOutput (equivalent.reflectLeftIsolation isolated) (refinement.step edge)
  | fromRightInput isolated edge =>
      exact .fromRightInput (equivalent.reflectRightIsolation isolated) edge
  | fromRightOutput isolated edge =>
      exact .fromRightOutput (equivalent.reflectRightIsolation isolated) edge
  | synchronizeLeftToRight connected send receive =>
      exact .synchronizeLeftToRight
        ((equivalent.leftToRight _ _).mp connected) (refinement.step send) receive
  | synchronizeRightToLeft connected send receive =>
      exact .synchronizeRightToLeft
        ((equivalent.rightToLeft _ _).mp connected) send (refinement.step receive)

def liftParallel
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    {concrete abstract : System LeftInternal LeftPort Message LeftObservation leftInterface}
    {peer : System RightInternal RightPort Message RightObservation rightInterface}
    {concreteWiring abstractWiring : Wiring LeftPort RightPort}
    (refinement : StrongRefinement concrete abstract)
    (equivalent : WiringEquivalent concreteWiring abstractWiring) :
    StrongRefinement (parallel concrete peer concreteWiring)
      (parallel abstract peer abstractWiring) where
  mapState := fun state => (refinement.mapState state.1, state.2)
  stateSurjective := by
    intro state
    obtain ⟨concreteState, mapped⟩ := refinement.stateSurjective state.1
    exact ⟨(concreteState, state.2), Prod.ext mapped rfl⟩
  initial := fun initial => ⟨refinement.initial initial.1, initial.2⟩
  observation := fun state => congrArg (fun observation => (observation, peer.observe state.2))
    (refinement.observation state.1)
  step := parallelStepCongruence refinement equivalent

/-- Symmetric operational lifting for refinement of the right component. -/
theorem parallelStepCongruenceRight
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    {peer : System LeftInternal LeftPort Message LeftObservation leftInterface}
    {concrete abstract : System RightInternal RightPort Message RightObservation rightInterface}
    {concreteWiring abstractWiring : Wiring LeftPort RightPort}
    (refinement : StrongRefinement concrete abstract)
    (equivalent : WiringEquivalent concreteWiring abstractWiring)
    {before after : peer.State × concrete.State}
    {action : OpenAction
      (CompositeInternal LeftInternal LeftPort RightInternal RightPort Message)
      (Sum LeftPort RightPort) Message} :
    ParallelStep peer concrete concreteWiring before action after →
      ParallelStep peer abstract abstractWiring
        (before.1, refinement.mapState before.2) action
        (after.1, refinement.mapState after.2) := by
  intro transition
  cases transition with
  | fromLeftInternal edge => exact .fromLeftInternal edge
  | fromRightInternal edge => exact .fromRightInternal (refinement.step edge)
  | fromLeftInput isolated edge =>
      exact .fromLeftInput (equivalent.reflectLeftIsolation isolated) edge
  | fromLeftOutput isolated edge =>
      exact .fromLeftOutput (equivalent.reflectLeftIsolation isolated) edge
  | fromRightInput isolated edge =>
      exact .fromRightInput (equivalent.reflectRightIsolation isolated) (refinement.step edge)
  | fromRightOutput isolated edge =>
      exact .fromRightOutput (equivalent.reflectRightIsolation isolated) (refinement.step edge)
  | synchronizeLeftToRight connected send receive =>
      exact .synchronizeLeftToRight
        ((equivalent.leftToRight _ _).mp connected) send (refinement.step receive)
  | synchronizeRightToLeft connected send receive =>
      exact .synchronizeRightToLeft
        ((equivalent.rightToLeft _ _).mp connected) (refinement.step send) receive

def liftParallelRight
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    {peer : System LeftInternal LeftPort Message LeftObservation leftInterface}
    {concrete abstract : System RightInternal RightPort Message RightObservation rightInterface}
    {concreteWiring abstractWiring : Wiring LeftPort RightPort}
    (refinement : StrongRefinement concrete abstract)
    (equivalent : WiringEquivalent concreteWiring abstractWiring) :
    StrongRefinement (parallel peer concrete concreteWiring)
      (parallel peer abstract abstractWiring) where
  mapState := fun state => (state.1, refinement.mapState state.2)
  stateSurjective := by
    intro state
    obtain ⟨concreteState, mapped⟩ := refinement.stateSurjective state.2
    exact ⟨(state.1, concreteState), Prod.ext rfl mapped⟩
  initial := fun initial => ⟨initial.1, refinement.initial initial.2⟩
  observation := fun state => congrArg (fun observation => (peer.observe state.1, observation))
    (refinement.observation state.2)
  step := parallelStepCongruenceRight refinement equivalent

theorem preservesComposableRightUnderWiring
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    {peer : System LeftInternal LeftPort Message LeftObservation leftInterface}
    {concrete abstract : System RightInternal RightPort Message RightObservation rightInterface}
    {concreteWiring abstractWiring : Wiring LeftPort RightPort}
    (refinement : StrongRefinement concrete abstract)
    (equivalent : WiringEquivalent concreteWiring abstractWiring)
    (ready : Composable peer concrete concreteWiring) :
    Composable peer abstract abstractWiring where
  interfaces := {
    leftToRightPorts := by
      intro leftPort rightPort connected
      exact ready.interfaces.leftToRightPorts
        ((equivalent.leftToRight leftPort rightPort).mpr connected)
    rightToLeftPorts := by
      intro rightPort leftPort connected
      exact ready.interfaces.rightToLeftPorts
        ((equivalent.rightToLeft rightPort leftPort).mpr connected)
    leftToRightContract := by
      intro leftPort rightPort message connected guaranteed
      exact ready.interfaces.leftToRightContract
        ((equivalent.leftToRight leftPort rightPort).mpr connected) guaranteed
    rightToLeftContract := by
      intro rightPort leftPort message connected guaranteed
      exact ready.interfaces.rightToLeftContract
        ((equivalent.rightToLeft rightPort leftPort).mpr connected) guaranteed
  }
  leftReceptive := ready.leftReceptive
  rightReceptive := refinement.preservesInputReceptive ready.rightReceptive

/-- M11-001c's exact-action two-sided core: refining both components preserves
    composability and yields a refinement between the two products. Contract
    soundness here is the `Composable` result; richer label variance is kept in
    `OpenRefinement` and guarded against the executable product by shared
    correspondence controls. -/
theorem twoSidedCompositionCongruence
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    {concreteLeft abstractLeft :
      System LeftInternal LeftPort Message LeftObservation leftInterface}
    {concreteRight abstractRight :
      System RightInternal RightPort Message RightObservation rightInterface}
    {concreteWiring abstractWiring : Wiring LeftPort RightPort}
    (leftRefinement : StrongRefinement concreteLeft abstractLeft)
    (rightRefinement : StrongRefinement concreteRight abstractRight)
    (equivalent : WiringEquivalent concreteWiring abstractWiring)
    (ready : Composable concreteLeft concreteRight concreteWiring) :
    Composable abstractLeft abstractRight abstractWiring ∧
      Nonempty (StrongRefinement
        (parallel concreteLeft concreteRight concreteWiring)
        (parallel abstractLeft abstractRight abstractWiring)) := by
  have middleReady : Composable abstractLeft concreteRight abstractWiring :=
    leftRefinement.preservesComposableLeftUnderWiring equivalent ready
  have abstractReady : Composable abstractLeft abstractRight abstractWiring :=
    rightRefinement.preservesComposableRightUnderWiring
      rightRefinementWiringEquivalent middleReady
  exact ⟨abstractReady, ⟨StrongRefinement.compose
    (leftRefinement.liftParallel equivalent)
    (rightRefinement.liftParallelRight rightRefinementWiringEquivalent)⟩⟩
where
  rightRefinementWiringEquivalent : WiringEquivalent abstractWiring abstractWiring := {
    leftToRight := fun _ _ => Iff.rfl
    rightToLeft := fun _ _ => Iff.rfl
  }

/-- Exact finite reachability for open systems. -/
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
    {Internal Port Message Observation : Type}
    {concreteInterface abstractInterface : Interface Port Message}
    {concrete : System Internal Port Message Observation concreteInterface}
    {abstract : System Internal Port Message Observation abstractInterface}
    (refinement : StrongRefinement concrete abstract) {state : concrete.State} :
    Reachable concrete state → Reachable abstract (refinement.mapState state) := by
  intro reachable
  induction reachable with
  | initial starts => exact .initial (refinement.initial starts)
  | step previous edge inductionHypothesis =>
      exact .step inductionHypothesis (refinement.step edge)

/-- Safety invariants transport contravariantly along any supported strong
    refinement, including the two-sided product refinement above. -/
theorem transportInvariant
    {Internal Port Message Observation : Type}
    {concreteInterface abstractInterface : Interface Port Message}
    {concrete : System Internal Port Message Observation concreteInterface}
    {abstract : System Internal Port Message Observation abstractInterface}
    (refinement : StrongRefinement concrete abstract)
    (predicate : abstract.State → Prop) (holds : Invariant abstract predicate) :
    Invariant concrete (predicate ∘ refinement.mapState) := by
  intro state reachable
  exact holds (refinement.mapState state) (refinement.mapReachable reachable)

/-- Bounded M11-001 composition congruence. A well-formed concrete composition
    remains well formed after exact-action refinement of its left component,
    and the concrete product strongly refines the abstract product.

    This theorem deliberately makes no claim about weak hiding, label maps,
    capabilities, grades, fairness, divergence, or liveness. -/
theorem compositionCongruence
    {LeftInternal LeftPort RightInternal RightPort Message LeftObservation RightObservation : Type}
    {leftInterface : Interface LeftPort Message}
    {rightInterface : Interface RightPort Message}
    {concrete abstract : System LeftInternal LeftPort Message LeftObservation leftInterface}
    {peer : System RightInternal RightPort Message RightObservation rightInterface}
    {concreteWiring abstractWiring : Wiring LeftPort RightPort}
    (refinement : StrongRefinement concrete abstract)
    (equivalent : WiringEquivalent concreteWiring abstractWiring)
    (ready : Composable concrete peer concreteWiring) :
    Composable abstract peer abstractWiring ∧
      Nonempty (StrongRefinement (parallel concrete peer concreteWiring)
        (parallel abstract peer abstractWiring)) := by
  exact ⟨refinement.preservesComposableLeftUnderWiring equivalent ready,
    ⟨refinement.liftParallel equivalent⟩⟩

#print axioms preservesInputReceptive
#print axioms outputCanSynchronize
#print axioms rightOutputCanSynchronize
#print axioms parallelInputReceptive
#print axioms preservesComposableLeft
#print axioms WiringEquivalent.reflectRightIsolation
#print axioms preservesComposableLeftUnderWiring
#print axioms parallelStepCongruence
#print axioms compositionCongruence
#print axioms StrongRefinement.identity
#print axioms StrongRefinement.compose
#print axioms parallelStepCongruenceRight
#print axioms liftParallelRight
#print axioms preservesComposableRightUnderWiring
#print axioms twoSidedCompositionCongruence
#print axioms mapReachable
#print axioms transportInvariant

end StrongRefinement

namespace Examples

def senderInterface : Interface Unit Unit where
  accepts := fun _ => False
  emits := fun _ => True
  assumption := fun _ _ => False
  guarantee := fun _ _ => True
  assumption_is_input := by
    intro port message impossible
    exact impossible.elim
  guarantee_is_output := by
    intro port message guaranteed
    trivial
  directions_disjoint := by
    intro port accepted emitted
    exact accepted.elim

def receiverInterface : Interface Unit Unit where
  accepts := fun _ => True
  emits := fun _ => False
  assumption := fun _ _ => True
  guarantee := fun _ _ => False
  assumption_is_input := by
    intro port message assumed
    trivial
  guarantee_is_output := by
    intro port message impossible
    exact impossible.elim
  directions_disjoint := by
    intro port accepted emitted
    exact emitted.elim

inductive ConcreteSenderStep :
    Bool → OpenAction Unit Unit Unit → Bool → Prop where
  | send (before : Bool) : ConcreteSenderStep before (.output () ()) (!before)

inductive AbstractSenderStep :
    Unit → OpenAction Unit Unit Unit → Unit → Prop where
  | send : AbstractSenderStep () (.output () ()) ()

inductive ReceiverStep :
    Bool → OpenAction Unit Unit Unit → Bool → Prop where
  | receive (before : Bool) : ReceiverStep before (.input () ()) true

def concreteSender : System Unit Unit Unit Unit senderInterface where
  State := Bool
  init := fun state => state = false
  step := ConcreteSenderStep
  observe := fun _ => ()
  step_admitted := by
    intro before action after step
    cases step
    trivial

def abstractSender : System Unit Unit Unit Unit senderInterface where
  State := Unit
  init := fun _ => True
  step := AbstractSenderStep
  observe := fun _ => ()
  step_admitted := by
    intro before action after step
    cases step
    trivial

def receiver : System Unit Unit Unit Bool receiverInterface where
  State := Bool
  init := fun state => state = false
  step := ReceiverStep
  observe := id
  step_admitted := by
    intro before action after step
    cases step
    trivial

def senderRefinement : StrongRefinement concreteSender abstractSender where
  mapState := fun _ => ()
  stateSurjective := by
    intro state
    cases state
    exact ⟨false, rfl⟩
  initial := by
    intro state initial
    trivial
  observation := by
    intro state
    rfl
  step := by
    intro before action after step
    cases step
    exact .send

theorem senderReceptive : InputReceptive concreteSender := by
  intro state port message accepted
  exact accepted.elim

theorem receiverReceptive : InputReceptive receiver := by
  intro state port message accepted
  cases port
  cases message
  exact ⟨true, .receive state⟩

def connectedWiring : Wiring Unit Unit where
  leftToRight := fun _ _ => True
  rightToLeft := fun _ _ => False

theorem senderReceiverCompatible :
    InterfaceCompatible senderInterface receiverInterface connectedWiring where
  leftToRightPorts := by
    intro leftPort rightPort connected
    exact ⟨True.intro, True.intro⟩
  rightToLeftPorts := by
    intro rightPort leftPort impossible
    exact impossible.elim
  leftToRightContract := by
    intro leftPort rightPort message connected guaranteed
    trivial
  rightToLeftContract := by
    intro rightPort leftPort message impossible guaranteed
    exact impossible.elim

theorem senderReceiverComposable :
    Composable concreteSender receiver connectedWiring where
  interfaces := senderReceiverCompatible
  leftReceptive := senderReceptive
  rightReceptive := receiverReceptive

theorem connectedWiringEquivalent : WiringEquivalent connectedWiring connectedWiring where
  leftToRight := fun _ _ => Iff.rfl
  rightToLeft := fun _ _ => Iff.rfl

/-- The positive control is inhabited by a nonidentity state abstraction and a
    product with a real synchronized transition. -/
theorem positiveCompositionCongruence :
    Composable abstractSender receiver connectedWiring ∧
      Nonempty (StrongRefinement
        (parallel concreteSender receiver connectedWiring)
        (parallel abstractSender receiver connectedWiring)) := by
  exact StrongRefinement.compositionCongruence senderRefinement
    connectedWiringEquivalent senderReceiverComposable

/-- Two-sided theorem control. The right witness is explicit identity here;
    the theorem itself accepts independent nonidentity refinements on both
    components. -/
theorem positiveTwoSidedCompositionCongruence :
    Composable abstractSender receiver connectedWiring ∧
      Nonempty (StrongRefinement
        (parallel concreteSender receiver connectedWiring)
        (parallel abstractSender receiver connectedWiring)) := by
  exact StrongRefinement.twoSidedCompositionCongruence senderRefinement
    (StrongRefinement.identity receiver) connectedWiringEquivalent
    senderReceiverComposable

theorem positiveProductInvariantTransport
    (predicate : (Unit × Bool) → Prop)
    (holds : StrongRefinement.Invariant
      (parallel abstractSender receiver connectedWiring) predicate) :
    ∃ refinement : StrongRefinement
        (parallel concreteSender receiver connectedWiring)
        (parallel abstractSender receiver connectedWiring),
      StrongRefinement.Invariant
        (parallel concreteSender receiver connectedWiring)
        (predicate ∘ refinement.mapState) := by
  have lifted : StrongRefinement
      (parallel concreteSender receiver connectedWiring)
      (parallel abstractSender receiver connectedWiring) :=
    StrongRefinement.compose
      (senderRefinement.liftParallel connectedWiringEquivalent)
      ((StrongRefinement.identity receiver).liftParallelRight
        connectedWiringEquivalent)
  exact ⟨lifted, lifted.transportInvariant predicate holds⟩

theorem positiveConcreteSynchronization :
    ParallelStep concreteSender receiver connectedWiring (false, false)
      (.internal (.leftToRight () () ())) (true, true) := by
  exact .synchronizeLeftToRight True.intro (.send false) (.receive false)

def emptyWiring : Wiring Unit Unit where
  leftToRight := fun _ _ => False
  rightToLeft := fun _ _ => False

def brokenAbstractWiring : Wiring Unit Unit where
  leftToRight := fun _ _ => True
  rightToLeft := fun _ _ => False

theorem emptyWiringRightIsolated : ¬ emptyWiring.rightConnected () := by
  intro connected
  cases connected with
  | inl edge =>
      obtain ⟨leftPort, impossible⟩ := edge
      exact impossible.elim
  | inr edge =>
      obtain ⟨leftPort, impossible⟩ := edge
      exact impossible.elim

theorem brokenWiringRightConnected : brokenAbstractWiring.rightConnected () := by
  left
  exact ⟨(), True.intro⟩

theorem peerOnlyWithEmptyWiring :
    ParallelStep abstractSender receiver emptyWiring ((), false)
      (.input (.inr ()) ()) ((), true) := by
  exact .fromRightInput emptyWiringRightIsolated (.receive false)

/-- Exact-semantics negative control: adding an abstract connection outside
    the concrete connection image blocks the same peer-only input step. This
    is the failure excluded by `WiringEquivalent.reflectRightIsolation`. -/
theorem brokenWiringBlocksPeerOnly :
    ¬ ParallelStep abstractSender receiver brokenAbstractWiring ((), false)
      (.input (.inr ()) ()) ((), true) := by
  intro transition
  cases transition with
  | fromRightInput isolated step => exact isolated brokenWiringRightConnected

theorem brokenWiringNotEquivalent :
    ¬ WiringEquivalent emptyWiring brokenAbstractWiring := by
  intro equivalent
  exact equivalent.reflectRightIsolation emptyWiringRightIsolated
    brokenWiringRightConnected

#print axioms positiveCompositionCongruence
#print axioms positiveTwoSidedCompositionCongruence
#print axioms positiveProductInvariantTransport
#print axioms positiveConcreteSynchronization
#print axioms peerOnlyWithEmptyWiring
#print axioms brokenWiringBlocksPeerOnly
#print axioms brokenWiringNotEquivalent

end Examples

end NMLT.Behavior.OpenComposition
