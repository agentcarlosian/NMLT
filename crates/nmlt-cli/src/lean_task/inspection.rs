//! Bounded declaration lookup in a freshly rebuilt, bound Lean environment.
use super::{Result, Run, Task, err, lean, name, parse_marker, retain_cli, write_json, write_new};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Query {
    prefix: String,
    limit: usize,
}
impl Query {
    pub fn new(prefix: &str, limit: &str) -> Result<Self> {
        let query = Self {
            prefix: prefix.into(),
            limit: limit.parse().map_err(err)?,
        };
        if !name(prefix.strip_suffix('.').unwrap_or(prefix)) || !(1..=16).contains(&query.limit) {
            return Err(
                "inspection needs a bounded declaration prefix and a limit from 1 to 16".into(),
            );
        }
        Ok(query)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Axiom,
    Definition,
    Theorem,
    Opaque,
    Quotient,
    Inductive,
    Constructor,
    Recursor,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    name: String,
    module: String,
    kind: Kind,
    type_lean: String,
    level_params: Vec<String>,
    axioms: Vec<String>,
    axioms_within_policy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    prefix: String,
    limit: usize,
    matched: usize,
    has_more: bool,
    declarations: Vec<Declaration>,
}
impl Context {
    fn validate(&self, query: &Query, permitted: &[String]) -> Result<()> {
        if self.prefix != query.prefix
            || self.limit != query.limit
            || self.declarations.len() != self.matched.min(query.limit)
            || self.has_more != (self.matched > query.limit)
            || self
                .declarations
                .windows(2)
                .any(|pair| pair[0].name >= pair[1].name)
        {
            return Err("inspection context differs from its bounded query".into());
        }
        for declaration in &self.declarations {
            if !declaration.name.starts_with(&query.prefix)
                || declaration.name.len() > 4096
                || declaration.module.is_empty()
                || declaration.module.len() > 4096
                || declaration.type_lean.is_empty()
                || declaration.type_lean.len() > 4096
                || declaration.level_params.len() > 16
                || declaration.axioms.len() > 64
                || declaration
                    .level_params
                    .iter()
                    .chain(&declaration.axioms)
                    .any(|name| name.is_empty() || name.len() > 4096)
                || declaration.axioms.windows(2).any(|pair| pair[0] >= pair[1])
                || declaration.axioms_within_policy
                    != declaration
                        .axioms
                        .iter()
                        .all(|axiom| permitted.contains(axiom))
            {
                return Err("invalid declaration metadata or axiom-policy annotation".into());
            }
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct Record {
    schema: &'static str,
    status: &'static str,
    assurance: &'static str,
    task_sha256: String,
    task: Task,
    query: Query,
    context: Context,
    process_policy: nmlt_runtime::process::Policy,
    stages: Vec<super::Stage>,
}

pub(super) fn run(
    task: Task,
    query: Query,
    executable: &Path,
    output: &Path,
    origin: &Path,
) -> Result<()> {
    let digest = task.digest()?;
    let toolchain = lean::Toolchain::open(executable).map_err(err)?;
    if *toolchain.identity() != task.lean {
        return Err("Lean installation differs from the bound task".into());
    }
    let mut run = Run::new(output, executable)?;
    task.prepare(origin, &mut run)?;
    if run.target(&task.manifest)? != task.target {
        return Err("Lean target identity differs from the bound task".into());
    }
    let source = source(&query, &task.manifest.permitted_axioms)?;
    let output = run.compile("NMLTTaskInspect", &source)?;
    let context: Context = parse_marker(&output.stdout, "NMLT_INSPECT=")?;
    context.validate(&query, &task.manifest.permitted_axioms)?;
    toolchain.verify_unchanged().map_err(err)?;
    if let Some(lake) = &task.lake {
        lake.verify_run(&run)?;
    }
    let record = Record {
        schema: "nmlt-lean-inspection-v1",
        status: "context_only",
        assurance: "none",
        task_sha256: digest.clone(),
        task,
        query,
        context,
        process_policy: run.policy.ok_or("missing inspection process policy")?,
        stages: run.stages,
    };
    retain_cli(&run.directory, &record.task.implementation_sha256)?;
    write_json(&run.directory.join("task.json"), &record.task)?;
    write_new(
        &run.directory.join("task.sha256"),
        format!("{digest}\n").as_bytes(),
    )?;
    write_new(
        &run.directory.join("context.md"),
        markdown(&record).as_bytes(),
    )?;
    write_json(&run.directory.join("inspection.json"), &record)?;
    println!(
        "context_only: {} declarations ({} matched)\ntask: {digest}\ninspection: {}",
        record.context.declarations.len(),
        record.context.matched,
        run.directory.join("inspection.json").display()
    );
    Ok(())
}

fn source(query: &Query, permitted: &[String]) -> Result<String> {
    let prefix = serde_json::to_string(&query.prefix).map_err(err)?;
    let permitted = serde_json::to_string(permitted).map_err(err)?;
    Ok(format!(
        r#"import NMLTTask
set_option maxHeartbeats 200000
open Lean Meta
run_meta do
  let env := (← getEnv).setExporting false
  let queryPrefix : String := {prefix}
  let limit : Nat := {limit}
  let permitted : List String := {permitted}
  let queryEntries := (env.constants.toList.filter fun (n, _) =>
    queryPrefix.isPrefixOf n.toString && (env.getModuleIdxFor? n).isSome).mergeSort
      (fun a b => a.1.toString ≤ b.1.toString)
  let mut declarations := #[]
  for (n, ci) in queryEntries.take limit do
    let some idx := env.getModuleIdxFor? n | throwError "declaration has no imported module"
    let moduleName := env.header.moduleNames[idx.toNat]!.toString
    let kind := match ci with
      | .axiomInfo _ => "axiom"
      | .defnInfo _ => "definition"
      | .thmInfo _ => "theorem"
      | .opaqueInfo _ => "opaque"
      | .quotInfo _ => "quotient"
      | .inductInfo _ => "inductive"
      | .ctorInfo _ => "constructor"
      | .recInfo _ => "recursor"
    let pretty ← withOptions (fun o => o.setBool `pp.fullNames true |>.setBool `pp.universes true) do
      return (← ppExpr ci.type).pretty
    let axes ← collectAxioms n
    let axes := (axes.toList.map toString).mergeSort (· ≤ ·)
    if pretty.utf8ByteSize > 4096 || ci.levelParams.length > 16 || axes.length > 64 then
      throwError "declaration inspection metadata exceeds its bound"
    declarations := declarations.push (Json.mkObj [
      ("name", toJson n.toString), ("module", toJson moduleName), ("kind", toJson kind),
      ("type_lean", toJson pretty), ("level_params", toJson (ci.levelParams.map toString)),
      ("axioms", toJson axes), ("axioms_within_policy", toJson (axes.all permitted.contains))])
  let result := Json.mkObj [
    ("prefix", toJson queryPrefix), ("limit", toJson limit), ("matched", toJson queryEntries.length),
    ("has_more", toJson (decide (queryEntries.length > limit))), ("declarations", toJson declarations)]
  if result.compress.utf8ByteSize > 49152 then
    throwError "declaration inspection exceeds 48 KiB"
  IO.println ("NMLT_INSPECT=" ++ result.compress)
"#,
        limit = query.limit
    ))
}

fn markdown(record: &Record) -> String {
    let mut result = format!(
        "# Lean declaration context\n\nTask: `{}`. Target: `{}`.\n\nContext only; this is not proof acceptance. The task's sources and target were rebuilt and compared with Lean. A matching prefix or permitted axiom set does not establish that a declaration proves the target. Submit a separate candidate to `lean-task prove` for full checking.\n\nPrefix: `{}`. Showing {} of {} matches.\n\n",
        record.task_sha256,
        record.task.target.declaration,
        record.query.prefix,
        record.context.declarations.len(),
        record.context.matched
    );
    for declaration in &record.context.declarations {
        // Escape HTML before rendering declaration metadata as preformatted text.
        result.push_str(&format!("## Declaration\n\n<pre>{}</pre>\n\n", escape(&format!("Name: {}\nModule: {}\nKind: {:?}\nType: {}\nAxioms: {:?}\nWithin selected axiom policy: {}",declaration.name,declaration.module,declaration.kind,declaration.type_lean,declaration.axioms,declaration.axioms_within_policy))));
    }
    result
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_is_a_bounded_prefix_query_not_a_lean_command() {
        assert!(Query::new("Example.", "16").is_ok());
        assert!(Query::new("Nat.zero_add", "1").is_ok());
        for prefix in [
            "",
            "../Other",
            "Example..",
            "Example\nrun_meta",
            "Example.\"",
            "by exact",
        ] {
            assert!(Query::new(prefix, "1").is_err());
        }
        for limit in ["0", "17", "-1", "all"] {
            assert!(Query::new("Example.", limit).is_err());
        }
    }

    #[test]
    fn returned_metadata_must_match_query_and_axiom_annotation() {
        let query = Query::new("Example.", "1").unwrap();
        let mut context = Context {
            prefix: query.prefix.clone(),
            limit: 1,
            matched: 2,
            has_more: true,
            declarations: vec![Declaration {
                name: "Example.goal".into(),
                module: "Example.Goals".into(),
                kind: Kind::Theorem,
                type_lean: "True".into(),
                level_params: vec![],
                axioms: vec!["sorryAx".into()],
                axioms_within_policy: false,
            }],
        };
        assert!(context.validate(&query, &[]).is_ok());
        context.declarations[0].axioms_within_policy = true;
        assert!(context.validate(&query, &[]).is_err());
        context.declarations[0].axioms_within_policy = false;
        context.has_more = false;
        assert!(context.validate(&query, &[]).is_err());
        assert_eq!(escape("x < y & z > w"), "x &lt; y &amp; z &gt; w");
    }
}
