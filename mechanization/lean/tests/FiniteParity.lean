import NMLT.Artifact.SemanticClosure

open NMLT.Artifact.BehaviorCore
open NMLT.Artifact.SemanticClosure

-- Test-only oracle for a closed, resource-free, bounded corpus. Transitions
-- below are decided by the existing Lean Behavior, not a second evaluator.
private def valueKey : Value → String
  | .bool false => "false"
  | .bool true => "true"
  | .unit => "()"
  | .enumeration name => name

private def stateKey (state : Valuation) : String :=
  String.intercalate "," (state.map fun (name, value) => s!"{name}={valueKey value}")

private def fail (message : String) : IO α := throw (IO.userError message)

private def checkFixture (system : System) : IO Unit := do
  unless system.capabilities.isEmpty && system.ports.isEmpty do
    fail "finite parity fixture must be closed and resource-free"
  for action in system.actions do
    let profile := action.resources
    unless action.direction == .internal && profile.requires.isEmpty &&
        profile.consumes.isEmpty && profile.transfers.isEmpty &&
        profile.receives.isEmpty && profile.grade.isEmpty &&
        profile.relies.isEmpty && profile.guarantees.isEmpty do
      fail "finite parity fixture contains an unsupported action/resource effect"

private def run (program : Program) (system : System) : IO Unit := do
  checkFixture system
  let states := stateSpace program system
  if states.length > 16 then fail "finite parity corpus exceeds its 16-state bound"
  let actions := system.actions.map Action.name
  let behavior := toBehavior program system actions []
  -- Include the decoder sentinel in the checks: it must never be initial or
  -- reachable. It does not count as a well-typed value-domain state.
  let indices := List.finRange (states.length + 1)
  let actionIndices := List.finRange (actions.length + 1)
  let initial : List (Fin (states.length + 1)) :=
    indices.filter fun state => decide (behavior.init state)
  unless initial.length == 1 do fail "expected one valid initial state"
  let mut reachable : List (Fin (states.length + 1)) := initial
  let mut pending : List (Fin (states.length + 1)) := initial
  let mut transitions : List String := []
  for _ in [:states.length + 1] do
    match pending with
    | [] => pure ()
    | before :: rest =>
        pending := rest
        let some beforeValues := stateAt? program system before |
          fail "sentinel was reachable"
        for action in actionIndices do
          for after in indices do
            if decide (behavior.step before action after) then
              let some actionName := nameAt? actions action |
                fail "sentinel action was enabled"
              let some afterValues := stateAt? program system after |
                fail "sentinel successor was reachable"
              transitions := transitions ++
                [s!"T|{stateKey beforeValues}|{system.name}.{actionName}|{stateKey afterValues}"]
              unless reachable.contains after do
                reachable := reachable ++ [after]
                pending := pending ++ [after]
  unless pending.isEmpty do fail "finite parity traversal was truncated"
  for index in initial do
    let some values := stateAt? program system index | fail "invalid initial state"
    IO.println s!"I|{stateKey values}"
  for index in reachable do
    let some values := stateAt? program system index | fail "invalid reachable state"
    IO.println s!"S|{stateKey values}"
  for transition in transitions do IO.println transition

def main (arguments : List String) : IO UInt32 := do
  match arguments with
  | [path, name] =>
      let input ← IO.FS.readFile path
      match parseProgram input with
      | .error message => fail message
      | .ok program =>
          let some system := program.systems.find? fun system => system.name == name |
            fail s!"unknown system {name}"
          run program system
          pure 0
  | _ => fail "usage: FiniteParity.lean <artifact> <system>"
