//! Shared diagnostic data; terminal and JSON render the same failure.
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub struct Location {
    pub path: String,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct Related {
    pub message: String,
    pub location: Location,
}
#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct Error(Box<Details>);
#[derive(Clone, Debug, Serialize)]
pub struct Details {
    pub schema: &'static str,
    pub severity: &'static str,
    pub code: String,
    pub message: String,
    pub location: Option<Location>,
    pub expected: Option<Value>,
    pub actual: Option<Value>,
    pub related: Vec<Related>,
}
impl std::ops::Deref for Error {
    type Target = Details;
    fn deref(&self) -> &Details {
        &self.0
    }
}
impl std::ops::DerefMut for Error {
    fn deref_mut(&mut self) -> &mut Details {
        &mut self.0
    }
}
impl Error {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self(Box::new(Details {
            schema: "nmlt-diagnostic-v1",
            severity: "error",
            code: code.into(),
            message: message.into(),
            location: None,
            expected: None,
            actual: None,
            related: vec![],
        }))
    }
    pub fn located(mut self, path: &Path, source: &str, span: nmlt_core::Span) -> Self {
        let (line, column) = nmlt_core::diagnostic::line_column(source, span.start);
        self.location = Some(Location {
            path: path.to_string_lossy().replace('\\', "/"),
            start: span.start,
            end: span.end,
            line,
            column,
        });
        self
    }
    pub fn mismatch(
        code: &str,
        message: impl Into<String>,
        expected: Value,
        actual: Value,
    ) -> Self {
        let mut result = Self::new(code, message);
        result.expected = Some(expected);
        result.actual = Some(actual);
        result
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(location) = &self.location {
            write!(
                f,
                "{}: bytes {}..{} at {}:{}: ",
                location.path, location.start, location.end, location.line, location.column
            )?;
        }
        write!(f, "error[{}]: {}", self.code, self.message)?;
        if let Some(expected) = &self.expected {
            write!(f, "\n  expected: {expected}")?;
        }
        if let Some(actual) = &self.actual {
            write!(f, "\n  actual: {actual}")?;
        }
        for related in &self.related {
            write!(
                f,
                "\n  {}:{}:{}: {}",
                related.location.path,
                related.location.line,
                related.location.column,
                related.message
            )?;
        }
        Ok(())
    }
}
impl From<String> for Error {
    fn from(e: String) -> Self {
        Self::new("NMLT-CLI", e)
    }
}
impl From<&str> for Error {
    fn from(e: &str) -> Self {
        e.to_owned().into()
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new("NMLT-IO", e.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::new("NMLT-JSON", e.to_string())
    }
}
impl From<nmlt_runtime::Error> for Error {
    fn from(e: nmlt_runtime::Error) -> Self {
        Self::new("NMLT-HOST", e.to_string())
    }
}
