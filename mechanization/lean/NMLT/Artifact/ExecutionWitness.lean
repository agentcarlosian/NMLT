import NMLT.Artifact.ExecutionClosure

namespace NMLT.Artifact.ExecutionWitness

open Lean NMLT.Artifact.BehaviorCore NMLT.Artifact.SemanticClosure
open NMLT.Artifact.ExecutionClosure NMLT.Behavior.ResourceBehavior
open NMLT.Behavior.ResourceWorld NMLT.Behavior.ResourceDynamics

private def fail (message : String) : Except String α := throw s!"execution: {message}"

private def keys (json : Json) (expected : List String) : Except String Unit := do
  let actual := (← json.getObj?).toList.map Prod.fst
  unless actual.length == expected.length && actual.all expected.contains do
    fail s!"unexpected object fields: {actual}"

private def str (json : Json) (key : String) : Except String String := do
  (← json.getObjVal? key).getStr?

private def owner (left right : String) : Json → Except String (Option Bool)
  | .null => pure none
  | .str value =>
      if value == left then pure (some false)
      else if value == right then pure (some true)
      else fail s!"unknown owner '{value}' for selected composition"
  | _ => fail "owner must be a selected leaf name or null"

def decodeModel (core : Json) (target : String) : Except String Model := do
  keys core ["schema", "source_path", "source_sha256", "enums", "systems",
    "compositions", "refinements", "known_capabilities", "initial_authority"]
  let program ← decodeProgramV2 core
  let some composition := program.compositions.find? (fun c => c.name == target) |
    fail s!"unknown binary composition '{target}'"
  let some left := program.systems.find? (fun s => s.name == composition.left) | fail "missing left leaf"
  let some right := program.systems.find? (fun s => s.name == composition.right) | fail "missing right leaf"
  let initial ← (← (← core.getObjVal? "initial_authority").getObjVal? target).getObj?
  let initialOwners ← initial.toList.mapM fun (cap, value) => do
    pure (cap, ← owner left.name right.name value)
  pure { program, composition, left, right, initialOwners }

def decodeState (m : Model) (json : Json) : Except String m.State := do
  keys json ["left", "right", "authority"]
  let l ← (← json.getObjVal? "left").getNat?
  let r ← (← json.getObjVal? "right").getNat?
  let authority ← json.getObjVal? "authority"
  keys authority (capabilityNames m.program)
  let owners ← (← authority.getObj?).toList.mapM fun (cap, value) => do
    pure (cap, ← owner m.left.name m.right.name value)
  if hl : l < (stateSpace m.program m.left).length then
    if hr : r < (stateSpace m.program m.right).length then
      pure {
        control := (⟨l, Nat.lt_succ_of_lt hl⟩, ⟨r, Nat.lt_succ_of_lt hr⟩)
        authority := { owner := fun c =>
          (nameAt? (capabilityNames m.program) c).bind fun name => (owners.lookup name).getD none }
      }
    else fail "right control index is outside the finite domain"
  else fail "left control index is outside the finite domain"

private def index (names : List String) (value : String) : Except String (Fin (names.length + 1)) := do
  let i := names.findIdx (· == value)
  if h : i < names.length then pure ⟨i, Nat.lt_succ_of_lt h⟩
  else fail s!"unknown action '{value}'"

def decodeAction (m : Model) (json : Json) : Except String m.Act := do
  keys json ["left", "right"]
  match ← json.getObjVal? "left", ← json.getObjVal? "right" with
  | .str left, .null => pure (.left (← index m.leftActions left))
  | .null, .str right => pure (.right (← index m.rightActions right))
  | .str left, .str right => pure (.sync (← index m.leftActions left) (← index m.rightActions right))
  | _, _ => fail "an action needs one or two named endpoints"

structure CheckedTail (m : Model) (before : m.State) where
  actions : List m.Act
  last : m.State
  path : Path m.behavior before actions last

private def checkTail (m : Model) (before : m.State) :
    List Json → List Json → Except String (CheckedTail m before)
  | [], [] => pure ⟨[], before, Path.nil before⟩
  | a :: actions, s :: states => do
      let action ← decodeAction m a
      let middle ← decodeState m s
      if h : m.behavior.step before action middle then
        let tail ← checkTail m middle actions states
        pure ⟨action :: tail.actions, tail.last, Path.cons h tail.path⟩
      else fail s!"claimed transition is not an actual unified step ({actions.length} later actions)"
  | _, _ => fail "a path needs exactly one more state than action"

structure Certificate where
  model : Model
  initial : model.State
  last : model.State
  actions : List model.Act
  formation : Composable model.leftBehavior model.rightBehavior model.connected
  initialized : model.behavior.init initial
  path : Path model.behavior initial actions last

def close (core witness : Json) : Except String Certificate := do
  keys witness ["schema", "artifact_sha256", "behavior", "states", "actions"]
  unless (← str witness "schema") == "behavior-execution-v1" do fail "unsupported witness schema"
  let digest ← str witness "artifact_sha256"
  unless digest.length == 64 && digest.toList.all (fun c => c.isDigit || ('a' ≤ c && c ≤ 'f')) do
    fail "invalid artifact SHA-256"
  let model ← decodeModel core (← str witness "behavior")
  let states ← (← witness.getObjVal? "states").getArr?
  let actions ← (← witness.getObjVal? "actions").getArr?
  let first :: rest := states.toList | fail "path has no initial state"
  let initial ← decodeState model first
  if hf : model.formationCondition then
    if hi : model.behavior.init initial then
      let tail ← checkTail model initial actions.toList rest
      pure ⟨model, initial, tail.last, tail.actions, model.formed hf, hi, tail.path⟩
    else fail "claimed first state does not satisfy the unified initializer"
  else fail "selected product does not satisfy formation"

theorem Certificate.reachable (c : Certificate) : Reachable c.model.behavior c.last :=
  ⟨c.initial, c.actions, c.initialized, c.path⟩

theorem Certificate.no_fabrication (c : Certificate)
    {capability : c.model.Cap}
    (vacant : c.initial.authority.Vacant capability) : c.last.authority.Vacant capability :=
  c.path.vacant_preserved vacant

theorem Certificate.unique_owner (c : Certificate)
    {capability : c.model.Cap} {a b : Bool}
    (left : c.last.authority.Owns a capability) (right : c.last.authority.Owns b capability) : a = b :=
  c.last.authority.ownership_unique left right

theorem Certificate.owner_origin (c : Certificate) {actor : Bool} {capability : c.model.Cap}
    (owned : c.last.authority.Owns actor capability) :
    c.initial.authority.Owns actor capability ∨
      ∃ a, a ∈ c.actions ∧ c.model.TransferTo a capability actor :=
  c.model.path_owned_origin c.path owned

end NMLT.Artifact.ExecutionWitness
