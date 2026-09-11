use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nmlt_compile::{compile_behavior_single, compile_behavior_v2};
use nmlt_core::diagnostic::line_column;
use nmlt_core::{Diagnostic, ParsedFile, lex_source, parse_source};
use nmlt_eval::{ExploreConfig, explore};
use nmlt_ir::BehaviorCoreProgram;

mod async_jobs;
mod diagnostics;
mod formatting;
mod invariant;
mod job_process;
mod jobs;
mod lean_task;
mod project;
mod runtime;
mod strict_json;
mod workflow;

const HELP: &str = "\
NMLT language frontend (pre-alpha)\n\n\
Usage:\n\
  nmlt lean-task <bind|inspect|prove|recheck> --help     Inspect bound tasks and independently check proofs\n\
  nmlt init <new-directory>                             Create a runnable project and tests\n\
  nmlt lock [project-directory]                        Pin imported sources and local tools\n\
  nmlt check-project [project-directory]               Check project types, inputs, and lock\n\
  nmlt check-invariant <file> --behavior <name> --property <System.Name> --max-states <n> --checker <path> --emit-evidence <new-dir> Check finite safety in Lean\n\
  nmlt run <project-directory> [--arg name=value]       Run with project inputs and budgets\n\
  nmlt test [project-directory]                        Run declared tests with real adapters\n\
  nmlt fmt [project-directory|source.nmlt] [--check]     Format workflow source/imports\n\
  nmlt replay <project.json> --project <directory>      Replay retained project sources\n\
  nmlt resume <saved-run-directory> --project <directory> [--acknowledge-uncertain-effects <reason>] Resume saved project work\n\
  nmlt check <file>                                      Check structural declarations\n\
  nmlt inspect <file>                                    List recognized systems\n\
  nmlt tokens <file>                                     Print the lossless token stream\n\
  nmlt typecheck <file>                                  Elaborate the finite behavior slice\n\
  nmlt typecheck <file> --profile workflow               Check workflow functions and effects\n\
  nmlt elaborate <file> --emit-core <artifact.json>      Emit behavior-core-v1\n\
  nmlt elaborate <file> --core-version v2 --emit-core <artifact.json> Emit opt-in v2\n\
  nmlt explore --behavior <name> --max-states <n> <core.json> Explore a canonical artifact\n\
  nmlt trace --behavior <name> --actions <comma-separated labels> --emit-path <path.json> --max-states <n> <core.json> Emit a v2 witness\n\
  nmlt run <source.nmlt> --behavior <name> --max-steps <n> --emit-run <new.json> [--actions <labels>] Execute finite v2\n\
  nmlt run <source.nmlt> --entry <name> --max-steps <n> --emit-run <new.json> [--arg name=value] Execute pure functions\n\
  nmlt run <source.nmlt> --entry <name> ... --jobs-dir <new-dir> --max-jobs <1..16> --job-timeout-ms <1..30000> Execute local jobs\n\
  nmlt jobs-recover <jobs-dir>                            Classify unfinished work without redispatch\n\
  nmlt jobs-resume <jobs-dir> --emit-run <new.json> [--lean-bin <path>] [--acknowledge-uncertain-effects <reason>] Resume saved source\n\
  nmlt jobs-repair <jobs-dir> --acknowledge-incomplete-tail <reason> Quarantine incomplete final appends\n\
  nmlt run <source.nmlt> --entry <name> ... --job-slots <1..4> [--lean-bin <path>] Execute scoped async jobs\n\
  nmlt replay <record.json> --source <source.nmlt>         Replay with the same executable\n\
  nmlt version                                           Print the frontend version\n\
  nmlt help                                              Show this help\n\n\
Prefix a command with --json for structured error diagnostics.\n\
Lean defines NMLT's normative behavior semantics. Exploration is not verification.\n";

fn main() -> ExitCode {
    let mut arguments: Vec<_> = env::args_os().skip(1).collect();
    let json = arguments.first().is_some_and(|a| a == "--json");
    if json {
        arguments.remove(0);
    }
    match dispatch(arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if json {
                eprintln!(
                    "{}",
                    serde_json::to_string(&error).expect("diagnostic JSON")
                );
            } else {
                eprintln!("{error}");
            }
            ExitCode::FAILURE
        }
    }
}

