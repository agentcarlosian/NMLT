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
    use super::line_column;

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
}
