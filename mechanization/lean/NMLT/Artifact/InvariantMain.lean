import NMLT.Artifact.InvariantWitness

open Lean (Json toJson)
open NMLT.Artifact.InvariantWitness NMLT.Artifact.SafetyPredicate

private def readBounded (path : String) : IO String := do
  let metadata ← (System.FilePath.mk path).metadata
  if metadata.byteSize > 4194304 then throw (IO.userError "safety input exceeds 4 MiB")
  IO.FS.readFile path

private def hashFile (path : String) : IO String := do
  -- GNU sha256sum escapes paths containing backslashes. Forward slashes work
  -- on both the native Windows checker and POSIX hosts.
  let result ← IO.Process.output { cmd := "sha256sum", args := #[path.replace "\\" "/"] }
  if result.exitCode != 0 then throw (IO.userError "could not hash safety input")
  let digest := (result.stdout.splitOn " ").head?.getD ""
  unless digest.length == 64 && digest.toList.all (fun c => c.isDigit || ('a' ≤ c && c ≤ 'f')) do
    throw (IO.userError "invalid SHA-256 output")
  pure digest

def main (arguments : List String) : IO UInt32 := do
  try
    let [corePath, sourcePath, artifactPath, witnessPath] := arguments |
      throw (IO.userError "usage: nmlt-invariant-check <core.json> <source.nmlt> <invariant.json> <witness.json>")
    let core ← readBounded corePath
    let source ← readBounded sourcePath
    let artifact ← readBounded artifactPath
    let witness ← readBounded witnessPath
    let coreHash ← hashFile corePath
    let sourceHash ← hashFile sourcePath
    let artifactHash ← hashFile artifactPath
    let witnessHash ← hashFile witnessPath
    let result := do
      let core ← Json.parse core
      let artifact ← Json.parse artifact
      let witness ← Json.parse witness
      unless (← stringAt artifact "core_sha256") == coreHash &&
          (← stringAt artifact "source_sha256") == sourceHash &&
          (← stringAt witness "invariant_sha256") == artifactHash do
        throw "safety: stale source/core/predicate digest"
      let checked ← close core artifact witness source
      pure (artifact, checked)
    match result with
    | .error message => IO.eprintln message; pure 1
    | .ok (artifact, checked) =>
      let (verdict, states, steps) := match checked with
        | .invariant c => ("invariant", c.reached.length, 0)
        | .counterexample c => ("counterexample", c.path.actions.length + 1, c.path.actions.length)
      let report := Json.mkObj [
        ("schema", .str "nmlt-safety-check-v1"),
        ("verdict", .str verdict),
        ("behavior", (artifact.getObjVal? "behavior").toOption.getD .null),
        ("certificate_states", toJson states),
        ("path_steps", toJson steps),
        ("core_sha256", .str coreHash),
        ("source_sha256", .str sourceHash),
        ("invariant_sha256", .str artifactHash),
        ("witness_sha256", .str witnessHash),
        ("scope", .str "decoded finite binary model; no verified source translation or host correspondence")]
      IO.println report.compress
      pure 0
  catch error =>
    IO.eprintln s!"safety rejected: {error}"
    pure 1