fn dispatch(arguments: Vec<std::ffi::OsString>) -> Result<(), diagnostics::Error> {
    let command = arguments.first().and_then(|a| a.to_str()).unwrap_or("help");
    match command {
        "lean-task" => return lean_task::command(&arguments[1..]).map_err(Into::into),
        "check-invariant" => return invariant::command(&arguments[1..]),
        "resume" => return project::resume(&arguments[1..]),
        "init" | "lock" | "check-project" | "test" => {
            return project::command(command, &arguments[1..]);
        }
        "fmt" => return formatting::command(&arguments[1..]),
        "run"
            if arguments.get(1).is_some_and(|p| Path::new(p).is_dir())
                || (arguments.len() == 1 && Path::new("nmlt.toml").is_file()) =>
        {
            return project::command("run", &arguments[1..]);
        }
        "replay" if arguments.get(2).is_some_and(|f| f == "--project") => {
            return project::replay(&arguments[1..]);
        }
        "typecheck"
            if arguments.len() == 4
                && arguments[2] == "--profile"
                && arguments[3] == "workflow" =>
        {
            workflow::load_diagnostic(Path::new(&arguments[1]), None)?;
        }
        "run" if arguments.get(1).is_some() && arguments.iter().any(|a| a == "--entry") => {
            workflow::load_diagnostic(Path::new(&arguments[1]), None)?;
        }
        _ => {}
    }
    run(arguments).map_err(Into::into)
}

