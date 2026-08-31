import NMLT.Artifact.BehaviorCore
import NMLT.Behavior.ResourceWorld

namespace NMLT.Artifact.SemanticClosure

open NMLT.Behavior.ResourceBehavior
open NMLT.Behavior.ResourceWorld
open NMLT.Artifact.BehaviorCore

inductive Value where
  | bool (value : Bool)
  | unit
  | enumeration (qualifiedConstructor : String)
deriving Repr, BEq, DecidableEq

abbrev Valuation := List (String × Value)

private def lookupValue (state : Valuation) (field : String) : Option Value :=
  (state.find? fun entry => entry.1 == field).map Prod.snd

partial def evaluateTerm (term : Term) (state : Valuation) : Option Value :=
  match term with
  | .bool value => some (.bool value)
  | .unit => some .unit
  | .enumeration typeName constructor =>
      some (.enumeration s!"{typeName}.{constructor}")
  | .read _ field => lookupValue state field
  | .not value => do
      let .bool result ← evaluateTerm value state | none
      pure (.bool (!result))
  | .equal left right => do
      let leftValue ← evaluateTerm left state
      let rightValue ← evaluateTerm right state
      pure (.bool (leftValue == rightValue))

def typeDomain (program : Program) (typeName : String) : List Value :=
  if typeName == "Bool" then
    [.bool false, .bool true]
  else if typeName == "Unit" then
    [.unit]
  else
    match program.enums.find? fun entry => entry.1 == typeName with
    | some (_, constructors) =>
        constructors.map fun constructor =>
          .enumeration s!"{typeName}.{constructor}"
    | none => []

def enumerateState (program : Program) : List StateDecl → List Valuation
  | [] => [[]]
  | field :: rest =>
      (typeDomain program field.typeName).flatMap fun value =>
        (enumerateState program rest).map fun suffix => (field.name, value) :: suffix

def stateSpace (program : Program) (system : System) : List Valuation :=
  enumerateState program system.state

private def initialValuation (system : System) : Option Valuation :=
  system.state.mapM fun field => do
    pure (field.name, ← evaluateTerm field.initial [])

private def actionNamed? (system : System) (name : String) : Option Action :=
  system.actions.find? fun action => action.name == name

private def portNamed? (system : System) (name : String) : Option Port :=
  system.ports.find? fun port => port.name == name

private def enabled (action : Action) (before : Valuation) : Bool :=
  action.guards.all fun guard => evaluateTerm guard before == some (Value.bool true)

private def execute (action : Action) (before : Valuation) : Option Valuation :=
  before.mapM fun (field, oldValue) =>
    match action.updates.find? (fun update => update.1 == field) with
    | none => pure (field, oldValue)
    | some (_, term) => do pure (field, ← evaluateTerm term before)

private def observe (system : System) (state : Valuation) : List (Option Value) :=
  system.observe.map (lookupValue state)

private def unique (names : List String) : List String :=
  names.foldl (fun result name => if result.contains name then result else result ++ [name]) []

def capabilityNames (program : Program) : List String :=
  unique (program.systems.flatMap fun system => system.capabilities.map Prod.fst)

def gradeAtoms (program : Program) : List String :=
  unique (program.systems.flatMap fun system =>
    system.actions.flatMap fun action => action.resources.grade.map Prod.fst)

structure Application where
  program : Program
  refinement : Refinement
  concrete : System
  abstract : System
  peer : System
  concreteComposition : Composition
  abstractComposition : Composition

def actionNames (application : Application) : List String :=
  unique (application.concrete.actions.map Action.name ++
    application.abstract.actions.map Action.name)

def peerActionNames (application : Application) : List String :=
  unique (application.peer.actions.map Action.name)

def nameAt? (names : List String) (index : Fin (names.length + 1)) : Option String :=
  names[index.val]?

def stateAt? (program : Program) (system : System)
    (index : Fin ((stateSpace program system).length + 1)) : Option Valuation :=
  (stateSpace program system)[index.val]?

private def indexOfValuation (states : List Valuation) (state : Valuation) :
    Fin (states.length + 1) :=
  let index := states.findIdx fun candidate => candidate == state
  ⟨index, Nat.lt_succ_iff.mpr List.findIdx_le_length⟩

private def profileFor
    (program : Program) (profile : Profile) :
    ResourceProfile
      (Fin ((capabilityNames program).length + 1))
      (Fin (program.facts.length + 1))
      (Fin ((gradeAtoms program).length + 1)) where
  requires := fun capability =>
    match nameAt? (capabilityNames program) capability with
    | some name => profile.requires.contains name = true
    | none => False
  consumes := fun capability =>
    match nameAt? (capabilityNames program) capability with
    | some name => profile.consumes.contains name = true
    | none => False
  transfers := fun capability =>
    match nameAt? (capabilityNames program) capability with
    | some name => profile.transfers.contains name = true
    | none => False
  receives := fun capability =>
    match nameAt? (capabilityNames program) capability with
    | some name => profile.receives.contains name = true
    | none => False
  grade := ⟨fun atom =>
    match nameAt? (gradeAtoms program) atom with
    | some name => (profile.grade.lookup name).getD 0
    | none => 0⟩
  relies := fun fact =>
    match nameAt? program.facts fact with
    | some name => profile.relies.contains name = true
    | none => False
  guarantees := fun fact =>
    match nameAt? program.facts fact with
    | some name => profile.guarantees.contains name = true
    | none => False

private def emptyProfile (program : Program) :=
  (ResourceProfile.empty :
    ResourceProfile
      (Fin ((capabilityNames program).length + 1))
      (Fin (program.facts.length + 1))
      (Fin ((gradeAtoms program).length + 1)))

instance profileForRequiresDecidable
    (program : Program) (profile : Profile)
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable ((profileFor program profile).requires capability) := by
  simp only [profileFor]
  split <;> infer_instance

instance profileForConsumesDecidable
    (program : Program) (profile : Profile)
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable ((profileFor program profile).consumes capability) := by
  simp only [profileFor]
  split <;> infer_instance

instance profileForTransfersDecidable
    (program : Program) (profile : Profile)
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable ((profileFor program profile).transfers capability) := by
  simp only [profileFor]
  split <;> infer_instance

instance profileForReceivesDecidable
    (program : Program) (profile : Profile)
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable ((profileFor program profile).receives capability) := by
  simp only [profileFor]
  split <;> infer_instance

instance profileForReliesDecidable
    (program : Program) (profile : Profile)
    (fact : Fin (program.facts.length + 1)) :
    Decidable ((profileFor program profile).relies fact) := by
  simp only [profileFor]
  split <;> infer_instance

