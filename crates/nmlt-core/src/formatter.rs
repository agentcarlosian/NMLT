use crate::{Diagnostic, SyntaxParse, parse_cst};

/// The deliberately conservative Phase 1 formatter mode.
///
/// `Preserve` emits the concrete syntax tree exactly. Introducing whitespace
/// normalization requires a separate, reviewed mode because comments and
/// malformed recovery text are part of the lossless syntax contract.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FormatMode {
    #[default]
    Preserve,
}

/// Formatting output paired with the syntax diagnostics that constrain it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatOutput {
    text: String,
    diagnostics: Vec<Diagnostic>,
}

impl FormatOutput {
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn into_text(self) -> String {
        self.text
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Parse and conservatively format a source file.
///
/// The Phase 1 policy is byte preservation for both valid and recovered input.
/// Diagnostics are returned so callers cannot confuse preserved malformed text
/// with an accepted model.
#[must_use]
pub fn format_source(source: &str, mode: FormatMode) -> FormatOutput {
    let parsed = parse_cst(source);
    format_cst(&parsed, mode)
}

/// Format an existing immutable syntax tree without reparsing it.
#[must_use]
pub fn format_cst(parsed: &SyntaxParse, mode: FormatMode) -> FormatOutput {
    match mode {
        FormatMode::Preserve => FormatOutput {
            text: parsed.reconstruct(),
            diagnostics: parsed.diagnostics().to_vec(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{FormatMode, format_source};

    #[test]
    fn preservation_mode_is_byte_exact_and_idempotent() {
        let source = "// keep λ and CRLF\r\nsystem S {\r\n\tstate x: Nat = 0 /* keep */\r\n}\r\n";
        let once = format_source(source, FormatMode::Preserve);
        let twice = format_source(once.text(), FormatMode::Preserve);
        assert!(once.diagnostics().is_empty());
        assert_eq!(once.text(), source);
        assert_eq!(twice.text(), once.text());
    }

    #[test]
    fn recovered_text_is_preserved_but_not_reported_as_valid() {
        let source = "system S { action go { mystery }";
        let formatted = format_source(source, FormatMode::Preserve);
        assert_eq!(formatted.text(), source);
        assert!(!formatted.diagnostics().is_empty());
    }
}
