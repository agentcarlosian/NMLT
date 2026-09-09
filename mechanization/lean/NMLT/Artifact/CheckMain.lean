import NMLT.Artifact.SemanticClosure
import NMLT.Artifact.ExecutionWitness
import NMLT.Artifact.ExecutionLift

open NMLT.Artifact.BehaviorCore
open NMLT.Artifact.SemanticClosure

private def usage : String :=
  "usage: nmlt-artifact-check <behavior-core-v1.json> <source.nmlt>"

private def fail (message : String) : IO UInt32 := do
  IO.eprintln s!"artifact rejected: {message}"
  pure 1

private def checkV1 (arguments : List String) : IO UInt32 := do
  match arguments with
  | [artifactPath, sourcePath] =>
      let artifact ← IO.FS.readFile artifactPath
      match parseProgram artifact with
      | .error message => fail message
      | .ok program =>
          let hash ← IO.Process.output {
            cmd := "sha256sum"
            args := #[sourcePath]
          }
          if hash.exitCode != 0 then
            fail s!"could not hash source: {hash.stderr.trimAscii}"
          else
            let actual := (hash.stdout.splitOn " ").head?.getD ""
            if actual != program.summary.sourceSha256 then
              fail s!"stale source digest: artifact has {program.summary.sourceSha256}, source has {actual}"
            else
              match close program with
              | .error message => fail message
              | .ok closure =>
                  IO.println (s!"accepted behavior-core-v1: {closure.program.systems} systems, " ++
                    s!"{closure.program.compositions} compositions, " ++
                    s!"{closure.program.refinements} refinements")
                  for application in closure.applications do
                    IO.println (s!"theorem application accepted: " ++
                      s!"{application.concrete} refines {application.abstract} " ++
                      s!"through {application.peer} " ++
                      s!"({application.concreteStates}/{application.abstractStates}/" ++
                      s!"{application.peerStates} finite states, " ++
                      s!"{application.declaredCapabilities} declared component capabilities)")
                    IO.println (s!"conditional dynamic step-lifting witness constructed: " ++
                      s!"step existence and reachability are not checked")
                  pure 0
  | _ => fail usage

private def hashFile (path : String) : IO String := do
  let result ← IO.Process.output { cmd := "sha256sum", args := #[path] }
  if result.exitCode != 0 then throw (IO.userError s!"could not hash '{path}'")
  pure ((result.stdout.splitOn " ").head?.getD "")

def main (arguments : List String) : IO UInt32 := do
  match arguments with
  | [artifactPath, sourcePath, witnessPath] =>
    let encoded ← IO.FS.readFile artifactPath
    let witnessText ← IO.FS.readFile witnessPath
    let result := do
      let core ← Lean.Json.parse encoded
      let witness ← Lean.Json.parse witnessText
      let certificate ← NMLT.Artifact.ExecutionWitness.close core witness
      let lifted ← NMLT.Artifact.ExecutionLift.checkApplications core witness
      let digest ← (← witness.getObjVal? "artifact_sha256").getStr?
      pure (certificate, digest, lifted)
    match result with
    | .error message => fail message
    | .ok (certificate, digest, lifted) =>
      if (← hashFile artifactPath) != digest then fail "stale execution artifact digest"
      else if (← hashFile sourcePath) != certificate.model.program.summary.sourceSha256 then
        fail "stale source digest"
      else
        IO.println s!"accepted behavior-core-v2 execution: {certificate.model.composition.name}; {certificate.actions.length} actual steps from the decoded initializer"
        IO.println "constructed: unified formation, finite path, reachability, and path ownership witnesses"
        IO.println s!"initial synchronized refinement applications: {lifted}"
        pure 0
  | _ => checkV1 arguments