instance profileForGuaranteesDecidable
    (program : Program) (profile : Profile)
    (fact : Fin (program.facts.length + 1)) :
    Decidable ((profileFor program profile).guarantees fact) := by
  simp only [profileFor]
  split <;> infer_instance

instance emptyProfileRequiresDecidable
    (program : Program)
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable ((emptyProfile program).requires capability) := by
  exact isFalse id

instance emptyProfileConsumesDecidable
    (program : Program)
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable ((emptyProfile program).consumes capability) := by
  exact isFalse id

instance emptyProfileTransfersDecidable
    (program : Program)
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable ((emptyProfile program).transfers capability) := by
  exact isFalse id

instance emptyProfileReceivesDecidable
    (program : Program)
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable ((emptyProfile program).receives capability) := by
  exact isFalse id

instance emptyProfileReliesDecidable
    (program : Program) (fact : Fin (program.facts.length + 1)) :
    Decidable ((emptyProfile program).relies fact) := by
  exact isFalse id

instance emptyProfileGuaranteesDecidable
    (program : Program) (fact : Fin (program.facts.length + 1)) :
    Decidable ((emptyProfile program).guarantees fact) := by
  exact isFalse id

private def actionFor?
    (system : System) (actionNames : List String)
    (action : Fin (actionNames.length + 1)) : Option Action := do
  let name ← nameAt? actionNames action
  actionNamed? system name

private def actionPayload
    (system : System) (actionNames : List String)
    (action : Fin (actionNames.length + 1)) : String :=
  match nameAt? actionNames action with
  | some name => (portNamed? system name).map Port.payload |>.getD "Unit"
  | none => "Unit"

def toBehavior
    (program : Program) (system : System) (actionNames extraHidden : List String) :
    Behavior
      (Fin (actionNames.length + 1))
      (Fin ((capabilityNames program).length + 1))
      (Fin (program.facts.length + 1))
      (Fin ((gradeAtoms program).length + 1))
      (List (Option Value)) where
  State := Fin ((stateSpace program system).length + 1)
  init := fun state =>
    match stateAt? program system state, initialValuation system with
    | some actual, some initial => actual = initial
    | _, _ => False
  step := fun before action after =>
    match stateAt? program system before,
        actionFor? system actionNames action,
        stateAt? program system after with
    | some beforeState, some selectedAction, some afterState =>
        enabled selectedAction beforeState = true ∧
          execute selectedAction beforeState = some afterState
    | _, _, _ => False
  observe := fun state =>
    match stateAt? program system state with
    | some value => SemanticClosure.observe system value
    | none => []
  hidden := fun action =>
    match nameAt? actionNames action with
    | some name =>
        (extraHidden.contains name ||
          ((actionNamed? system name).map Action.hidden).getD false) = true
    | none => False
  direction := fun action =>
    (actionFor? system actionNames action).map Action.direction |>.getD .internal
  payload := actionPayload system actionNames
  owns := fun capability =>
    match nameAt? (capabilityNames program) capability with
    | some name => (system.capabilities.map Prod.fst).contains name = true
    | none => False
  resources := fun action =>
    match actionFor? system actionNames action with
    | some selected => profileFor program selected.resources
    | none => emptyProfile program

instance behaviorRequiresDecidable
    (program : Program) (system : System) (actions extraHidden : List String)
    (action : Fin (actions.length + 1))
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable
      (((toBehavior program system actions extraHidden).resources action).requires capability) := by
  unfold toBehavior
  dsimp only
  split <;> infer_instance

instance behaviorConsumesDecidable
    (program : Program) (system : System) (actions extraHidden : List String)
    (action : Fin (actions.length + 1))
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable
      (((toBehavior program system actions extraHidden).resources action).consumes capability) := by
  unfold toBehavior
  dsimp only
  split <;> infer_instance

instance behaviorTransfersDecidable
    (program : Program) (system : System) (actions extraHidden : List String)
    (action : Fin (actions.length + 1))
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable
      (((toBehavior program system actions extraHidden).resources action).transfers capability) := by
  unfold toBehavior
  dsimp only
  split <;> infer_instance

instance behaviorReceivesDecidable
    (program : Program) (system : System) (actions extraHidden : List String)
    (action : Fin (actions.length + 1))
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable
      (((toBehavior program system actions extraHidden).resources action).receives capability) := by
  unfold toBehavior
  dsimp only
  split <;> infer_instance

instance behaviorReliesDecidable
    (program : Program) (system : System) (actions extraHidden : List String)
    (action : Fin (actions.length + 1))
    (fact : Fin (program.facts.length + 1)) :
    Decidable
      (((toBehavior program system actions extraHidden).resources action).relies fact) := by
  unfold toBehavior
  dsimp only
  split <;> infer_instance

instance behaviorGuaranteesDecidable
    (program : Program) (system : System) (actions extraHidden : List String)
    (action : Fin (actions.length + 1))
    (fact : Fin (program.facts.length + 1)) :
    Decidable
      (((toBehavior program system actions extraHidden).resources action).guarantees fact) := by
  unfold toBehavior
  dsimp only
  split <;> infer_instance

def decideInitial
    (actual initial : Option Valuation) :
    Decidable (
      match actual, initial with
      | some actualState, some initialState => actualState = initialState
      | _, _ => False) :=
  match actual, initial with
  | some actualState, some initialState => decEq actualState initialState
  | none, none => isFalse id
  | none, some _ => isFalse id
  | some _, none => isFalse id

def decideStep
    (before : Option Valuation) (action : Option Action) (after : Option Valuation) :
    Decidable (
      match before, action, after with
      | some beforeState, some selectedAction, some afterState =>
          enabled selectedAction beforeState = true ∧
            execute selectedAction beforeState = some afterState
      | _, _, _ => False) :=
  match before with
  | none => isFalse id
  | some beforeState =>
      match action with
      | none => isFalse id
      | some selectedAction =>
          match after with
          | none => isFalse id
          | some afterState =>
              @instDecidableAnd _ _
                (Bool.decEq (enabled selectedAction beforeState) true)
                (decEq (execute selectedAction beforeState) (some afterState))

def decideHidden (system : System) (actions extraHidden : List String)
    (action : Fin (actions.length + 1)) :
    Decidable (
      match nameAt? actions action with
      | some actionName =>
          (extraHidden.contains actionName ||
            ((actionNamed? system actionName).map Action.hidden).getD false) = true
      | none => False) :=
  match nameAt? actions action with
  | some actionName =>
      Bool.decEq
        (extraHidden.contains actionName ||
          ((actionNamed? system actionName).map Action.hidden).getD false) true
  | none => isFalse id

def decideOwns (capability : Option String) (owned : List String) :
    Decidable (
      match capability with
      | some name => owned.contains name = true
      | none => False) :=
  match capability with
  | some name => Bool.decEq (owned.contains name) true
  | none => isFalse id

