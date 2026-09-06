use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nmlt_compile::{compile_behavior_single, compile_behavior_v2};
use nmlt_core::diagnostic::line_column;
use nmlt_core::{Diagnostic, ParsedFile, lex_source, parse_source};
use nmlt_eval::{ExploreConfig, explore};
use nmlt_ir::BehaviorCoreProgram;

const HELP: &str = "\
NMLT language frontend (pre-alpha)\n\n\
Usage:\n\
  nmlt check <file>                                      Check structural declarations\n\
  nmlt inspect <file>                                    List recognized systems\n\
  nmlt tokens <file>                                     Print the lossless token stream\n\
  nmlt typecheck <file>                                  Elaborate the finite behavior slice\n\
  nmlt elaborate <file> --emit-core <artifact.json>      Emit behavior-core-v1\n\
  nmlt elaborate <file> --core-version v2 --emit-core <artifact.json> Emit opt-in v2\n\
  nmlt explore --behavior <name> --max-states <n> <core.json> Explore a canonical artifact\n\
  nmlt trace --behavior <name> --actions <comma-separated labels> --emit-path <path.json> --max-states <n> <core.json> Emit a v2 witness\n\
  nmlt version                                           Print the frontend version\n\
  nmlt help                                              Show this help\n\n\
Lean defines NMLT's normative behavior semantics. Exploration is not verification.\n";

fn main() -> ExitCode {
    match run(env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
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
