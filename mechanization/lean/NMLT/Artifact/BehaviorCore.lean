import Lean.Data.Json
import NMLT.Behavior.ResourceBehavior

namespace NMLT.Artifact.BehaviorCore

open Lean
open NMLT.Behavior.ResourceBehavior

abbrev JObject := Std.TreeMap.Raw String Json compare

structure Summary where
  sourcePath : String
  sourceSha256 : String
  systems : Nat
  compositions : Nat
  refinements : Nat
deriving Repr

structure Profile where
  requires : List String
  consumes : List String
  transfers : List String
  receives : List String
  grade : List (String × Nat)
  relies : List String
  guarantees : List String
deriving Repr, BEq

private def reject (message : String) : Except String α :=
  throw s!"behavior-core-v1: {message}"

private def lookup (object : JObject) (key : String) : Except String Json :=
  match object.get? key with
  | some value => pure value
  | none => reject s!"missing property '{key}'"

private def objectAt (json : Json) (key : String) : Except String JObject := do
  let object ← json.getObj?
  let value ← lookup object key
  value.getObj?

private def stringAt (json : Json) (key : String) : Except String String := do
  let value ← json.getObjVal? key
  value.getStr?

private def boolAt (json : Json) (key : String) : Except String Bool := do
  let value ← json.getObjVal? key
  value.getBool?

private def strictlySorted : List String → Bool
  | [] | [_] => true
  | first :: second :: rest => decide (first < second) && strictlySorted (second :: rest)

private def strings (json : Json) : Except String (List String) := do
  let array ← json.getArr?
  array.toList.mapM fun value => value.getStr?

private def sortedStrings (json : Json) (context : String) : Except String (List String) := do
  let values ← strings json
  if strictlySorted values then
    pure values
  else
    reject s!"'{context}' must be strictly sorted and duplicate-free"

private def profile (action : Json) : Except String Profile := do
  let resources ← action.getObjVal? "resources"
  let gradeJson ← resources.getObjVal? "grade"
  let gradeObject ← gradeJson.getObj?
  let grade ← gradeObject.toList.mapM fun (atom, value) => do
    pure (atom, ← value.getNat?)
  let requires ← sortedStrings (← resources.getObjVal? "requires") "requires"
  let consumes ← sortedStrings (← resources.getObjVal? "consumes") "consumes"
  let transfers ← sortedStrings (← resources.getObjVal? "transfers") "transfers"
  let receives ← sortedStrings (← resources.getObjVal? "receives") "receives"
  let relies ← sortedStrings (← resources.getObjVal? "relies") "relies"
  let guarantees ← sortedStrings (← resources.getObjVal? "guarantees") "guarantees"
  pure {
    requires
    consumes
    transfers
    receives
    grade
    relies
    guarantees
  }

private def subset (left right : List String) : Bool :=
  left.all fun item => right.contains item

private def profileRefines (concrete abstract : Profile) : Bool :=
  let abstractGrade (atom : String) := (abstract.grade.lookup atom).getD 0
  subset concrete.requires abstract.requires &&
    concrete.consumes == abstract.consumes &&
    concrete.transfers == abstract.transfers &&
    concrete.receives == abstract.receives &&
    concrete.grade.all (fun (atom, amount) => amount ≤ abstractGrade atom) &&
    subset concrete.relies abstract.relies &&
    subset abstract.guarantees concrete.guarantees

private def stutterCompatible (resources : Profile) : Bool :=
  resources.requires.isEmpty && resources.consumes.isEmpty &&
    resources.transfers.isEmpty && resources.receives.isEmpty &&
    resources.grade.all (fun (_, amount) => amount == 0) &&
    resources.relies.isEmpty

private partial def validateTerm
    (facts : List String) (state : JObject) (expected : String) (term : Json) : Except String Unit := do
  let kind ← stringAt term "kind"
  let encodedType ← stringAt term "type"
  if encodedType != expected then
    reject s!"term kind '{kind}' has type '{encodedType}', expected '{expected}'"
  match kind with
  | "bool" =>
      if expected != "Bool" then reject "Bool literal has a non-Bool type"
      let _ ← boolAt term "value"
  | "unit" =>
      if expected != "Unit" then reject "Unit literal has a non-Unit type"
  | "enum" =>
      let constructor ← stringAt term "constructor"
      if !facts.contains s!"{expected}.{constructor}" then
        reject s!"unknown constructor '{expected}.{constructor}'"
  | "read" =>
      let field ← stringAt term "field"
      let fieldState ← lookup state field
      let fieldType ← stringAt fieldState "type"
      if fieldType != expected then
        reject s!"read of '{field}' has type '{fieldType}', expected '{expected}'"
  | "not" =>
      if expected != "Bool" then reject "negation has a non-Bool type"
      validateTerm facts state "Bool" (← term.getObjVal? "value")
  | "equal" =>
      if expected != "Bool" then reject "equality has a non-Bool result type"
      let left ← term.getObjVal? "left"
      let right ← term.getObjVal? "right"
      let operandType ← stringAt left "type"
      validateTerm facts state operandType left
      validateTerm facts state operandType right
  | _ => reject s!"unknown term kind '{kind}'"