instance behaviorInitDecidable
    (program : Program) (system : System) (actions extraHidden : List String)
    (state : (toBehavior program system actions extraHidden).State) :
    Decidable ((toBehavior program system actions extraHidden).init state) :=
  decideInitial (stateAt? program system state) (initialValuation system)

instance behaviorStepDecidable
    (program : Program) (system : System) (actions extraHidden : List String)
    (before : (toBehavior program system actions extraHidden).State)
    (action : Fin (actions.length + 1))
    (after : (toBehavior program system actions extraHidden).State) :
    Decidable ((toBehavior program system actions extraHidden).step before action after) :=
  decideStep
    (stateAt? program system before)
    (actionFor? system actions action)
    (stateAt? program system after)

instance behaviorHiddenDecidable
    (program : Program) (system : System) (actions extraHidden : List String)
    (action : Fin (actions.length + 1)) :
    Decidable ((toBehavior program system actions extraHidden).hidden action) :=
  decideHidden system actions extraHidden action

instance behaviorOwnsDecidable
    (program : Program) (system : System) (actions extraHidden : List String)
    (capability : Fin ((capabilityNames program).length + 1)) :
    Decidable ((toBehavior program system actions extraHidden).owns capability) :=
  decideOwns (nameAt? (capabilityNames program) capability)
    (system.capabilities.map Prod.fst)

instance directionComplementaryDecidable (left right : Direction) :
    Decidable (Direction.Complementary left right) := by
  cases left <;> cases right <;>
    first | exact isTrue trivial | exact isFalse id

private def mappedValuation
    (application : Application) (concreteState : Valuation) : Valuation :=
  application.abstract.state.map fun abstractField =>
    let concreteField :=
      application.refinement.stateMap.find?
        (fun entry => entry.2 == abstractField.name) |>.map Prod.fst
    let value := concreteField.bind (lookupValue concreteState) |>.getD .unit
    (abstractField.name, value)

def mapState (application : Application) :
    (toBehavior application.program application.concrete
      (actionNames application) application.refinement.hiddenActions).State →
    (toBehavior application.program application.abstract
      (actionNames application) []).State :=
  fun concreteIndex =>
    match stateAt? application.program application.concrete concreteIndex with
    | some concreteState =>
        indexOfValuation
          (stateSpace application.program application.abstract)
          (mappedValuation application concreteState)
    | none => Fin.last _

private def connectionPairs (composition : Composition) : List (String × String) :=
  composition.connections.map fun connection =>
    (connection.leftAction, connection.rightAction)

def Application.concreteConnection (application : Application) :
    Fin ((actionNames application).length + 1) →
    Fin ((peerActionNames application).length + 1) → Prop :=
  fun left right =>
    match nameAt? (actionNames application) left,
        nameAt? (peerActionNames application) right with
    | some leftName, some rightName =>
        (connectionPairs application.concreteComposition).contains (leftName, rightName) = true
    | _, _ => False

def Application.abstractConnection (application : Application) :
    Fin ((actionNames application).length + 1) →
    Fin ((peerActionNames application).length + 1) → Prop :=
  fun left right =>
    match nameAt? (actionNames application) left,
        nameAt? (peerActionNames application) right with
    | some leftName, some rightName =>
        (connectionPairs application.abstractComposition).contains (leftName, rightName) = true
    | _, _ => False

instance concreteConnectionDecidable
    (application : Application)
    (left : Fin ((actionNames application).length + 1))
    (right : Fin ((peerActionNames application).length + 1)) :
    Decidable (application.concreteConnection left right) := by
  unfold Application.concreteConnection
  split <;> infer_instance

instance abstractConnectionDecidable
    (application : Application)
    (left : Fin ((actionNames application).length + 1))
    (right : Fin ((peerActionNames application).length + 1)) :
    Decidable (application.abstractConnection left right) := by
  unfold Application.abstractConnection
  split <;> infer_instance

private def systemNamed? (program : Program) (name : String) : Option System :=
  program.systems.find? fun system => system.name == name

private def matchingApplications
    (program : Program) (refinement : Refinement) : List Application :=
  let concreteCompositions :=
    program.compositions.filter fun composition => composition.left == refinement.concrete
  let abstractCompositions :=
    program.compositions.filter fun composition => composition.left == refinement.abstract
  concreteCompositions.flatMap fun concreteComposition =>
    abstractCompositions.filterMap fun abstractComposition =>
      if concreteComposition.right != abstractComposition.right then none
      else do
        let concrete ← systemNamed? program refinement.concrete
        let abstract ← systemNamed? program refinement.abstract
        let peer ← systemNamed? program concreteComposition.right
        pure {
          program
          refinement
          concrete
          abstract
          peer
          concreteComposition
          abstractComposition
        }

def applications (program : Program) : List Application :=
  program.refinements.flatMap (matchingApplications program)

structure ApplicationSummary where
  concrete : String
  abstract : String
  peer : String
  concreteComposition : String
  abstractComposition : String
  concreteStates : Nat
  abstractStates : Nat
  peerStates : Nat
  declaredCapabilities : Nat
deriving Repr

def applicationSummary (application : Application) : ApplicationSummary := {
  concrete := application.concrete.name
  abstract := application.abstract.name
  peer := application.peer.name
  concreteComposition := application.concreteComposition.name
  abstractComposition := application.abstractComposition.name
  concreteStates := (stateSpace application.program application.concrete).length
  abstractStates := (stateSpace application.program application.abstract).length
  peerStates := (stateSpace application.program application.peer).length
  declaredCapabilities :=
    application.concrete.capabilities.length + application.peer.capabilities.length
}

abbrev CapabilityIndex (application : Application) :=
  Fin ((capabilityNames application.program).length + 1)

abbrev FactIndex (application : Application) :=
  Fin (application.program.facts.length + 1)

abbrev GradeIndex (application : Application) :=
  Fin ((gradeAtoms application.program).length + 1)

abbrev ActionIndex (application : Application) :=
  Fin ((actionNames application).length + 1)

abbrev PeerActionIndex (application : Application) :=
  Fin ((peerActionNames application).length + 1)

abbrev ConcreteStateIndex (application : Application) :=
  Fin ((stateSpace application.program application.concrete).length + 1)

abbrev AbstractStateIndex (application : Application) :=
  Fin ((stateSpace application.program application.abstract).length + 1)

inductive BinaryOwner where
  | component
  | peer
deriving Repr, BEq, DecidableEq

