import NMLT.Artifact.SemanticClosure

open NMLT.Artifact.BehaviorCore
open NMLT.Artifact.SemanticClosure

private def usage : String :=
  "usage: nmlt-artifact-check <behavior-core-v1.json> <source.nmlt>"

private def fail (message : String) : IO UInt32 := do
  IO.eprintln s!"artifact rejected: {message}"
  pure 1

def main (arguments : List String) : IO UInt32 := do
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
                      s!"{application.peerStates} finite states)")
                  pure 0
  | _ => fail usage
