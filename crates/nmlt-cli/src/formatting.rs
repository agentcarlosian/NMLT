//! Workflow formatting over the canonical lossless lexer, with closure preflight.
use super::{diagnostics::Error, project, workflow};
use nmlt_core::{TokenKind as K, lex_source};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

fn newline(output: &mut String) {
    while output.ends_with(' ') {
        output.pop();
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
}
fn space(output: &mut String) {
    if !output.is_empty() && !output.ends_with([' ', '\n']) {
        output.push(' ');
    }
}
fn format(source: &str) -> Result<String, Error> {
    let lexed = lex_source(source);
    if !lexed.diagnostics.is_empty() {
        return Err(Error::new(
            "NMLT-FMT",
            "cannot format a malformed token stream",
        ));
    }
    let tokens: Vec<_> = lexed
        .tokens
        .iter()
        .filter(|t| t.kind != K::Whitespace)
        .collect();
    let mut output = String::new();
    let mut indent = 0usize;
    let mut import_line = false;
    for (index, token) in tokens.iter().enumerate() {
        let text = token.text(source);
        import_line |= token.kind == K::Identifier && text == "import";
        let next = tokens.get(index + 1).map(|t| t.text(source));
        if token.kind == K::RightBrace {
            newline(&mut output);
            indent = indent.saturating_sub(1);
        }
        if output.is_empty() || output.ends_with('\n') {
            output.push_str(&"  ".repeat(indent));
        }
        match token.kind {
            K::LeftBrace => {
                space(&mut output);
                output.push('{');
                newline(&mut output);
                indent += 1;
            }
            K::RightBrace => {
                output.push('}');
                if matches!(next, Some("," | ";" | ")" | "]")) {
                } else if next == Some("else") {
                    space(&mut output);
                } else {
                    newline(&mut output);
                }
            }
            K::LeftParen | K::LeftBracket => {
                while output.ends_with(' ') {
                    output.pop();
                }
                output.push_str(text);
            }
            K::RightParen | K::RightBracket => {
                while output.ends_with(' ') {
                    output.pop();
                }
                output.push_str(text);
            }
            K::LineComment => {
                space(&mut output);
                output.push_str(text);
                newline(&mut output);
            }
            K::BlockComment => {
                space(&mut output);
                output.push_str(text);
                space(&mut output);
            }
            K::Punctuation if text == ";" => {
                while output.ends_with(' ') {
                    output.pop();
                }
                output.push(';');
                newline(&mut output);
            }
            K::Punctuation if text == "," || text == ":" => {
                while output.ends_with(' ') {
                    output.pop();
                }
                output.push_str(text);
                space(&mut output);
            }
            K::Punctuation if text == "." => {
                while output.ends_with(' ') {
                    output.pop();
                }
                output.push('.');
            }
            K::Punctuation => {
                space(&mut output);
                output.push_str(text);
                space(&mut output);
            }
            _ => {
                if !output.ends_with([' ', '\n', '(', '[', '.']) {
                    space(&mut output);
                }
                output.push_str(text);
            }
        }
        if import_line
            && tokens
                .get(index + 1)
                .is_some_and(|next| source[token.span.end..next.span.start].contains('\n'))
        {
            newline(&mut output);
            import_line = false;
        }
    }
    newline(&mut output);
    let after = lex_source(&output);
    let before: Vec<_> = tokens.iter().map(|t| (t.kind, t.text(source))).collect();
    let after: Vec<_> = after
        .tokens
        .iter()
        .filter(|t| t.kind != K::Whitespace)
        .map(|t| (t.kind, t.text(&output)))
        .collect();
    if before != after {
        return Err(Error::new(
            "NMLT-FMT-TOKENS",
            "formatting would change a token; source was left intact",
        ));
    }
    Ok(output)
}

pub(super) fn command(arguments: &[OsString]) -> Result<(), Error> {
    let (path, check) = match arguments {
        [] => (PathBuf::from("."), false),
        [path] => (PathBuf::from(path), false),
        [path, flag] if flag == "--check" => (PathBuf::from(path), true),
        _ => return Err("usage: nmlt fmt [project-directory|source.nmlt] [--check]".into()),
    };
    let (root, program) = if path.is_dir() {
        let project = project::Project::open(&path)?;
        (project.root, project.program)
    } else {
        let physical = path.canonicalize()?;
        let parent = physical.parent().ok_or("source has no parent")?.to_owned();
        (parent, workflow::load_diagnostic(&path, None)?)
    };
    let mut originals = BTreeMap::new();
    let mut formatted = BTreeMap::new();
    let mut changed = vec![];
    for identity in program.sources() {
        let path = root.join(&identity.path);
        let source = String::from_utf8(workflow::bounded_read(
            &path,
            nmlt_workflow::MAX_SOURCE_BYTES as u64,
        )?)
        .map_err(|_| "source must be UTF-8")?;
        if super::runtime::digest(source.as_bytes()) != identity.source_sha256 {
            return Err(Error::new(
                "NMLT-FILE-CHANGED",
                "source changed during formatter preflight",
            ));
        }
        let result = format(&source)?;
        if result != source {
            changed.push(identity.path.clone());
        }
        originals.insert(identity.path.clone(), source);
        formatted.insert(identity.path.clone(), result);
    }
    let entry = &program.sources().first().ok_or("empty package")?.path;
    nmlt_workflow::compile_package(entry, |name| {
        formatted
            .get(name)
            .cloned()
            .ok_or_else(|| "unexpected formatter dependency".into())
    })
    .map_err(|e| Error::new("NMLT-FMT-COMPILE", e.diagnostic.message))?;
    if !check {
        for name in &changed {
            let path = root.join(name);
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || path.canonicalize()?.parent() != Some(Path::new(&root))
            {
                return Err("source is no longer a local regular file".into());
            }
            project::replace(
                &path,
                Some(originals[name].as_bytes()),
                formatted[name].as_bytes(),
            )?;
        }
    }
    println!(
        "{}",
        serde_json::json!({"schema":"nmlt-format-v1","check":check,"changed":changed})
    );
    if check && !changed.is_empty() {
        return Err(Error::new("NMLT-FMT-CHECK", "source formatting differs"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::format;
    #[test]
    fn preserves_comments_strings_and_is_idempotent() {
        let source = "// λ\nfn main( x : Int)->Text{let y=x+1;/* exact */\"{ // ; \\\" }\"}";
        let formatted = format(source).unwrap();
        assert_eq!(format(&formatted).unwrap(), formatted);
        assert!(formatted.contains("/* exact */"));
        assert!(formatted.contains("\"{ // ; \\\" }\""));
        assert!(formatted.contains("\n  let y"));
    }
}