/-- Initial authority for the decoded concrete product, derived from declarations. -/
def initialAuthorityWorld (application : Application) :
    AuthorityWorld (CapabilityIndex application) BinaryOwner where
  owner := fun capability =>
    match nameAt? (capabilityNames application.program) capability with
    | some name =>
        if (application.concrete.capabilities.map Prod.fst).contains name then
          some .component
        else if (application.peer.capabilities.map Prod.fst).contains name then
          some .peer
        else
          none
    | none => none

abbrev concreteBehavior (application : Application) :=
  toBehavior application.program application.concrete
    (actionNames application) application.refinement.hiddenActions

abbrev abstractBehavior (application : Application) :=
  toBehavior application.program application.abstract
    (actionNames application) application.refinement.hiddenActions

abbrev peerBehavior (application : Application) :=
  toBehavior application.program application.peer
    (peerActionNames application) []

abbrev DynamicConcreteState (application : Application) :=
  NMLT.Behavior.ResourceWorld.ProductState
    (concreteBehavior application) (peerBehavior application) BinaryOwner

abbrev DynamicConcreteStep (application : Application) :=
  NMLT.Behavior.ResourceWorld.ProductStep
    (concreteBehavior application) (peerBehavior application)
    BinaryOwner.component BinaryOwner.peer application.concreteConnection

structure ProfileConditions
    (application : Application)
    (concrete abstract :
      ResourceProfile (CapabilityIndex application)
        (FactIndex application) (GradeIndex application)) : Prop where
  requires : ∀ capability, concrete.requires capability → abstract.requires capability
  consumes : ∀ capability, concrete.consumes capability ↔ abstract.consumes capability
  transfers : ∀ capability, concrete.transfers capability ↔ abstract.transfers capability
  receives : ∀ capability, concrete.receives capability ↔ abstract.receives capability
  grade : ∀ atom, concrete.grade.use atom ≤ abstract.grade.use atom
  relies : ∀ fact, concrete.relies fact → abstract.relies fact
  guarantees : ∀ fact, abstract.guarantees fact → concrete.guarantees fact

instance profileConditionsDecidable
    (application : Application)
    (concrete abstract :
      ResourceProfile (CapabilityIndex application)
        (FactIndex application) (GradeIndex application))
    [DecidablePred concrete.requires] [DecidablePred abstract.requires]
    [DecidablePred concrete.consumes] [DecidablePred abstract.consumes]
    [DecidablePred concrete.transfers] [DecidablePred abstract.transfers]
    [DecidablePred concrete.receives] [DecidablePred abstract.receives]
    [DecidablePred concrete.relies] [DecidablePred abstract.relies]
    [DecidablePred concrete.guarantees] [DecidablePred abstract.guarantees] :
    Decidable (ProfileConditions application concrete abstract) := by
  if requires : ∀ capability, concrete.requires capability → abstract.requires capability then
    if consumes : ∀ capability, concrete.consumes capability ↔ abstract.consumes capability then
      if transfers : ∀ capability, concrete.transfers capability ↔ abstract.transfers capability then
        if receives : ∀ capability, concrete.receives capability ↔ abstract.receives capability then
          if grade : ∀ atom, concrete.grade.use atom ≤ abstract.grade.use atom then
            if relies : ∀ fact, concrete.relies fact → abstract.relies fact then
              if guarantees : ∀ fact, abstract.guarantees fact → concrete.guarantees fact then
                exact isTrue { requires, consumes, transfers, receives, grade, relies, guarantees }
              else exact isFalse fun conditions => guarantees conditions.guarantees
            else exact isFalse fun conditions => relies conditions.relies
          else exact isFalse fun conditions => grade conditions.grade
        else exact isFalse fun conditions => receives conditions.receives
      else exact isFalse fun conditions => transfers conditions.transfers
    else exact isFalse fun conditions => consumes conditions.consumes
  else exact isFalse fun conditions => requires conditions.requires

def ProfileConditions.toResourceRefines
    {application : Application}
    {concrete abstract :
      ResourceProfile (CapabilityIndex application)
        (FactIndex application) (GradeIndex application)}
    (conditions : ProfileConditions application concrete abstract) :
    ResourceRefines concrete abstract where
  requires := conditions.requires
  consumes := conditions.consumes
  transfers := conditions.transfers
  receives := conditions.receives
  grade := conditions.grade
  relies := conditions.relies
  guarantees := conditions.guarantees

structure SynchronizationConditions
    (application : Application)
    (sender receiver :
      ResourceProfile (CapabilityIndex application)
        (FactIndex application) (GradeIndex application)) : Prop where
  transfer : ∀ capability, sender.transfers capability ↔ receiver.receives capability
  noReverseTransfer : ∀ capability,
    receiver.transfers capability ↔ sender.receives capability
  senderRely : ∀ fact, sender.relies fact → receiver.guarantees fact
  receiverRely : ∀ fact, receiver.relies fact → sender.guarantees fact

instance synchronizationConditionsDecidable
    (application : Application)
    (sender receiver :
      ResourceProfile (CapabilityIndex application)
        (FactIndex application) (GradeIndex application))
    [DecidablePred sender.transfers] [DecidablePred receiver.transfers]
    [DecidablePred sender.receives] [DecidablePred receiver.receives]
    [DecidablePred sender.relies] [DecidablePred receiver.relies]
    [DecidablePred sender.guarantees] [DecidablePred receiver.guarantees] :
    Decidable (SynchronizationConditions application sender receiver) := by
  if transfer : ∀ capability, sender.transfers capability ↔ receiver.receives capability then
    if noReverseTransfer : ∀ capability,
        receiver.transfers capability ↔ sender.receives capability then
      if senderRely : ∀ fact, sender.relies fact → receiver.guarantees fact then
        if receiverRely : ∀ fact, receiver.relies fact → sender.guarantees fact then
          exact isTrue { transfer, noReverseTransfer, senderRely, receiverRely }
        else exact isFalse fun conditions => receiverRely conditions.receiverRely
      else exact isFalse fun conditions => senderRely conditions.senderRely
    else exact isFalse fun conditions => noReverseTransfer conditions.noReverseTransfer
  else exact isFalse fun conditions => transfer conditions.transfer

def SynchronizationConditions.toCompatible
    {application : Application}
    {sender receiver :
      ResourceProfile (CapabilityIndex application)
        (FactIndex application) (GradeIndex application)}
    (conditions : SynchronizationConditions application sender receiver) :
    SynchronizationCompatible sender receiver where
  transfer := conditions.transfer
  noReverseTransfer := conditions.noReverseTransfer
  senderRely := conditions.senderRely
  receiverRely := conditions.receiverRely

