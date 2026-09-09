import NMLT.Artifact.ExecutionWitness

open Lean NMLT.Artifact.BehaviorCore NMLT.Artifact.SemanticClosure
open NMLT.Artifact.ExecutionClosure NMLT.Artifact.ExecutionWitness
open NMLT.Behavior.ResourceBehavior

private def ownerVectors : Nat → List (List (Option Bool))
  | 0 => [[]]
  | n + 1 => [none, some false, some true].flatMap fun owner =>
      (ownerVectors n).map (owner :: ·)

private def stateKey (m : Model) (s : m.State) : String :=
  let caps := capabilityNames m.program
  let entries := (List.finRange (caps.length + 1)).map fun c =>
    let name := (nameAt? caps c).getD "sentinel"
    let actor := match s.authority.owner c with
      | none => "vacant"
      | some false => m.left.name
      | some true => m.right.name
    s!"{name}={actor}"
  s!"{s.control.1.val}|{s.control.2.val}|{String.intercalate "," entries}"

private def outputKey (m : Model) (s : m.State) : IO String := do
  if s.control.1.val ≥ (stateSpace m.program m.left).length ||
      s.control.2.val ≥ (stateSpace m.program m.right).length then
    throw (IO.userError "a control sentinel was reachable")
  if (s.authority.owner (Fin.last (capabilityNames m.program).length)).isSome then
    throw (IO.userError "sentinel authority was fabricated")
  let key := stateKey m s
  pure ((key.splitOn ",sentinel=").head!)

private def actionKey (m : Model) : m.Act → String
  | .left a => s!"{m.left.name}.{(nameAt? m.leftActions a).getD "sentinel"}"
  | .right a => s!"{m.right.name}.{(nameAt? m.rightActions a).getD "sentinel"}"
  | .sync l r => s!"{m.left.name}.{(nameAt? m.leftActions l).getD "sentinel"}|{m.right.name}.{(nameAt? m.rightActions r).getD "sentinel"}"

private def run (m : Model) : IO Unit := do
  unless capabilityNames m.program == ["permit"] do
    throw (IO.userError "comparison requires the frozen one-capability corpus")
  unless decide m.formationCondition do throw (IO.userError "product is not formed")
  let ls := stateSpace m.program m.left
  let rs := stateSpace m.program m.right
  unless ls.length == 4 && rs.length == 2 do throw (IO.userError "finite control domain changed")
  let lefts := List.finRange (ls.length + 1)
  let rights := List.finRange (rs.length + 1)
  let worlds := ownerVectors ((capabilityNames m.program).length + 1)
  let states : List m.State := lefts.flatMap fun l => rights.flatMap fun r => worlds.map fun owners =>
    ⟨(l,r), ⟨fun c => (owners[c.val]?).getD none⟩⟩
  let la := List.finRange (m.leftActions.length + 1)
  let ra := List.finRange (m.rightActions.length + 1)
  let actions : List m.Act := la.map ProductAction.left ++ ra.map ProductAction.right ++
    la.flatMap (fun l => ra.map (ProductAction.sync l))
  let initial : List m.State := states.filter fun s => decide (m.behavior.init s)
  unless initial.length == 1 do throw (IO.userError "expected one initial state")
  let mut seen := initial.map (stateKey m)
  let mut reached : List m.State := initial
  let mut pending : List m.State := initial
  let mut edges : List String := []
  for _ in [:states.length] do
    match pending with
    | [] => pure ()
    | before :: rest =>
      pending := rest
      for action in actions do
        for after in states do
          if decide (m.behavior.step before action after) then
            let label := actionKey m action
            if label.endsWith "sentinel" || (label.splitOn "sentinel|").length > 1 then
              throw (IO.userError "sentinel action was enabled")
            edges := edges ++ [s!"T|{← outputKey m before}|{label}|{← outputKey m after}"]
            unless seen.contains (stateKey m after) do
              seen := seen ++ [stateKey m after]
              reached := reached ++ [after]
              pending := pending ++ [after]
  unless pending.isEmpty do throw (IO.userError "comparison traversal was truncated")
  for s in initial do IO.println s!"I|{← outputKey m s}"
  for s in reached do IO.println s!"S|{← outputKey m s}"
  for edge in edges do IO.println edge

def main (args : List String) : IO UInt32 := do
  let [path, target] := args | throw (IO.userError "expected artifact and target")
  let input ← IO.FS.readFile path
  let result := do decodeModel (← Json.parse input) target
  match result with
  | .error e => throw (IO.userError e)
  | .ok m => run m; pure 0
