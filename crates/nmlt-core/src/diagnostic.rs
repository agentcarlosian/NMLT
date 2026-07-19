use std::fmt;

/// A half-open byte range in a UTF-8 source file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Diagnostic severity. Warnings do not currently arise during parsing but are
/// included so the representation can remain stable as checks expand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => formatter.write_str("error"),
            Self::Warning => formatter.write_str("warning"),
        }
    }
}

/// A source-associated frontend diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
}

impl Diagnostic {
    #[must_use]
    pub fn error(code: &'static str, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }
}

/// Render diagnostics in the canonical frontend snapshot format.
///
/// The format intentionally contains only stable data: severity, code,
/// half-open byte span, one-based location, and message. Source excerpts and
/// terminal styling are presentation concerns and are not snapshot identity.
#[must_use]
pub fn render_diagnostic_snapshot(source: &str, diagnostics: &[Diagnostic]) -> String {
    let mut output = String::new();
    for diagnostic in diagnostics {
        use std::fmt::Write as _;

        match diagnostic.span {
            Some(span) => {
                let (line, column) = line_column(source, span.start);
                writeln!(
                    output,
                    "{}[{}] bytes {}..{} at {}:{}: {}",
                    diagnostic.severity,
                    diagnostic.code,
                    span.start,
                    span.end,
                    line,
                    column,
                    diagnostic.message
                )
                .expect("writing to a String cannot fail");
            }
            None => {
                writeln!(
                    output,
                    "{}[{}] bytes - at -: {}",
                    diagnostic.severity, diagnostic.code, diagnostic.message
                )
                .expect("writing to a String cannot fail");
            }
        }
    }
    output
}

/// Convert a byte offset to one-based line and column coordinates.
#[must_use]
pub fn line_column(source: &str, byte_offset: usize) -> (usize, usize) {
    let mut bounded = byte_offset.min(source.len());
    while !source.is_char_boundary(bounded) {
        bounded -= 1;
    }
    let prefix = &source[..bounded];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit_once('\n').map_or_else(
        || prefix.chars().count() + 1,
        |(_, tail)| tail.chars().count() + 1,
    );
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, Span, line_column, render_diagnostic_snapshot};

    #[test]
    fn reports_one_based_coordinates() {
        let source = "alpha\nbeta";
        assert_eq!(line_column(source, 0), (1, 1));
        assert_eq!(line_column(source, 6), (2, 1));
        assert_eq!(line_column(source, source.len()), (2, 5));
    }

    #[test]
    fn counts_unicode_columns_as_characters() {
        let source = "λx";
        assert_eq!(line_column(source, 1), (1, 1));
        assert_eq!(line_column(source, source.len()), (1, 3));
    }

    #[test]
    fn snapshot_format_is_stable_and_style_free() {
        let source = "system S {\n  state x\n}";
        let diagnostics = [Diagnostic::error(
            "NMLT2007",
            "expected `=` in state declaration",
            Some(Span::new(20, 20)),
        )];
        assert_eq!(
            render_diagnostic_snapshot(source, &diagnostics),
            "error[NMLT2007] bytes 20..20 at 2:10: expected `=` in state declaration\n"
        );
    }
}