structure RefinementConditions (application : Application) : Prop where
  init : ∀ state : ConcreteStateIndex application,
    (concreteBehavior application).init state →
    (abstractBehavior application).init (mapState application state)
  observeState : ∀ state : ConcreteStateIndex application,
    (concreteBehavior application).observe state =
      (abstractBehavior application).observe (mapState application state)
  hiddenPreserved : ∀ action : ActionIndex application,
    (concreteBehavior application).hidden action ↔
      (abstractBehavior application).hidden action
  hiddenStep : ∀ (before : ConcreteStateIndex application)
      (action : ActionIndex application) (after : ConcreteStateIndex application),
    (concreteBehavior application).step before action after →
    (concreteBehavior application).hidden action →
    mapState application before = mapState application after
  visibleStep : ∀ (before : ConcreteStateIndex application)
      (action : ActionIndex application) (after : ConcreteStateIndex application),
    (concreteBehavior application).step before action after →
    ¬ (concreteBehavior application).hidden action →
    (abstractBehavior application).step
      (mapState application before) action (mapState application after)
  resources : ∀ action : ActionIndex application,
    ProfileConditions application
      ((concreteBehavior application).resources action)
      ((abstractBehavior application).resources action)
  hiddenResources : ∀ action : ActionIndex application,
    (concreteBehavior application).hidden action →
    ProfileConditions application
      ((concreteBehavior application).resources action)
      (emptyProfile application.program)

instance concreteInitDecidable
    (application : Application) (state : ConcreteStateIndex application) :
    Decidable ((concreteBehavior application).init state) := by
  unfold concreteBehavior
  infer_instance

instance abstractInitDecidable
    (application : Application) (state : AbstractStateIndex application) :
    Decidable ((abstractBehavior application).init state) := by
  unfold abstractBehavior
  infer_instance

instance concreteHiddenDecidable
    (application : Application) (action : ActionIndex application) :
    Decidable ((concreteBehavior application).hidden action) := by
  unfold concreteBehavior
  infer_instance

instance abstractHiddenDecidable
    (application : Application) (action : ActionIndex application) :
    Decidable ((abstractBehavior application).hidden action) := by
  unfold abstractBehavior
  infer_instance

instance concreteStepDecidable
    (application : Application) (before : ConcreteStateIndex application)
    (action : ActionIndex application) (after : ConcreteStateIndex application) :
    Decidable ((concreteBehavior application).step before action after) := by
  unfold concreteBehavior
  infer_instance

instance abstractStepDecidable
    (application : Application) (before : AbstractStateIndex application)
    (action : ActionIndex application) (after : AbstractStateIndex application) :
    Decidable ((abstractBehavior application).step before action after) := by
  unfold abstractBehavior
  infer_instance