private partial def renderTerm (term : Json) : Except String String := do
  match ← stringAt term "kind" with
  | "bool" => pure (if ← boolAt term "value" then "true" else "false")
  | "unit" => pure "unit"
  | "enum" => stringAt term "constructor"
  | "read" => stringAt term "field"
  | "not" =>
      let value ← term.getObjVal? "value"
      pure s!"!{← renderTerm value}"
  | "equal" =>
      let left ← term.getObjVal? "left"
      let right ← term.getObjVal? "right"
      let leftText ← renderTerm left
      let rightText ← renderTerm right
      pure s!"{leftText} == {rightText}"
  | kind => reject s!"cannot render unknown term kind '{kind}'"

private partial def closedInitializer (term : Json) : Except String Bool := do
  match ← stringAt term "kind" with
  | "bool" | "unit" | "enum" => pure true
  | "read" => pure false
  | "not" => closedInitializer (← term.getObjVal? "value")
  | "equal" =>
      pure ((← closedInitializer (← term.getObjVal? "left")) &&
        (← closedInitializer (← term.getObjVal? "right")))
  | kind => reject s!"unknown term kind '{kind}'"

private def actionAt (system : Json) (name : String) : Except String Json := do
  lookup (← objectAt system "actions") name

private def systemAt (systems : JObject) (name : String) : Except String Json :=
  lookup systems name

private def validateAction
    (facts : List String) (state capabilities ports : JObject)
    (systemName actionName : String) (action : Json) : Except String Unit := do
  let direction ← stringAt action "direction"
  let parametersJson ← action.getObjVal? "parameters"
  let parameters ← parametersJson.getArr?
  let parameterNames ← parameters.toList.mapM fun parameter => stringAt parameter "name"
  let outputs ← strings (← action.getObjVal? "outputs")
  let resources ← profile action
  let ownedCapabilities := capabilities.toList.map Prod.fst
  if !subset resources.requires ownedCapabilities ||
      !subset resources.consumes ownedCapabilities ||
      !subset resources.transfers ownedCapabilities then
    reject s!"action '{systemName}.{actionName}' uses authority it does not own"
  if resources.consumes.any resources.transfers.contains then
    reject s!"action '{systemName}.{actionName}' both consumes and transfers the same authority"
  if resources.receives.any ownedCapabilities.contains then
    reject s!"action '{systemName}.{actionName}' receives authority it already owns"
  if direction != "internal" && direction != "input" && direction != "output" then
    reject s!"action '{systemName}.{actionName}' has invalid direction '{direction}'"
  if direction == "internal" then
    if ports.contains actionName then
      reject s!"internal action '{systemName}.{actionName}' cannot own a boundary port"
    if !parameterNames.isEmpty || !outputs.isEmpty || !resources.receives.isEmpty ||
        !resources.transfers.isEmpty then
      reject s!"internal action '{systemName}.{actionName}' cannot bind or move a port payload"
  else
    let port ← lookup ports actionName
    let portDirection ← stringAt port "direction"
    if portDirection != direction then
      reject s!"action and port '{systemName}.{actionName}' disagree on direction"
    let payload ← stringAt port "payload"
    if direction == "input" then
      if !outputs.isEmpty || !resources.transfers.isEmpty then
        reject s!"input action '{systemName}.{actionName}' cannot emit or transfer authority"
      if payload == "Unit" then
        if parameters.size != 0 || !resources.receives.isEmpty then
          reject s!"input action '{systemName}.{actionName}' has a nonempty Unit payload binding"
      else
        if parameters.size != 1 || (← stringAt parameters[0]! "type") != payload then
          reject s!"input action '{systemName}.{actionName}' does not bind its port payload"
        if resources.receives != parameterNames then
          reject s!"input action '{systemName}.{actionName}' receive profile does not match its payload binding"
    else
      if !parameterNames.isEmpty || !resources.receives.isEmpty then
        reject s!"output action '{systemName}.{actionName}' cannot bind or receive authority"
      if payload == "Unit" then
        if !outputs.isEmpty || !resources.transfers.isEmpty then
          reject s!"output action '{systemName}.{actionName}' emits a value on a Unit port"
      else
        if outputs.length != 1 then
          reject s!"output action '{systemName}.{actionName}' does not emit its port payload"
        let outputType ← lookup capabilities outputs[0]!
        if (← outputType.getStr?) != payload then
          reject s!"output action '{systemName}.{actionName}' payload type does not match its port"
        if resources.transfers != outputs then
          reject s!"output action '{systemName}.{actionName}' transfer profile does not match its payload"
  let _ ← boolAt action "hidden"
  if !subset resources.relies facts || !subset resources.guarantees facts then
    reject s!"action '{systemName}.{actionName}' names an unknown contract fact"
  let guardsJson ← action.getObjVal? "guards"
  let guards ← strings guardsJson
  let guardAstJson ← action.getObjVal? "guard_ast"
  let guardAst ← guardAstJson.getArr?
  if guards.length != guardAst.size then
    reject s!"action '{systemName}.{actionName}' has mismatched guard text and AST counts"
  for term in guardAst do
    validateTerm facts state "Bool" term
  for index in [:guards.length] do
    if guards[index]! != (← renderTerm guardAst[index]!) then
      reject s!"action '{systemName}.{actionName}' guard text does not match its AST"
  let updates ← objectAt action "updates"
  let updateAst ← objectAt action "update_ast"
  if updates.toList.map Prod.fst != updateAst.toList.map Prod.fst then
    reject s!"action '{systemName}.{actionName}' has mismatched update text and AST fields"
  for (field, term) in updateAst.toList do
    let fieldState ← lookup state field
    validateTerm facts state (← stringAt fieldState "type") term
    let updateTextJson ← lookup updates field
    let updateText ← updateTextJson.getStr?
    if updateText != (← renderTerm term) then
      reject s!"action '{systemName}.{actionName}' update text does not match its AST"
  pure ()