fn run(arguments: Vec<std::ffi::OsString>) -> Result<(), String> {
    let Some(command) = arguments.first().and_then(|argument| argument.to_str()) else {
        print!("{HELP}");
        return Ok(());
    };
    match command {
        "help" | "--help" | "-h" => {
            print!("{HELP}");
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("nmlt {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "jobs-resume" => async_jobs::resume(&arguments[1..]),
        "jobs-repair" => async_jobs::repair(&arguments[1..]),
        "check" | "inspect" => {
            let path = single_path_argument(command, &arguments[1..])?;
            let parsed = load_and_parse(&path)?;
            if command == "check" {
                print_check(&path, &parsed);
            } else {
                print_inspect(&path, &parsed);
            }
            Ok(())
        }
        "tokens" => {
            let path = single_path_argument(command, &arguments[1..])?;
            print_tokens(&path)
        }
        "typecheck" => {
            if arguments.len() == 4 && arguments[2] == "--profile" && arguments[3] == "workflow" {
                return workflow::typecheck(Path::new(&arguments[1]));
            }
            let path = single_path_argument(command, &arguments[1..])?;
            let artifact = compile_path(&path)?;
            println!(
                "type_checked: {} ({} systems, {} compositions, {} refinements)",
                path.display(),
                artifact.systems.len(),
                artifact.compositions.len(),
                artifact.refinements.len()
            );
            println!("semantic_authority: Lean not invoked; Rust frontend acceptance only");
            Ok(())
        }
        "elaborate" => {
            let (source, output, dynamic) = elaborate_arguments(&arguments[1..])?;
            let artifact = if dynamic {
                let bytes = fs::read(&source).map_err(|e| e.to_string())?;
                compile_behavior_v2(repository_path(&source)?, bytes).map_err(|e| e.to_string())?
            } else {
                compile_path(&source)?
            };
            fs::write(&output, artifact.to_json_pretty())
                .map_err(|error| format!("could not write '{}': {error}", output.display()))?;
            println!(
                "elaborated: {} -> {} ({})",
                source.display(),
                output.display(),
                artifact.schema
            );
            println!("semantic_authority: requires separate Lean artifact checking");
            Ok(())
        }
        "explore" => {
            let (behavior, max_states, path) = explore_arguments(&arguments[1..])?;
            let encoded = fs::read_to_string(&path)
                .map_err(|error| format!("could not read '{}': {error}", path.display()))?;
            let artifact = BehaviorCoreProgram::from_canonical_json(&encoded)
                .or_else(|_| BehaviorCoreProgram::from_canonical_json_v2(&encoded))?;
            let result = explore(&artifact, &behavior, ExploreConfig { max_states })
                .map_err(|error| error.to_string())?;
            println!("behavior: {}", result.behavior);
            println!("assurance: none (reference exploration only)");
            println!("states: {}", result.states.len());
            println!("transitions: {}", result.transitions.len());
            println!("truncated: {}", result.truncated);
            for (index, state) in result.states.iter().enumerate() {
                let values = state
                    .values
                    .iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let authority = state
                    .authority
                    .iter()
                    .map(|(capability, owner)| format!("{capability}={owner}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("state {index}: {values} authority=[{authority}]");
            }
            for transition in &result.transitions {
                let grade = transition
                    .grade
                    .iter()
                    .map(|(atom, value)| format!("{atom}={value}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                println!(
                    "step {} -> {}: {} grade=[{}] transfers=[{}]",
                    transition.from,
                    transition.to,
                    transition.label,
                    grade,
                    transition.transfers.join(", ")
                );
            }
            Ok(())
        }
        "trace" => emit_trace(&arguments[1..]),
        "__square-worker" if arguments.len() == 1 => jobs::worker(),
        "jobs-recover" => {
            let directory = single_path_argument(command, &arguments[1..])?;
            if directory.join("source-context.json").exists() {
                async_jobs::recover(&directory)
            } else {
                jobs::recover(&directory)
            }
        }
        "run"
            if arguments[2.min(arguments.len())..]
                .chunks_exact(2)
                .any(|pair| pair[0] == "--entry") =>
        {
            workflow::run(&arguments[1..])
        }
        "run" => runtime::run(&arguments[1..]),
        "replay" => workflow::replay_or_finite(&arguments[1..]),
        unknown => Err(format!("unknown command '{unknown}'\n\n{HELP}")),
    }
}

fn compile_path(path: &Path) -> Result<BehaviorCoreProgram, String> {
    let source =
        fs::read(path).map_err(|error| format!("could not read '{}': {error}", path.display()))?;
    compile_behavior_single(repository_path(path)?, source).map_err(|error| error.to_string())
}

fn repository_path(path: &Path) -> Result<String, String> {
    let canonical = path.canonicalize().map_err(|error| error.to_string())?;
    let root = env::current_dir()
        .map_err(|error| error.to_string())?
        .canonicalize()
        .map_err(|error| error.to_string())?;
    Ok(canonical.strip_prefix(root).map_or_else(
        |_| {
            format!(
                "external/{}",
                canonical
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("input.nmlt")
            )
        },
        |relative| relative.to_string_lossy().replace('\\', "/"),
    ))
}

fn elaborate_arguments(
    arguments: &[std::ffi::OsString],
) -> Result<(PathBuf, PathBuf, bool), String> {
    match arguments {
        [source, flag, output] if flag == "--emit-core" => {
            Ok((PathBuf::from(source), PathBuf::from(output), false))
        }
        [source, version_flag, version, flag, output]
            if version_flag == "--core-version" && version == "v2" && flag == "--emit-core" =>
        {
            Ok((PathBuf::from(source), PathBuf::from(output), true))
        }
        _ => Err("usage: nmlt elaborate <file> --emit-core <artifact.json>".to_owned()),
    }
}

fn emit_trace(arguments: &[std::ffi::OsString]) -> Result<(), String> {
    let [bf, behavior, af, actions, of, output, mf, max, path] = arguments else {
        return Err("usage: nmlt trace --behavior <name> --actions <labels> --emit-path <path.json> --max-states <n> <core.json>".into());
    };
    if bf != "--behavior" || af != "--actions" || of != "--emit-path" || mf != "--max-states" {
        return Err("invalid trace options".into());
    }
    let behavior = behavior.to_str().ok_or("behavior is not UTF-8")?;
    let action_text = actions.to_str().ok_or("actions are not UTF-8")?;
    if !action_text.is_empty() && action_text.split(',').any(str::is_empty) {
        return Err("empty action label between separators".into());
    }
    let labels: Vec<_> = action_text
        .split(',')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let max_states = max
        .to_str()
        .ok_or("max-states is not UTF-8")?
        .parse()
        .map_err(|_| "invalid max-states")?;
    let encoded = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let artifact = BehaviorCoreProgram::from_canonical_json_v2(&encoded)?;
    let graph =
        explore(&artifact, behavior, ExploreConfig { max_states }).map_err(|e| e.to_string())?;
    let digest = nmlt_hir::sha256_bytes(encoded.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let witness =
        nmlt_eval::execution_path(&artifact, &graph, digest, &labels).map_err(|e| e.to_string())?;
    fs::write(output, witness.to_json_pretty()?).map_err(|e| e.to_string())?;
    println!(
        "witness emitted: {} steps; assurance: none; requires separate Lean execution checking",
        witness.actions.len()
    );
    Ok(())
}

fn explore_arguments(arguments: &[std::ffi::OsString]) -> Result<(String, usize, PathBuf), String> {
    match arguments {
        [behavior_flag, behavior, max_flag, max_states, path]
            if behavior_flag == "--behavior" && max_flag == "--max-states" =>
        {
            let behavior = behavior
                .to_str()
                .ok_or_else(|| "behavior name is not UTF-8".to_owned())?
                .to_owned();
            let max_states = max_states
                .to_str()
                .ok_or_else(|| "max-states is not UTF-8".to_owned())?
                .parse::<usize>()
                .map_err(|_| "max-states must be a positive integer".to_owned())?;
            Ok((behavior, max_states, PathBuf::from(path)))
        }
        _ => Err("usage: nmlt explore --behavior <name> --max-states <n> <core.json>".to_owned()),
    }
}

fn single_path_argument(
    command: &str,
    arguments: &[std::ffi::OsString],
) -> Result<PathBuf, String> {
    match arguments {
        [path] => Ok(PathBuf::from(path)),
        [] => Err(format!("'{command}' requires one source path")),
        _ => Err(format!("'{command}' accepts exactly one source path")),
    }
}

fn load_and_parse(path: &Path) -> Result<ParsedFile, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("could not read '{}': {error}", path.display()))?;
    match parse_source(&source) {
        Ok(parsed) => Ok(parsed),
        Err(diagnostics) => Err(render_diagnostics(path, &source, &diagnostics)),
    }
}

fn print_tokens(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("could not read '{}': {error}", path.display()))?;
    let lexed = lex_source(&source);
    if !lexed.diagnostics.is_empty() {
        return Err(render_diagnostics(path, &source, &lexed.diagnostics));
    }
    for token in lexed.tokens {
        println!(
            "{}..{}\t{}\t{}",
            token.span.start,
            token.span.end,
            token.kind,
            token.text(&source).escape_debug()
        );
    }
    Ok(())
}

fn render_diagnostics(path: &Path, source: &str, diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            if let Some(span) = diagnostic.span {
                let (line, column) = line_column(source, span.start);
                format!(
                    "{}:{line}:{column}: {}[{}]: {}",
                    path.display(),
                    diagnostic.severity,
                    diagnostic.code,
                    diagnostic.message
                )
            } else {
                format!(
                    "{}: {}[{}]: {}",
                    path.display(),
                    diagnostic.severity,
                    diagnostic.code,
                    diagnostic.message
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn print_check(path: &Path, parsed: &ParsedFile) {
    println!(
        "ok: {} ({} system declaration{})",
        path.display(),
        parsed.systems.len(),
        if parsed.systems.len() == 1 { "" } else { "s" }
    );
    println!("note: structural parsing only; run typecheck for behavior elaboration");
}

fn print_inspect(path: &Path, parsed: &ParsedFile) {
    println!("file: {}", path.display());
    for system in &parsed.systems {
        println!("system: {}", system.name);
    }
    println!("assurance: structural inspection only");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_verifier_commands_are_unknown() {
        for command in ["model-check", "evidence"] {
            let error = run(vec![command.into()]).unwrap_err();
            assert!(error.starts_with("unknown command"));
        }
    }
}
