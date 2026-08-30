import NMLT.Artifact.BehaviorCore

open NMLT.Artifact.BehaviorCore

private def usage : String :=
  "usage: nmlt-artifact-check <behavior-core-v1.json> <source.nmlt>"

private def fail (message : String) : IO UInt32 := do
  IO.eprintln s!"artifact rejected: {message}"
  pure 1

def main (arguments : List String) : IO UInt32 := do
  match arguments with
  | [artifactPath, sourcePath] =>
      let artifact ← IO.FS.readFile artifactPath
      match parse artifact with
      | .error message => fail message
      | .ok summary =>
          let hash ← IO.Process.output {
            cmd := "sha256sum"
            args := #[sourcePath]
          }
          if hash.exitCode != 0 then
            fail s!"could not hash source: {hash.stderr.trimAscii}"
          else
            let actual := (hash.stdout.splitOn " ").head?.getD ""
            if actual != summary.sourceSha256 then
              fail s!"stale source digest: artifact has {summary.sourceSha256}, source has {actual}"
            else
              IO.println (s!"accepted behavior-core-v1: {summary.systems} systems, " ++
                s!"{summary.compositions} compositions, {summary.refinements} refinements")
              pure 0
  | _ => fail usage
