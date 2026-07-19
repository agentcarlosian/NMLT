use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nmlt_compile::compile_single;
use nmlt_core::diagnostic::line_column;
use nmlt_core::{Diagnostic, EvidenceManifest, ParsedFile, lex_source, parse_source};
use nmlt_engine::{CheckConfig, ResultClass, Value, check_model, from_checked};

const HELP: &str = "\
NMLT structural frontend (pre-alpha)\n\n\
Usage:\n\
  nmlt check <file>       Check structural system declarations\n\
  nmlt inspect <file>     List structurally recognized systems\n\
  nmlt tokens <file>      Print the lossless Phase 1 token stream\n\
  nmlt typecheck <file>   Check the executable typed behavior fragment\n\
  nmlt model-check [--json] <file> Explore its bounded reachable-state graph\n\
  nmlt evidence <file>    Emit an explicitly unknown evidence scaffold\n\
  nmlt version            Print the frontend version\n\
  nmlt help               Show this help\n\n\
Structural success is not type checking or semantic verification.\n";

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
        "check" | "inspect" | "evidence" => {
            let path = single_path_argument(command, &arguments[1..])?;
            let parsed = load_and_parse(&path)?;
            match command {
                "check" => print_check(&path, &parsed),
                "inspect" => print_inspect(&path, &parsed),
                "evidence" => print_evidence(&path),
                _ => unreachable!(),
            }
            Ok(())
        }
        "tokens" => {
            let path = single_path_argument(command, &arguments[1..])?;
            print_tokens(&path)
        }
        "typecheck" => {
            let path = single_path_argument(command, &arguments[1..])?;
            let source = read_source(&path)?;
            let checked = compile_path(&path, source)?;
            let typed = from_checked(&checked).map_err(|errors| errors.join("\n"))?;
            println!(
                "type_checked: {} (system {}, {} state variables, {} actions, {} properties)",
                path.display(),
                typed.model().system_name,
                typed.model().states.len(),
                typed.model().actions.len(),
                typed.model().properties.len()
            );
            Ok(())
        }
        "model-check" => {
            let (json, path) = model_check_arguments(&arguments[1..])?;
            let source = read_source(&path)?;
            let checked = compile_path(&path, source)?;
            let typed = from_checked(&checked).map_err(|errors| errors.join("\n"))?;
            let report =
                check_model(&typed, CheckConfig::default()).map_err(|errors| errors.join("\n"))?;
            if json {
                println!("{}", report.to_json_pretty());
            } else {
                print_model_report(&report);
            }
            Ok(())
        }
        unknown => Err(format!("unknown command `{unknown}`\n\n{HELP}")),
    }
}

fn compile_path(path: &Path, source: String) -> Result<nmlt_kernel::CheckedProgram, String> {
    let canonical_path = path.canonicalize().map_err(|error| error.to_string())?;
    let canonical_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let repository_path = canonical_path.strip_prefix(canonical_root).map_or_else(
        |_| {
            format!(
                "external/{}",
                canonical_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("input.nmlt")
            )
        },
        |relative| relative.to_string_lossy().replace('\\', "/"),
    );
    compile_single("Main", repository_path, source).map_err(|error| error.to_string())
}

fn model_check_arguments(arguments: &[std::ffi::OsString]) -> Result<(bool, PathBuf), String> {
    match arguments {
        [path] => Ok((false, PathBuf::from(path))),
        [flag, path] if flag == "--json" => Ok((true, PathBuf::from(path))),
        [] => Err("`model-check` requires one source path".to_owned()),
        _ => Err("usage: nmlt model-check [--json] <file>".to_owned()),
    }
}

fn single_path_argument(
    command: &str,
    arguments: &[std::ffi::OsString],
) -> Result<PathBuf, String> {
    match arguments {
        [path] => Ok(PathBuf::from(path)),
        [] => Err(format!("`{command}` requires one source path")),
        _ => Err(format!("`{command}` accepts exactly one source path")),
    }
}

fn load_and_parse(path: &Path) -> Result<ParsedFile, String> {
    let source = read_source(path)?;
    match parse_source(&source) {
        Ok(parsed) => Ok(parsed),
        Err(diagnostics) => Err(render_diagnostics(path, &source, &diagnostics)),
    }
}

fn read_source(path: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|error| format!("could not read `{}`: {error}", path.display()))
}

fn print_tokens(path: &Path) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("could not read `{}`: {error}", path.display()))?;
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
    println!("note: structural parsing only; no semantic verification ran");
}

fn print_inspect(path: &Path, parsed: &ParsedFile) {
    println!("file: {}", path.display());
    for system in &parsed.systems {
        println!("system: {}", system.name);
    }
    println!("assurance: unknown (structural parsing only)");
}

fn print_evidence(path: &Path) {
    let manifest = EvidenceManifest::structural_unknown(path.display().to_string());
    println!("{}", manifest.to_json_pretty());
}

fn print_model_report(report: &nmlt_engine::CheckReport) {
    println!("system: {}", report.system);
    println!("result: {}", report.class.as_str());
    println!("complete: {}", report.complete);
    println!("states: {}", report.explored_states);
    println!("transitions: {}", report.explored_transitions);
    println!(
        "bounds: max_states={}, max_depth={}",
        report.config.max_states, report.config.max_depth
    );
    for property in &report.properties {
        println!(
            "property: {} = {} ({})",
            property.property,
            property.class.as_str(),
            property.reason
        );
        if property.class == ResultClass::Refuted {
            for step in &property
                .witness
                .as_ref()
                .expect("refutations carry traces")
                .steps
            {
                let state = step
                    .state
                    .iter()
                    .map(|(name, value)| format!("{name}={}", render_value(value)))
                    .collect::<Vec<_>>()
                    .join(", ");
                println!(
                    "  [{}] {} {{{state}}}",
                    step.index,
                    step.action.as_deref().unwrap_or("initial")
                );
            }
        }
    }
}

fn render_value(value: &Value) -> String {
    match value {
        Value::Bool(value) => value.to_string(),
        Value::Int(value) => value.to_string(),
        Value::Symbol(value) => value.clone(),
    }
}
