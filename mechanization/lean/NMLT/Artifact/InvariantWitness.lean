import NMLT.Artifact.FiniteInvariant
import NMLT.Artifact.SafetyPredicate

namespace NMLT.Artifact.InvariantWitness
open Lean (Json)
open NMLT.Artifact.BehaviorCore NMLT.Artifact.SemanticClosure
open NMLT.Artifact.ExecutionClosure NMLT.Artifact.ExecutionWitness
open NMLT.Artifact.SafetyPredicate
open NMLT.Behavior.ResourceDynamics NMLT.Behavior.ResourceWorld

def predicateAt (m : Model) (right : Bool) (predicate : Predicate) (s : m.State) : Bool :=
  let state := if right then stateAt? m.program m.right s.control.2
    else stateAt? m.program m.left s.control.1
  match state with
  | some values => evaluate predicate values
  | none => false

private def controlSize (program : Program) (system : System) : Except String Nat := do
  let mut count := 1
  for field in system.state do
    count := count * (typeDomain program field.typeName).length
    unless 0 < count && count ≤ 512 do throw "safety: control universe exceeds bound"
  pure (count + 1)

def universeBounds (m : Model) : Except String (Nat × Nat) := do
  let left ← controlSize m.program m.left
  let right ← controlSize m.program m.right
  let caps := (capabilityNames m.program).length
  unless caps ≤ 4 do throw "safety: capability universe exceeds bound"
  let stateCount := left * right * 3 ^ (caps + 1)
  let la := m.leftActions.length + 1
  let ra := m.rightActions.length + 1
  let actionCount := la + ra + la * ra
  unless stateCount ≤ 512 && actionCount ≤ 256 do throw "safety: complete semantic universe exceeds bound"
  pure (stateCount, actionCount)

private def checkedProperty (m : Model) (source : String) (property : Json) : Except String (Bool × Predicate) := do
  keys property ["system", "name", "expression", "expression_start", "expression_end", "declaration_start", "declaration_end", "predicate"]
  let systemName ← stringAt property "system"
  let name ← stringAt property "name"
  unless !name.isEmpty && name.utf8ByteSize ≤ 256 do throw "safety: invalid property name"
  let right ← if systemName == m.left.name then pure false
    else if systemName == m.right.name then pure true
    else throw "safety: property system is not a selected leaf"
  let system := if right then m.right else m.left
  let expression ← stringAt property "expression"
  let start ← (← property.getObjVal? "expression_start").getNat?
  let finish ← (← property.getObjVal? "expression_end").getNat?
  unless start ≤ finish && finish ≤ source.utf8ByteSize do throw "safety: expression span is outside source"
  let some actual := String.fromUTF8? (source.toUTF8.extract start finish) |
    throw "safety: expression span is not aligned to UTF-8"
  unless actual == expression do throw "safety: expression bytes differ from source slice"
  let declarationStart ← (← property.getObjVal? "declaration_start").getNat?
  let declarationEnd ← (← property.getObjVal? "declaration_end").getNat?
  unless declarationStart ≤ start && finish ≤ declarationEnd && declarationEnd ≤ source.utf8ByteSize do
    throw "safety: expression is outside its source declaration"
  let some declaration := String.fromUTF8? (source.toUTF8.extract declarationStart declarationEnd) |
    throw "safety: declaration span is not aligned to UTF-8"
  checkDeclaration name expression declaration
  let parsed ← SafetyPredicate.parse m.program system expression
  let encoded ← SafetyPredicate.decode m.program system 64 (← property.getObjVal? "predicate")
  unless parsed == encoded do throw "safety: predicate tree differs from independently parsed source expression"
  pure (right, parsed)

structure Certificate where
  model : Model
  right : Bool
  predicate : Predicate
  reached : List model.State
  formed : model.formationCondition
  initial : model.State
  admitted : model.behavior.init initial
  initialized : FiniteInvariant.initialization model reached (predicateAt model right predicate) = true
  preserved : FiniteInvariant.preservation model reached (predicateAt model right predicate) = true

