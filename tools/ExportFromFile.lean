-- Run the unchanged, pinned lean4export entry point without a large OS argv.
-- Main is provided by the exporter checkout, not the NMLT Lean package.
import Main

def exportFromFile : IO Unit := do
  let some constantsFile ← IO.getEnv "NMLT_EXPORT_CONSTANTS"
    | throw <| IO.userError "missing NMLT_EXPORT_CONSTANTS"
  let some moduleName ← IO.getEnv "NMLT_EXPORT_MODULE"
    | throw <| IO.userError "missing NMLT_EXPORT_MODULE"
  let constants ← IO.FS.lines constantsFile
  let some outputFile ← IO.getEnv "NMLT_EXPORT_OUTPUT"
    | throw <| IO.userError "missing NMLT_EXPORT_OUTPUT"
  -- Avoid the elaborator's #eval message buffer for large environments.
  IO.FS.withFile outputFile .write fun handle =>
    IO.withStdout (.ofHandle handle) do
      main ([moduleName, "--"] ++ constants.toList)
      handle.flush

#eval exportFromFile