instance refinementConditionsDecidable (application : Application) :
    Decidable (RefinementConditions application) := by
  letI (action : ActionIndex application) :
      Decidable (ProfileConditions application
        ((concreteBehavior application).resources action)
        ((abstractBehavior application).resources action)) := by
    unfold concreteBehavior abstractBehavior
    infer_instance
  letI (action : ActionIndex application) :
      Decidable ((concreteBehavior application).hidden action →
        ProfileConditions application
          ((concreteBehavior application).resources action)
          (emptyProfile application.program)) := by
    if hidden : (concreteBehavior application).hidden action then
      if compatible : ProfileConditions application
          ((concreteBehavior application).resources action)
          (emptyProfile application.program) then
        exact isTrue fun _ => compatible
      else exact isFalse fun condition => compatible (condition hidden)
    else exact isTrue fun actionHidden => False.elim (hidden actionHidden)
  letI (state : ConcreteStateIndex application) :
      Decidable (
        (concreteBehavior application).init state →
        (abstractBehavior application).init (mapState application state)) := by
    if concreteInitial : (concreteBehavior application).init state then
      if abstractInitial :
          (abstractBehavior application).init (mapState application state) then
        exact isTrue fun _ => abstractInitial
      else exact isFalse fun condition => abstractInitial (condition concreteInitial)
    else exact isTrue fun initial => False.elim (concreteInitial initial)
  letI (state : ConcreteStateIndex application) :
      Decidable (
        (concreteBehavior application).observe state =
          (abstractBehavior application).observe (mapState application state)) :=
    decEq _ _
  letI (action : ActionIndex application) :
      Decidable (
        (concreteBehavior application).hidden action ↔
        (abstractBehavior application).hidden action) := by
    if concreteHidden : (concreteBehavior application).hidden action then
      if abstractHidden : (abstractBehavior application).hidden action then
        exact isTrue ⟨fun _ => abstractHidden, fun _ => concreteHidden⟩
      else exact isFalse fun equivalence =>
        abstractHidden (equivalence.mp concreteHidden)
    else
      if abstractHidden : (abstractBehavior application).hidden action then
        exact isFalse fun equivalence =>
          concreteHidden (equivalence.mpr abstractHidden)
      else exact isTrue ⟨
        fun hidden => False.elim (concreteHidden hidden),
        fun hidden => False.elim (abstractHidden hidden)
      ⟩
  letI : Decidable (∀ state : ConcreteStateIndex application,
      (concreteBehavior application).init state →
      (abstractBehavior application).init (mapState application state)) :=
    Nat.decidableForallFin _
  letI : Decidable (∀ state : ConcreteStateIndex application,
      (concreteBehavior application).observe state =
        (abstractBehavior application).observe (mapState application state)) :=
    Nat.decidableForallFin _
  letI : Decidable (∀ action : ActionIndex application,
      (concreteBehavior application).hidden action ↔
        (abstractBehavior application).hidden action) :=
    Nat.decidableForallFin _
  letI (before after : ConcreteStateIndex application) :
      Decidable (mapState application before = mapState application after) :=
    instDecidableEqFin _ _ _
  letI (before : ConcreteStateIndex application)
      (action : ActionIndex application) (after : ConcreteStateIndex application) :
      Decidable (
        (concreteBehavior application).step before action after →
        (concreteBehavior application).hidden action →
        mapState application before = mapState application after) :=
    by
      if stepped : (concreteBehavior application).step before action after then
        if hidden : (concreteBehavior application).hidden action then
          if equal : mapState application before = mapState application after then
            exact isTrue fun _ _ => equal
          else exact isFalse fun condition => equal (condition stepped hidden)
        else exact isTrue fun _ actionHidden => False.elim (hidden actionHidden)
      else exact isTrue fun step => False.elim (stepped step)
  letI (before : ConcreteStateIndex application)
      (action : ActionIndex application) :
      Decidable (∀ after : ConcreteStateIndex application,
        (concreteBehavior application).step before action after →
        (concreteBehavior application).hidden action →
        mapState application before = mapState application after) :=
    Nat.decidableForallFin _
  letI (before : ConcreteStateIndex application) :
      Decidable (∀ action : ActionIndex application,
        ∀ after : ConcreteStateIndex application,
        (concreteBehavior application).step before action after →
        (concreteBehavior application).hidden action →
        mapState application before = mapState application after) :=
    Nat.decidableForallFin _
  letI : Decidable (∀ before : ConcreteStateIndex application,
      ∀ action : ActionIndex application, ∀ after : ConcreteStateIndex application,
      (concreteBehavior application).step before action after →
      (concreteBehavior application).hidden action →
      mapState application before = mapState application after) :=
    Nat.decidableForallFin _
  letI (before : ConcreteStateIndex application)
      (action : ActionIndex application) (after : ConcreteStateIndex application) :
      Decidable (
        (concreteBehavior application).step before action after →
        ¬ (concreteBehavior application).hidden action →
        (abstractBehavior application).step
          (mapState application before) action (mapState application after)) := by
    if stepped : (concreteBehavior application).step before action after then
      if hidden : (concreteBehavior application).hidden action then
        exact isTrue fun _ notHidden => False.elim (notHidden hidden)
      else
        if abstractStepped : (abstractBehavior application).step
            (mapState application before) action (mapState application after) then
          exact isTrue fun _ _ => abstractStepped
        else exact isFalse fun condition =>
          abstractStepped (condition stepped hidden)
    else exact isTrue fun step => False.elim (stepped step)
  letI (before : ConcreteStateIndex application)
      (action : ActionIndex application) :
      Decidable (∀ after : ConcreteStateIndex application,
        (concreteBehavior application).step before action after →
        ¬ (concreteBehavior application).hidden action →
        (abstractBehavior application).step
          (mapState application before) action (mapState application after)) :=
    Nat.decidableForallFin _
  letI (before : ConcreteStateIndex application) :
      Decidable (∀ action : ActionIndex application,
        ∀ after : ConcreteStateIndex application,
        (concreteBehavior application).step before action after →
        ¬ (concreteBehavior application).hidden action →
        (abstractBehavior application).step
          (mapState application before) action (mapState application after)) :=
    Nat.decidableForallFin _
  letI : Decidable (∀ before : ConcreteStateIndex application,
      ∀ action : ActionIndex application, ∀ after : ConcreteStateIndex application,
      (concreteBehavior application).step before action after →
      ¬ (concreteBehavior application).hidden action →
      (abstractBehavior application).step
        (mapState application before) action (mapState application after)) :=
    Nat.decidableForallFin _
  letI : Decidable (∀ action : ActionIndex application,
      ProfileConditions application
        ((concreteBehavior application).resources action)
        ((abstractBehavior application).resources action)) :=
    Nat.decidableForallFin _
  letI : Decidable (∀ action : ActionIndex application,
      (concreteBehavior application).hidden action →
      ProfileConditions application
        ((concreteBehavior application).resources action)
        (emptyProfile application.program)) :=
    Nat.decidableForallFin _
  if init : ∀ state : ConcreteStateIndex application,
      (concreteBehavior application).init state →
      (abstractBehavior application).init (mapState application state) then
    if observeState : ∀ state : ConcreteStateIndex application,
        (concreteBehavior application).observe state =
          (abstractBehavior application).observe (mapState application state) then
      if hiddenPreserved : ∀ action : ActionIndex application,
          (concreteBehavior application).hidden action ↔
            (abstractBehavior application).hidden action then
        if hiddenStep : ∀ (before : ConcreteStateIndex application)
            (action : ActionIndex application) (after : ConcreteStateIndex application),
            (concreteBehavior application).step before action after →
            (concreteBehavior application).hidden action →
            mapState application before = mapState application after then
          if visibleStep : ∀ (before : ConcreteStateIndex application)
              (action : ActionIndex application) (after : ConcreteStateIndex application),
              (concreteBehavior application).step before action after →
              ¬ (concreteBehavior application).hidden action →
              (abstractBehavior application).step
                (mapState application before) action (mapState application after) then
            if resources : ∀ action : ActionIndex application,
                ProfileConditions application
                  ((concreteBehavior application).resources action)
                  ((abstractBehavior application).resources action) then
              if hiddenResources : ∀ action : ActionIndex application,
                  (concreteBehavior application).hidden action →
                  ProfileConditions application
                    ((concreteBehavior application).resources action)
                    (emptyProfile application.program) then
                exact isTrue {
                  init, observeState, hiddenPreserved, hiddenStep, visibleStep,
                  resources, hiddenResources
                }
              else exact isFalse fun conditions => hiddenResources conditions.hiddenResources
            else exact isFalse fun conditions => resources conditions.resources
          else exact isFalse fun conditions => visibleStep conditions.visibleStep
        else exact isFalse fun conditions => hiddenStep conditions.hiddenStep
      else exact isFalse fun conditions => hiddenPreserved conditions.hiddenPreserved
    else exact isFalse fun conditions => observeState conditions.observeState
  else exact isFalse fun conditions => init conditions.init

def RefinementConditions.toResourceWeakRefinement
    {application : Application}
    (conditions : RefinementConditions application) :
    ResourceWeakRefinement
      (concreteBehavior application) (abstractBehavior application) where
  mapState := mapState application
  init := fun {state} => conditions.init state
  observe := id
  observeState := conditions.observeState
  hiddenPreserved := conditions.hiddenPreserved
  hiddenStep := fun {before action after} =>
    conditions.hiddenStep before action after
  visibleStep := fun {before action after} =>
    conditions.visibleStep before action after
  resources := fun action => (conditions.resources action).toResourceRefines
  hiddenResources := fun action hidden =>
    (conditions.hiddenResources action hidden).toResourceRefines

/-- Reverse requirement preservation needed when profiles enable world steps. -/
structure WorldRequirementConditions (application : Application) : Prop where
  requiresBack : ∀ (action : ActionIndex application)
      (capability : CapabilityIndex application),
    ((abstractBehavior application).resources action).requires capability →
    ((concreteBehavior application).resources action).requires capability

instance worldRequirementConditionsDecidable (application : Application) :
    Decidable (WorldRequirementConditions application) := by
  letI (action : ActionIndex application) (capability : CapabilityIndex application) :
      Decidable (
        ((abstractBehavior application).resources action).requires capability →
        ((concreteBehavior application).resources action).requires capability) := by
    if abstractRequires :
        ((abstractBehavior application).resources action).requires capability then
      if concreteRequires :
          ((concreteBehavior application).resources action).requires capability then
        exact isTrue fun _ => concreteRequires
      else exact isFalse fun implication =>
        concreteRequires (implication abstractRequires)
    else exact isTrue fun required => False.elim (abstractRequires required)
  letI (action : ActionIndex application) :
      Decidable (∀ capability : CapabilityIndex application,
        ((abstractBehavior application).resources action).requires capability →
        ((concreteBehavior application).resources action).requires capability) :=
    Nat.decidableForallFin _
  letI : Decidable (∀ action : ActionIndex application,
      ∀ capability : CapabilityIndex application,
        ((abstractBehavior application).resources action).requires capability →
        ((concreteBehavior application).resources action).requires capability) :=
    Nat.decidableForallFin _
  if requiresBack : ∀ action capability,
      ((abstractBehavior application).resources action).requires capability →
      ((concreteBehavior application).resources action).requires capability then
    exact isTrue { requiresBack }
  else exact isFalse fun conditions => requiresBack conditions.requiresBack