theorem Certificate.safe (c : Certificate) (s : c.model.State)
    (reachable : Reachable c.model.behavior s) : predicateAt c.model c.right c.predicate s = true :=
  FiniteInvariant.reachable_safe c.model c.reached (predicateAt c.model c.right c.predicate)
    c.initialized c.preserved s reachable

structure Counterexample where
  path : ExecutionWitness.Certificate
  right : Bool
  predicate : Predicate
  violated : predicateAt path.model right predicate path.last = false

theorem Counterexample.reachable_violation (c : Counterexample) :
    ∃ s, Reachable c.path.model.behavior s ∧ predicateAt c.path.model c.right c.predicate s = false :=
  ⟨c.path.last, c.path.reachable, c.violated⟩

inductive Checked where
  | invariant (certificate : Certificate)
  | counterexample (certificate : Counterexample)

def close (core artifact witness : Json) (source : String) : Except String Checked := do
  keys artifact ["schema", "core_sha256", "source_sha256", "behavior", "property"]
  keys witness ["schema", "core_sha256", "invariant_sha256", "claim"]
  unless (← stringAt artifact "schema") == "behavior-invariant-v1" do throw "safety: unsupported artifact schema"
  unless (← stringAt witness "schema") == "behavior-invariant-witness-v1" do throw "safety: unsupported witness schema"
  unless (← stringAt artifact "core_sha256") == (← stringAt witness "core_sha256") do throw "safety: core digest mismatch"
  unless (← stringAt artifact "source_sha256") == (← stringAt core "source_sha256") do throw "safety: source identity mismatch"
  let behavior ← stringAt artifact "behavior"
  let property ← artifact.getObjVal? "property"
  let claim ← witness.getObjVal? "claim"
  match ← stringAt claim "kind" with
  | "invariant" =>
    keys claim ["kind", "states"]
    let model ← decodeModel core behavior
    let (stateCount, actionCount) ← universeBounds model
    let (right, predicate) ← checkedProperty model source property
    let rawStates ← (← claim.getObjVal? "states").getArr?
    unless 0 < rawStates.size && rawStates.size ≤ 256 && stateCount * actionCount * rawStates.size ≤ 1000000 do
      throw "safety: state/obligation bound exceeded"
    let reached ← rawStates.toList.mapM (decodeState model)
    let some initial := reached.find? (fun s => decide (model.behavior.init s)) |
      throw "safety: no admitted initial state in certificate"
    if hf : model.formationCondition then
      if hi : model.behavior.init initial then
        if h0 : FiniteInvariant.initialization model reached (predicateAt model right predicate) = true then
          if h1 : FiniteInvariant.preservation model reached (predicateAt model right predicate) = true then
            pure (.invariant ⟨model, right, predicate, reached, hf, initial, hi, h0, h1⟩)
          else throw "safety: step-preservation obligation failed"
        else throw "safety: initialization obligation failed"
      else throw "safety: supplied initial state is not admitted"
    else throw "safety: selected binary product is not formed"
  | "counterexample" =>
    keys claim ["kind", "path"]
    let rawPath ← claim.getObjVal? "path"
    unless (← stringAt rawPath "behavior") == behavior &&
        (← stringAt rawPath "artifact_sha256") == (← stringAt artifact "core_sha256") do
      throw "safety: counterexample target or core digest mismatch"
    let model ← decodeModel core behavior
    let _ ← universeBounds model
    let rawStates ← (← rawPath.getObjVal? "states").getArr?
    let rawActions ← (← rawPath.getObjVal? "actions").getArr?
    unless rawStates.size ≤ 257 && rawActions.size ≤ 256 do throw "safety: counterexample path exceeds bound"
    let path ← ExecutionWitness.close core rawPath
    let (right, predicate) ← checkedProperty path.model source property
    if h : predicateAt path.model right predicate path.last = false then
      pure (.counterexample ⟨path, right, predicate, h⟩)
    else throw "safety: reachable path does not falsify the predicate"
  | _ => throw "safety: unsupported claim"

end NMLT.Artifact.InvariantWitness
