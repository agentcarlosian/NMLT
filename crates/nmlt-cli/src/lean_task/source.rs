use super::{Manifest, Result, ResultRecord, Target, name};

pub(super) fn target(manifest: &Manifest) -> String {
    format!(
        r#"import {module}
import Lean.AddDecl
import Lean.Meta.InferType
set_option autoImplicit false
set_option maxHeartbeats 200000
open Lean Meta
run_meta do
  let ci ← getConstInfo `{declaration}
  unless (← isProp ci.type) do throwError "target declaration must have a proposition as its type"
  let env ← getEnv
  let some idx := env.getModuleIdxFor? ci.name | throwError "target is not an imported declaration"
  let moduleName := env.header.moduleNames[idx.toNat]!
  unless moduleName == `{module} do throwError "target belongs to another module"
  let pretty ← withOptions (fun o => o.setBool `pp.fullNames true |>.setBool `pp.universes true) do
    return (← ppExpr ci.type).pretty
  let metadata := Json.mkObj [
    ("declaration", toJson ci.name.toString),
    ("module", toJson moduleName.toString),
    ("type_repr", toJson (reprStr ci.type)),
    ("type_lean", toJson pretty),
    ("level_params", toJson (ci.levelParams.map toString)),
    ("type_references", toJson ((ci.type.getUsedConstants.toList.map toString).mergeSort (· ≤ ·)))]
  addDecl (.defnDecl {{
    name := `NMLTTask.target
    levelParams := ci.levelParams
    type := mkSort .zero
    value := ci.type
    hints := .abbrev
    safety := .safe }})
  IO.println ("NMLT_TASK=" ++ metadata.compress)
"#,
        module = manifest.target_module,
        declaration = manifest.target
    )
}

pub(super) fn proof(target: &Target, term: &str) -> Result<String> {
    if target
        .level_params
        .iter()
        .any(|p| !name(p) || p.contains('.'))
    {
        return Err("unsupported universe parameter name".into());
    }
    let universes = if target.level_params.is_empty() {
        String::new()
    } else {
        format!(".{{{}}}", target.level_params.join(", "))
    };
    let params = target
        .level_params
        .iter()
        .map(|p| format!("`{p}"))
        .collect::<Vec<_>>()
        .join(", ");
    let levels = target
        .level_params
        .iter()
        .map(|p| format!("Level.param `{p}"))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(
        r#"import NMLTTask
set_option autoImplicit false
set_option maxHeartbeats 200000
theorem NMLTChecked.result{universes} : NMLTTask.target{universes} := {term}

open Lean Meta
run_meta do
  let env := (← getEnv).setExporting false
  let some ci := env.find? `NMLTChecked.result | throwError "missing proof declaration"
  unless ci.levelParams == [{params}] do throwError "proof universe parameters changed"
  unless ci.type == mkConst `NMLTTask.target [{levels}] do throwError "proof target identity changed"
  let some value := ci.value? (allowOpaque := true) | throwError "missing proof value"
  let axes ← collectAxioms `NMLTChecked.result
  let metadata := Json.mkObj [
    ("root", toJson "NMLTChecked.result"),
    ("axioms", toJson ((axes.toList.map toString).mergeSort (· ≤ ·))),
    ("proof_references", toJson ((value.getUsedConstants.toList.map toString).mergeSort (· ≤ ·)))]
  IO.println ("NMLT_PROOF=" ++ metadata.compress)
"#
    ))
}

pub(super) fn patch(target: &str, proof: &str) -> String {
    let mut result = String::new();
    for (name, text) in [("NMLTTask.lean", target), ("NMLTProof.lean", proof)] {
        result.push_str(&format!("diff --git a/{name} b/{name}\nnew file mode 100644\n--- /dev/null\n+++ b/{name}\n@@ -0,0 +1,{} @@\n", text.lines().count()));
        for line in text.lines() {
            result.push('+');
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

pub(super) fn explanation(record: &ResultRecord) -> String {
    format!(
        "# Independently checked Lean proof\n\nTask: `{}`. Target: `{}` in `{}`.\n\n```lean\n{}\n```\n\nThe task snapshot binds the original elaborated type, every declared source module, the Lean installation and the axiom policy. `NMLTTask.target` is constructed directly from that declaration's type in Lean; the candidate can supply only its proof.\n\nThe proof root is `{}`. Its direct formal references are `{}`. The independent export contains {} declarations; the full declaration list and dependency graph are in `result.json`. [Read the linked dependency report](proof-dependencies.md) or inspect `proof-dependencies.json` for separate type/value constants, recursor reductions, projection types, literal support and export groups. These describe exported syntax; the graph is not a separate proof check or a draft proof plan.\n\nPermitted axioms: `{:?}`. Actual transitive axioms: `{:?}`.\n\nExport SHA-256: `{}`. NanoDA checked {} declarations. The checked source files are under `build/`; `proof.patch` adds the target binding and proof modules to the same source project.\n\nFresh recheck reconstructs the saved modules and repeats Lean and NanoDA checking with the selected task hash and original tool identities. It needs no model session or working project. The trusted project code, host, target-binding implementation and exporter remain trusted; this does not establish human approval, statement faithfulness or novelty.\n",
        record.task_sha256,
        record.task.target.declaration,
        record.task.target.module,
        record.task.target.type_lean,
        record.proof.root,
        record.proof.proof_references.join("`, `"),
        record.exported_declarations.len(),
        record.task.manifest.permitted_axioms,
        record.proof.axioms,
        record.export_sha256,
        record.checked_declarations
    )
}