def WorldRequirementConditions.toWorldWeakRefinement
    {application : Application}
    (requirements : WorldRequirementConditions application)
    (refinement : RefinementConditions application) :
    WorldWeakRefinement
      (concreteBehavior application) (abstractBehavior application) where
  behavior := refinement.toResourceWeakRefinement
  worldResources := fun action => {
    profile := (refinement.resources action).toResourceRefines
    requiresBack := requirements.requiresBack action
  }

structure WiringConditions (application : Application) : Prop where
  connected : ∀ left right,
    application.concreteConnection left right ↔
      application.abstractConnection left right

instance wiringConditionsDecidable (application : Application) :
    Decidable (WiringConditions application) := by
  if connected : ∀ left right,
      application.concreteConnection left right ↔
        application.abstractConnection left right then
    exact isTrue { connected }
  else exact isFalse fun conditions => connected conditions.connected

def WiringConditions.toEquivalent
    {application : Application}
    (conditions : WiringConditions application) :
    WiringEquivalent application.concreteConnection
      application.abstractConnection where
  connected := conditions.connected

structure CompositionConditions
    (application : Application)
    (leftSystem : System)
    (extraHidden : List String)
    (connection : ActionIndex application → PeerActionIndex application → Prop) : Prop where
  directions : ∀ (leftAction : ActionIndex application)
      (rightAction : PeerActionIndex application), connection leftAction rightAction →
    Direction.Complementary
      ((toBehavior application.program leftSystem
        (actionNames application) extraHidden).direction leftAction)
      ((peerBehavior application).direction rightAction)
  payloads : ∀ (leftAction : ActionIndex application)
      (rightAction : PeerActionIndex application), connection leftAction rightAction →
    (toBehavior application.program leftSystem
      (actionNames application) extraHidden).payload leftAction =
      (peerBehavior application).payload rightAction
  hiddenLeft : ∀ (leftAction : ActionIndex application)
      (rightAction : PeerActionIndex application), connection leftAction rightAction →
    ¬ (toBehavior application.program leftSystem
      (actionNames application) extraHidden).hidden leftAction
  hiddenRight : ∀ (leftAction : ActionIndex application)
      (rightAction : PeerActionIndex application), connection leftAction rightAction →
    ¬ (peerBehavior application).hidden rightAction
  capabilities : ∀ capability,
    (toBehavior application.program leftSystem
      (actionNames application) extraHidden).owns capability →
    (peerBehavior application).owns capability → False
  resources : ∀ (leftAction : ActionIndex application)
      (rightAction : PeerActionIndex application), connection leftAction rightAction →
    ((toBehavior application.program leftSystem
      (actionNames application) extraHidden).direction leftAction = .output →
        SynchronizationConditions application
          ((toBehavior application.program leftSystem
            (actionNames application) extraHidden).resources leftAction)
          ((peerBehavior application).resources rightAction)) ∧
    ((peerBehavior application).direction rightAction = .output →
        SynchronizationConditions application
          ((peerBehavior application).resources rightAction)
          ((toBehavior application.program leftSystem
            (actionNames application) extraHidden).resources leftAction))

instance compositionConditionsDecidable
    (application : Application)
    (leftSystem : System)
    (extraHidden : List String)
    (connection : ActionIndex application → PeerActionIndex application → Prop)
    [∀ left right, Decidable (connection left right)] :
    Decidable (CompositionConditions application leftSystem extraHidden connection) := by
  if directions : ∀ (leftAction : ActionIndex application)
      (rightAction : PeerActionIndex application), connection leftAction rightAction →
      Direction.Complementary
        ((toBehavior application.program leftSystem
          (actionNames application) extraHidden).direction leftAction)
        ((peerBehavior application).direction rightAction) then
    if payloads : ∀ (leftAction : ActionIndex application)
        (rightAction : PeerActionIndex application), connection leftAction rightAction →
        (toBehavior application.program leftSystem
          (actionNames application) extraHidden).payload leftAction =
          (peerBehavior application).payload rightAction then
      if hiddenLeft : ∀ (leftAction : ActionIndex application)
          (rightAction : PeerActionIndex application), connection leftAction rightAction →
          ¬ (toBehavior application.program leftSystem
            (actionNames application) extraHidden).hidden leftAction then
        if hiddenRight : ∀ (leftAction : ActionIndex application)
            (rightAction : PeerActionIndex application), connection leftAction rightAction →
            ¬ (peerBehavior application).hidden rightAction then
          if capabilities : ∀ capability,
              (toBehavior application.program leftSystem
                (actionNames application) extraHidden).owns capability →
              (peerBehavior application).owns capability → False then
            if resources : ∀ (leftAction : ActionIndex application)
                (rightAction : PeerActionIndex application), connection leftAction rightAction →
                ((toBehavior application.program leftSystem
                  (actionNames application) extraHidden).direction leftAction = .output →
                    SynchronizationConditions application
                      ((toBehavior application.program leftSystem
                        (actionNames application) extraHidden).resources leftAction)
                      ((peerBehavior application).resources rightAction)) ∧
                ((peerBehavior application).direction rightAction = .output →
                    SynchronizationConditions application
                      ((peerBehavior application).resources rightAction)
                      ((toBehavior application.program leftSystem
                        (actionNames application) extraHidden).resources leftAction)) then
              exact isTrue {
                directions, payloads, hiddenLeft, hiddenRight, capabilities, resources
              }
            else exact isFalse fun conditions => resources conditions.resources
          else exact isFalse fun conditions => capabilities conditions.capabilities
        else exact isFalse fun conditions => hiddenRight conditions.hiddenRight
      else exact isFalse fun conditions => hiddenLeft conditions.hiddenLeft
    else exact isFalse fun conditions => payloads conditions.payloads
  else exact isFalse fun conditions => directions conditions.directions