private def validateSystem (facts : List String) (name : String) (system : Json) : Except String Unit := do
  let states ← objectAt system "state"
  let capabilities ← objectAt system "capabilities"
  let ports ← objectAt system "ports"
  let actions ← objectAt system "actions"
  let observe ← system.getObjVal? "observe"
  let observations ← strings observe
  for (field, state) in states.toList do
    let stateType ← stringAt state "type"
    if stateType != "Bool" && stateType != "Unit" &&
        !facts.any (fun fact => fact.startsWith s!"{stateType}.") then
      reject s!"state '{name}.{field}' is outside the finite core"
    let initial ← stringAt state "initial"
    let initialAst ← state.getObjVal? "initial_ast"
    validateTerm facts states stateType initialAst
    if !(← closedInitializer initialAst) then
      reject s!"state '{name}.{field}' initializer must be closed in behavior-core-v1"
    if initial != (← renderTerm initialAst) then
      reject s!"state '{name}.{field}' initializer text does not match its AST"
  for (capability, capabilityType) in capabilities.toList do
    let encoded ← capabilityType.getStr?
    if !encoded.startsWith "Once<" then
      reject s!"capability '{name}.{capability}' is not affine"
  for (portName, port) in ports.toList do
    let direction ← stringAt port "direction"
    if direction != "input" && direction != "output" then
      reject s!"port '{name}.{portName}' has invalid direction"
    let _ ← stringAt port "payload"
  for observed in observations do
    if !states.contains observed then
      reject s!"observation '{name}.{observed}' is not a state field"
  for (actionName, action) in actions.toList do
    validateAction facts states capabilities ports name actionName action

