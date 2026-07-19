use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nmlt_core::diagnostic::line_column;
use nmlt_core::{Diagnostic, EvidenceManifest, ParsedFile, lex_source, parse_source};

const HELP: &str = "\
NMLT structural frontend (pre-alpha)\n\n\
Usage:\n\
  nmlt check <file>       Check structural system declarations\n\
  nmlt inspect <file>     List structurally recognized systems\n\
  nmlt tokens <file>      Print the lossless Phase 1 token stream\n\
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
        unknown => Err(format!("unknown command `{unknown}`\n\n{HELP}")),
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
    let source = fs::read_to_string(path)
        .map_err(|error| format!("could not read `{}`: {error}", path.display()))?;
    match parse_source(&source) {
        Ok(parsed) => Ok(parsed),
        Err(diagnostics) => Err(render_diagnostics(path, &source, &diagnostics)),
    }
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