def CompositionConditions.toComposable
    {application : Application}
    {leftSystem : System}
    {extraHidden : List String}
    {connection : ActionIndex application → PeerActionIndex application → Prop}
    (conditions :
      CompositionConditions application leftSystem extraHidden connection) :
    Composable
      (toBehavior application.program leftSystem
        (actionNames application) extraHidden)
      (peerBehavior application) connection where
  directions := fun {leftAction rightAction} =>
    conditions.directions leftAction rightAction
  payloads := fun {leftAction rightAction} =>
    conditions.payloads leftAction rightAction
  hiddenLeft := fun {leftAction rightAction} =>
    conditions.hiddenLeft leftAction rightAction
  hiddenRight := fun {leftAction rightAction} =>
    conditions.hiddenRight leftAction rightAction
  capabilities := conditions.capabilities
  resources := by
    intro leftAction rightAction connected
    let resources := conditions.resources leftAction rightAction connected
    exact ⟨
      fun output => (resources.1 output).toCompatible,
      fun output => (resources.2 output).toCompatible
    ⟩

structure ApplicationConditions (application : Application) : Prop where
  refinement : RefinementConditions application
  worldRequirements : WorldRequirementConditions application
  wiring : WiringConditions application
  concreteComposition :
    CompositionConditions application application.concrete
      application.refinement.hiddenActions application.concreteConnection
  abstractComposition :
    CompositionConditions application application.abstract
      application.refinement.hiddenActions application.abstractConnection

instance applicationConditionsDecidable (application : Application) :
    Decidable (ApplicationConditions application) := by
  if refinement : RefinementConditions application then
    if worldRequirements : WorldRequirementConditions application then
      if wiring : WiringConditions application then
        if concreteComposition :
            CompositionConditions application application.concrete
              application.refinement.hiddenActions application.concreteConnection then
          if abstractComposition :
              CompositionConditions application application.abstract
                application.refinement.hiddenActions application.abstractConnection then
            exact isTrue {
              refinement, worldRequirements, wiring,
              concreteComposition, abstractComposition
            }
          else exact isFalse fun conditions =>
            abstractComposition conditions.abstractComposition
        else exact isFalse fun conditions =>
          concreteComposition conditions.concreteComposition
      else exact isFalse fun conditions => wiring conditions.wiring
    else exact isFalse fun conditions =>
      worldRequirements conditions.worldRequirements
  else exact isFalse fun conditions => refinement conditions.refinement

structure Certificate (application : Application) where
  refinement :
    ResourceWeakRefinement
      (concreteBehavior application) (abstractBehavior application)
  worldRefinement :
    WorldWeakRefinement
      (concreteBehavior application) (abstractBehavior application)
  wiring :
    WiringEquivalent application.concreteConnection
      application.abstractConnection
  concreteComposition :
    Composable (concreteBehavior application) (peerBehavior application)
      application.concreteConnection
  abstractComposition :
    Composable (abstractBehavior application) (peerBehavior application)
      application.abstractConnection

def ApplicationConditions.toCertificate
    {application : Application}
    (conditions : ApplicationConditions application) : Certificate application where
  refinement := conditions.refinement.toResourceWeakRefinement
  worldRefinement :=
    conditions.worldRequirements.toWorldWeakRefinement conditions.refinement
  wiring := conditions.wiring.toEquivalent
  concreteComposition := conditions.concreteComposition.toComposable
  abstractComposition := conditions.abstractComposition.toComposable

theorem Certificate.lifted
    {application : Application} (certificate : Certificate application) :
    Nonempty
      (ResourceWeakRefinement
        (parallel (concreteBehavior application) (peerBehavior application)
          application.concreteConnection)
        (parallel (abstractBehavior application) (peerBehavior application)
          application.abstractConnection)) :=
  liftParallel certificate.refinement certificate.wiring
    certificate.concreteComposition certificate.abstractComposition

theorem Certificate.liftedSynchronized
    {application : Application} (certificate : Certificate application)
    {before after : DynamicConcreteState application}
    {leftAction : ActionIndex application}
    {rightAction : PeerActionIndex application}
    (step : DynamicConcreteStep application before
      (.sync leftAction rightAction) after) :
    NMLT.Behavior.ResourceWorld.ProductStep
      (abstractBehavior application) (peerBehavior application)
      BinaryOwner.component BinaryOwner.peer application.abstractConnection
      (mapProductState certificate.worldRefinement before)
      (.sync leftAction rightAction)
      (mapProductState certificate.worldRefinement after) :=
  liftSynchronized certificate.worldRefinement certificate.wiring
    certificate.concreteComposition certificate.abstractComposition step

/--
The decoded certificate carries the complete one-step dynamic product
simulation, including isolated visible transitions, peer transitions,
synchronization, and resource-safe hidden stuttering.
-/
def Certificate.liftedDynamic
    {application : Application} (certificate : Certificate application) :
    DynamicProductRefinement
      (concreteBehavior application) (abstractBehavior application)
      (peerBehavior application) BinaryOwner.component BinaryOwner.peer
      application.concreteConnection application.abstractConnection :=
  liftProductSteps certificate.worldRefinement certificate.wiring
    certificate.concreteComposition certificate.abstractComposition

theorem Certificate.liftedStep
    {application : Application} (certificate : Certificate application)
    {before after : DynamicConcreteState application}
    {action : ProductAction
      (ActionIndex application) (PeerActionIndex application)}
    (step : DynamicConcreteStep application before action after) :
    DynamicStepMatch
      (concreteBehavior application)
      (abstractBehavior application) (peerBehavior application)
      BinaryOwner.component BinaryOwner.peer application.abstractConnection
      (mapProductState certificate.worldRefinement before) action
      (mapProductState certificate.worldRefinement after) :=
  certificate.liftedDynamic.matchStep step

def certify (application : Application) : Except String (Certificate application) :=
  if _worldRequirements : WorldRequirementConditions application then
    if conditions : ApplicationConditions application then
      pure conditions.toCertificate
    else
      throw s!"artifact theorem application failed for '{application.refinement.concrete} refines {application.refinement.abstract}'"
  else
    throw (s!"artifact dynamic refinement failed for " ++
      s!"'{application.refinement.concrete} refines {application.refinement.abstract}': " ++
      "abstract requirements are not preserved by concrete world-step enabledness")

structure ClosureSummary where
  program : Summary
  applications : List ApplicationSummary
deriving Repr

def close (program : Program) : Except String ClosureSummary := do
  let mut accepted := []
  for refinement in program.refinements do
    match matchingApplications program refinement with
    | [application] =>
        let _ ← certify application
        accepted := accepted ++ [applicationSummary application]
    | candidates =>
        throw (s!"artifact theorem application failed: refinement " ++
          s!"'{refinement.concrete} refines {refinement.abstract}' needs exactly " ++
          s!"one paired composition, found {candidates.length}")
  pure {
    program := program.summary
    applications := accepted
  }

def parseAndClose (input : String) : Except String ClosureSummary := do
  close (← parseProgram input)

end NMLT.Artifact.SemanticClosure