private def validateConnection (systems : JObject) (connection : Json) : Except String Unit := do
  let leftSystemName ← stringAt connection "left_system"
  let rightSystemName ← stringAt connection "right_system"
  let leftActionName ← stringAt connection "left_action"
  let rightActionName ← stringAt connection "right_action"
  let leftSystem ← systemAt systems leftSystemName
  let rightSystem ← systemAt systems rightSystemName
  let leftAction ← actionAt leftSystem leftActionName
  let rightAction ← actionAt rightSystem rightActionName
  let leftHidden ← boolAt leftAction "hidden"
  let rightHidden ← boolAt rightAction "hidden"
  if leftHidden || rightHidden then
    reject "a hidden action is connected across a composition boundary"
  let leftDirection ← stringAt leftAction "direction"
  let rightDirection ← stringAt rightAction "direction"
  if !((leftDirection == "output" && rightDirection == "input") ||
      (leftDirection == "input" && rightDirection == "output")) then
    reject "connected actions must have complementary directions"
  let leftPort ← lookup (← objectAt leftSystem "ports") leftActionName
  let rightPort ← lookup (← objectAt rightSystem "ports") rightActionName
  if (← stringAt leftPort "payload") != (← stringAt rightPort "payload") then
    reject "connected actions must have equal payload types"
  let leftResources ← profile leftAction
  let rightResources ← profile rightAction
  let sender := if leftDirection == "output" then leftResources else rightResources
  let receiver := if leftDirection == "output" then rightResources else leftResources
  if sender.transfers != receiver.receives || receiver.transfers != sender.receives then
    reject "connected transfer and receive profiles do not match exactly"
  if !subset sender.relies receiver.guarantees ||
      !subset receiver.relies sender.guarantees then
    reject "a synchronized reliance is not discharged by the peer guarantee"

private def validateComposition
    (systems : JObject) (name : String) (composition : Json) : Except String Unit := do
  let leftName ← stringAt composition "left"
  let rightName ← stringAt composition "right"
  if leftName == rightName then
    reject s!"composition '{name}' must contain two distinct systems"
  let left ← systemAt systems leftName
  let right ← systemAt systems rightName
  let leftCapabilityObject ← objectAt left "capabilities"
  let rightCapabilityObject ← objectAt right "capabilities"
  let leftCapabilities := leftCapabilityObject.toList.map Prod.fst
  let rightCapabilities := rightCapabilityObject.toList.map Prod.fst
  if !leftCapabilities.all (fun capability => !rightCapabilities.contains capability) then
    reject s!"composition '{name}' violates the capability partition"
  let connectionJson ← composition.getObjVal? "connections"
  let connections ← connectionJson.getArr?
  let endpoints ← connections.toList.mapM fun connection => do
    pure ((← stringAt connection "left_action"),
      (← stringAt connection "right_action"))
  let leftEndpoints := endpoints.map Prod.fst
  let rightEndpoints := endpoints.map Prod.snd
  if leftEndpoints.length != leftEndpoints.eraseDups.length ||
      rightEndpoints.length != rightEndpoints.eraseDups.length then
    reject s!"composition '{name}' connections must be one-to-one"
  for connection in connections do
    let connectionLeft ← stringAt connection "left_system"
    let connectionRight ← stringAt connection "right_system"
    if connectionLeft != leftName || connectionRight != rightName then
      reject s!"composition '{name}' contains an endpoint outside its binary product"
    validateConnection systems connection

private def validateRefinement (systems : JObject) (refinement : Json) : Except String Unit := do
  let concreteName ← stringAt refinement "concrete"
  let abstractName ← stringAt refinement "abstract"
  let concrete ← systemAt systems concreteName
  let abstract ← systemAt systems abstractName
  let stateMap ← objectAt refinement "state_map"
  let concreteStateObject ← objectAt concrete "state"
  let abstractStateObject ← objectAt abstract "state"
  let concreteStates := concreteStateObject.toList.map Prod.fst
  let abstractStates := abstractStateObject.toList.map Prod.fst
  if stateMap.size != concreteStates.length then
    reject s!"refinement '{concreteName} refines {abstractName}' has an incomplete state map"
  let mapped ← stateMap.toList.mapM fun (_, value) => value.getStr?
  if !concreteStates.all (fun field => stateMap.contains field) ||
      !abstractStates.all (fun field => mapped.contains field) ||
      mapped.length != mapped.eraseDups.length then
    reject s!"refinement '{concreteName} refines {abstractName}' does not map states bijectively"
  for (concreteField, abstractFieldJson) in stateMap.toList do
    let abstractField ← abstractFieldJson.getStr?
    let concreteDecl ← lookup concreteStateObject concreteField
    let abstractDecl ← lookup abstractStateObject abstractField
    let concreteType ← stringAt concreteDecl "type"
    let abstractType ← stringAt abstractDecl "type"
    if concreteType != abstractType then
      reject (s!"refinement '{concreteName} refines {abstractName}' maps " ++
        s!"incompatible state types '{concreteType}' and '{abstractType}'")
  let hiddenJson ← refinement.getObjVal? "hidden_actions"
  let hidden ← sortedStrings hiddenJson "hidden_actions"
  let concreteActions ← objectAt concrete "actions"
  if !hidden.all concreteActions.contains then
    reject s!"refinement '{concreteName} refines {abstractName}' hides an unknown action"
  for (actionName, action) in concreteActions.toList do
    let actionHidden ← boolAt action "hidden"
    let isHidden := actionHidden || hidden.contains actionName
    let concreteProfile ← profile action
    if isHidden then
      let updates ← objectAt action "updates"
      for (field, valueJson) in updates.toList do
        let value ← valueJson.getStr?
        if value != field then
          reject s!"hidden action '{concreteName}.{actionName}' changes mapped state"
      let updateAst ← objectAt action "update_ast"
      for (field, term) in updateAst.toList do
        let kind ← stringAt term "kind"
        if kind != "read" || (← stringAt term "field") != field then
          reject s!"hidden action '{concreteName}.{actionName}' AST changes mapped state"
      if !stutterCompatible concreteProfile then
        reject s!"hidden action '{concreteName}.{actionName}' cannot refine stutter"
    else
      let abstractAction ← actionAt abstract actionName
      let abstractProfile ← profile abstractAction
      if !profileRefines concreteProfile abstractProfile then
        reject s!"action '{concreteName}.{actionName}' does not resource-refine its abstract action"

