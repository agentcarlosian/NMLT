//! Native Lean JSON messages with positions mapped to the captured source.
use super::{Result, decode_json, err, module_path};
use nmlt_runtime::{process, sha256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Position {
    line: usize,
    column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Severity {
    Information,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Message {
    file_name: String,
    pos: Position,
    end_pos: Option<Position>,
    keep_full_range: bool,
    severity: Severity,
    is_silent: bool,
    caption: String,
    data: String,
    kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Range {
    start_byte: usize,
    end_byte: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Diagnostic {
    message: Message,
    source_range: Option<Range>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Report {
    schema: String,
    module: String,
    source_path: String,
    source_sha256: String,
    stage_path: String,
    position_encoding: String,
    exit_code: Option<i32>,
    diagnostics: Vec<Diagnostic>,
    other_stdout_lines: usize,
}

impl Report {
    pub fn parse(
        module: &str,
        source: &str,
        absolute_source: &Path,
        stage: usize,
        output: &process::Output,
    ) -> Result<Self> {
        let relative = module_path(module).to_string_lossy().replace('\\', "/");
        let absolute = absolute_source.to_string_lossy().replace('\\', "/");
        let text = std::str::from_utf8(&output.stdout).map_err(err)?;
        let mut diagnostics = vec![];
        let mut other_stdout_lines = 0;
        for row in text.lines().filter(|row| !row.is_empty()) {
            let parsed = decode_json::<Value>(row.as_bytes());
            let value = match parsed {
                Ok(value) if value.get("severity").is_some() && value.get("fileName").is_some() => {
                    value
                }
                _ => {
                    // Project IO and the fixed helper markers are preserved in
                    // the raw stage output; they are not native JSON messages.
                    other_stdout_lines += 1;
                    continue;
                }
            };
            let message: Message = serde_json::from_value(value).map_err(err)?;
            if diagnostics.len() >= 256
                || message.file_name.len() > 4096
                || message.kind.len() > 4096
            {
                return Err("Lean diagnostics exceed message or name bounds".into());
            }
            let reported = message.file_name.replace('\\', "/");
            let source_range = if reported == relative || reported == absolute {
                let start = offset(source, &message.pos);
                let end = message
                    .end_pos
                    .as_ref()
                    .map_or(start, |pos| offset(source, pos));
                start
                    .zip(end)
                    .filter(|(start, end)| start <= end)
                    .map(|(start_byte, end_byte)| Range {
                        start_byte,
                        end_byte,
                    })
            } else {
                None
            };
            diagnostics.push(Diagnostic {
                message,
                source_range,
            });
        }
        Ok(Self {
            schema: "nmlt-lean-diagnostics-v1".into(),
            module: module.into(),
            source_path: format!("build/{relative}"),
            source_sha256: sha256(source.as_bytes()),
            stage_path: format!("stage-{stage}.json"),
            position_encoding:
                "one-based-lines;zero-based-Unicode-scalar-columns;UTF-8-byte-ranges".into(),
            exit_code: output.exit_code,
            diagnostics,
            other_stdout_lines,
        })
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.severity == Severity::Error)
    }

    pub fn error_text(&self) -> String {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.message.severity == Severity::Error)
            .map(|diagnostic| {
                let message = &diagnostic.message;
                format!(
                    "{}:{}:{}: {}{}{}",
                    message.file_name,
                    message.pos.line,
                    message.pos.column.saturating_add(1),
                    message.caption,
                    if message.caption.is_empty() { "" } else { ": " },
                    message.data
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub(super) fn metadata_rows(bytes: &[u8], marker: &str) -> Result<Vec<String>> {
    let text = std::str::from_utf8(bytes).map_err(err)?;
    let mut rows = vec![];
    for row in text.lines() {
        if let Some(value) = row.strip_prefix(marker) {
            rows.push(value.into());
        } else if let Ok(message) = decode_json::<Message>(row.as_bytes())
            && message.severity == Severity::Information
        {
            rows.extend(
                message
                    .data
                    .lines()
                    .filter_map(|line| line.strip_prefix(marker))
                    .map(str::to_owned),
            );
        }
    }
    Ok(rows)
}

/// Lean Position columns count Unicode scalar values, not UTF-8 bytes or LSP UTF-16 units.
fn offset(source: &str, position: &Position) -> Option<usize> {
    if position.line == 0 {
        return None;
    }
    let mut line_start = 0;
    for _ in 1..position.line {
        line_start += source.get(line_start..)?.find('\n')? + 1;
    }
    let rest = source.get(line_start..)?;
    let line = &rest[..rest.find('\n').unwrap_or(rest.len())];
    if let Some((offset, _)) = line.char_indices().nth(position.column) {
        Some(line_start + offset)
    } else if position.column == line.chars().count() {
        Some(line_start + line.len())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn message(file: &str, start: usize, end: usize) -> Value {
        json!({"fileName":file,"pos":{"line":2,"column":start},"endPos":{"line":2,"column":end},
            "keepFullRange":false,"severity":"error","isSilent":false,"caption":"",
            "data":"type mismatch","kind":"[anonymous]"})
    }
    fn parse(messages: &[Value], source: &str) -> Report {
        let output = process::Output {
            exit_code: Some(1),
            stdout: messages
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n")
                .into_bytes(),
            stderr: vec![],
        };
        Report::parse(
            "Example.Goals",
            source,
            Path::new("/saved/Example/Goals.lean"),
            2,
            &output,
        )
        .unwrap()
    }

    #[test]
    fn native_unicode_columns_map_to_exact_captured_utf8_bytes() {
        let source = "first\r\nα😀z\r\n";
        let report = parse(&[message("Example/Goals.lean", 1, 2)], source);
        assert_eq!(
            report.diagnostics[0].source_range,
            Some(Range {
                start_byte: 9,
                end_byte: 13
            })
        );
        assert_eq!(&source[9..13], "😀");
        assert!(report.has_errors());
        assert_eq!(report.source_sha256, sha256(source.as_bytes()));
        assert_eq!(
            offset(source, &Position { line: 3, column: 0 }),
            Some(source.len())
        );
        assert_eq!(offset(source, &Position { line: 2, column: 5 }), None);
        assert_eq!(offset(source, &Position { line: 0, column: 0 }), None);
    }

    #[test]
    fn foreign_synthetic_and_reversed_ranges_remain_unmapped() {
        let report = parse(
            &[
                message("Outside.lean", 0, 1),
                message("Example/Goals.lean", 9, 10),
                message("Example/Goals.lean", 2, 1),
            ],
            "first\nab\n",
        );
        assert!(
            report
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.source_range.is_none())
        );
        let absolute = parse(&[message("/saved/Example/Goals.lean", 0, 1)], "first\nab\n");
        assert!(absolute.diagnostics[0].source_range.is_some());
    }

    #[test]
    fn warnings_and_program_output_do_not_become_proof_acceptance() {
        let mut warning = message("Example/Goals.lean", 0, 1);
        warning["severity"] = json!("warning");
        let output = process::Output {
            exit_code: Some(0),
            stdout: format!("{}\nNMLT_TASK={{}}\nproject log\n", warning).into_bytes(),
            stderr: vec![],
        };
        let report = Report::parse(
            "Example.Goals",
            "first\na",
            Path::new("/saved/Example/Goals.lean"),
            1,
            &output,
        )
        .unwrap();
        assert!(!report.has_errors());
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.other_stdout_lines, 2);
    }

    #[test]
    fn helper_metadata_can_be_wrapped_in_native_information_messages() {
        let mut wrapped = message("NMLTTask.lean", 0, 1);
        wrapped["severity"] = json!("information");
        wrapped["data"] = json!("NMLT_TASK=[\"Nat\"]\n");
        let decoded: Vec<String> =
            super::super::parse_marker(wrapped.to_string().as_bytes(), "NMLT_TASK=").unwrap();
        assert_eq!(decoded, ["Nat"]);
        let duplicate = format!("{}\n{}", wrapped, wrapped);
        assert!(
            super::super::parse_marker::<Vec<String>>(duplicate.as_bytes(), "NMLT_TASK=").is_err()
        );
        wrapped["severity"] = json!("warning");
        assert!(
            super::super::parse_marker::<Vec<String>>(wrapped.to_string().as_bytes(), "NMLT_TASK=")
                .is_err()
        );
    }
}