private def validSha256 (digest : String) : Bool :=
  digest.length == 64 && digest.toList.all fun character =>
    character.isDigit || ('a' ≤ character && character ≤ 'f')

/-- Decode and semantically validate the finite `behavior-core-v1` envelope. -/
def decode (json : Json) : Except String Summary := do
  let schema ← stringAt json "schema"
  if schema != "behavior-core-v1" then
    reject s!"unsupported schema '{schema}'"
  let sourcePath ← stringAt json "source_path"
  let sourceSha256 ← stringAt json "source_sha256"
  if !validSha256 sourceSha256 then
    reject "source_sha256 is not a lowercase SHA-256 digest"
  let enums ← objectAt json "enums"
  let mut facts := []
  for (enumName, variantsJson) in enums.toList do
    let variants ← sortedStrings variantsJson s!"enum {enumName}"
    if variants.isEmpty then
      reject s!"enum '{enumName}' must have at least one constructor"
    facts := facts ++ variants.map fun variant => s!"{enumName}.{variant}"
  let systems ← objectAt json "systems"
  if systems.size == 0 then
    reject "at least one behavior system is required"
  for (name, system) in systems.toList do
    validateSystem facts name system
  let compositions ← objectAt json "compositions"
  for (name, composition) in compositions.toList do
    validateComposition systems name composition
  let refinementJson ← json.getObjVal? "refinements"
  let refinements ← refinementJson.getArr?
  for refinement in refinements do
    validateRefinement systems refinement
  pure {
    sourcePath
    sourceSha256
    systems := systems.size
    compositions := compositions.size
    refinements := refinements.size
  }

def parse (input : String) : Except String Summary := do
  decode (← Json.parse input)

inductive Term where
  | bool (value : Bool)
  | unit
  | enumeration (typeName constructor : String)
  | read (typeName field : String)
  | not (value : Term)
  | equal (left right : Term)
deriving Repr, BEq

structure StateDecl where
  name : String
  typeName : String
  initial : Term
deriving Repr, BEq

structure Port where
  name : String
  direction : Direction
  payload : String
deriving Repr, BEq

structure Action where
  name : String
  direction : Direction
  hidden : Bool
  guards : List Term
  updates : List (String × Term)
  resources : Profile
deriving Repr, BEq

structure System where
  name : String
  state : List StateDecl
  capabilities : List (String × String)
  ports : List Port
  actions : List Action
  observe : List String
deriving Repr, BEq

structure Connection where
  leftSystem : String
  leftAction : String
  rightSystem : String
  rightAction : String
deriving Repr, BEq

structure Composition where
  name : String
  left : String
  right : String
  connections : List Connection
deriving Repr, BEq

structure Refinement where
  concrete : String
  abstract : String
  stateMap : List (String × String)
  hiddenActions : List String
deriving Repr, BEq

structure Program where
  summary : Summary
  enums : List (String × List String)
  facts : List String
  systems : List System
  compositions : List Composition
  refinements : List Refinement
deriving Repr

private partial def decodeTerm (term : Json) : Except String Term := do
  let kind ← stringAt term "kind"
  match kind with
  | "bool" => pure (.bool (← boolAt term "value"))
  | "unit" => pure .unit
  | "enum" =>
      pure (.enumeration (← stringAt term "type") (← stringAt term "constructor"))
  | "read" =>
      pure (.read (← stringAt term "type") (← stringAt term "field"))
  | "not" =>
      pure (.not (← decodeTerm (← term.getObjVal? "value")))
  | "equal" =>
      pure (.equal
        (← decodeTerm (← term.getObjVal? "left"))
        (← decodeTerm (← term.getObjVal? "right")))
  | _ => reject s!"cannot construct semantics for unknown term kind '{kind}'"

private def decodeDirection (encoded : String) : Except String Direction :=
  match encoded with
  | "internal" => pure .internal
  | "input" => pure .input
  | "output" => pure .output
  | _ => reject s!"cannot construct semantics for direction '{encoded}'"

private def decodeSystem (name : String) (system : Json) : Except String System := do
  let states ← objectAt system "state"
  let state ← states.toList.mapM fun (field, declaration) => do
    pure {
      name := field
      typeName := (← stringAt declaration "type")
      initial := (← decodeTerm (← declaration.getObjVal? "initial_ast"))
    }
  let capabilitiesObject ← objectAt system "capabilities"
  let capabilities ← capabilitiesObject.toList.mapM fun (capability, encodedType) => do
    pure (capability, ← encodedType.getStr?)
  let portsObject ← objectAt system "ports"
  let ports ← portsObject.toList.mapM fun (portName, port) => do
    pure {
      name := portName
      direction := (← decodeDirection (← stringAt port "direction"))
      payload := (← stringAt port "payload")
    }
  let actionsObject ← objectAt system "actions"
  let actions ← actionsObject.toList.mapM fun (actionName, action) => do
    let guardJson ← (← action.getObjVal? "guard_ast").getArr?
    let guards ← guardJson.toList.mapM decodeTerm
    let updateJson ← objectAt action "update_ast"
    let updates ← updateJson.toList.mapM fun (field, term) => do
      pure (field, ← decodeTerm term)
    pure {
      name := actionName
      direction := (← decodeDirection (← stringAt action "direction"))
      hidden := (← boolAt action "hidden")
      guards
      updates
      resources := (← profile action)
    }
  pure {
    name
    state
    capabilities
    ports
    actions
    observe := (← strings (← system.getObjVal? "observe"))
  }

private def decodeComposition (name : String) (composition : Json) :
    Except String Composition := do
  let connectionJson ← (← composition.getObjVal? "connections").getArr?
  let connections ← connectionJson.toList.mapM fun connection => do
    pure {
      leftSystem := (← stringAt connection "left_system")
      leftAction := (← stringAt connection "left_action")
      rightSystem := (← stringAt connection "right_system")
      rightAction := (← stringAt connection "right_action")
    }
  pure {
    name
    left := (← stringAt composition "left")
    right := (← stringAt composition "right")
    connections
  }

private def decodeRefinement (refinement : Json) : Except String Refinement := do
  let stateMapObject ← objectAt refinement "state_map"
  let stateMap ← stateMapObject.toList.mapM fun (concrete, abstract) => do
    pure (concrete, ← abstract.getStr?)
  pure {
    concrete := (← stringAt refinement "concrete")
    abstract := (← stringAt refinement "abstract")
    stateMap
    hiddenActions := (← strings (← refinement.getObjVal? "hidden_actions"))
  }

/--
Construct the finite semantic data after the normative decoder has accepted
the artifact. This is intentionally separate from Summary: downstream Lean
code receives the actual terms, transitions, resources, wirings, and maps.
-/
def decodeProgram (json : Json) : Except String Program := do
  let summary ← decode json
  let enumsObject ← objectAt json "enums"
  let enums ← enumsObject.toList.mapM fun (enumName, variantsJson) => do
    pure (enumName, ← strings variantsJson)
  let mut facts := []
  for (enumName, variants) in enums do
    facts := facts ++ variants.map fun variant => s!"{enumName}.{variant}"
  let systemsObject ← objectAt json "systems"
  let systems ← systemsObject.toList.mapM fun (name, system) =>
    decodeSystem name system
  let compositionsObject ← objectAt json "compositions"
  let compositions ← compositionsObject.toList.mapM fun (name, composition) =>
    decodeComposition name composition
  let refinementJson ← (← json.getObjVal? "refinements").getArr?
  let refinements ← refinementJson.toList.mapM decodeRefinement
  pure { summary, enums, facts, systems, compositions, refinements }

def parseProgram (input : String) : Except String Program := do
  decodeProgram (← Json.parse input)

end NMLT.Artifact.BehaviorCore
