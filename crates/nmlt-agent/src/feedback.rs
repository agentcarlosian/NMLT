use std::collections::BTreeMap;

use crate::authority::ByteSpan;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ResultClass {
    SyntaxAccepted,
    TypeAccepted,
    ModelChecked,
    Refuted,
    Unknown,
    Conflict,
}

impl ResultClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SyntaxAccepted => "syntax_accepted",
            Self::TypeAccepted => "type_accepted",
            Self::ModelChecked => "model_checked",
            Self::Refuted => "refuted",
            Self::Unknown => "unknown",
            Self::Conflict => "conflict",
        }
    }

    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(
            self,
            Self::SyntaxAccepted | Self::TypeAccepted | Self::ModelChecked
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseDiagnostic {
    pub code: String,
    pub primary_span: ByteSpan,
    pub related_spans: Vec<ByteSpan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeDiagnostic {
    pub code: String,
    pub declaration: String,
    pub primary_span: ByteSpan,
    pub expected: String,
    pub actual: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterexampleStep {
    pub index: usize,
    pub action: String,
    pub before: BTreeMap<String, String>,
    pub after: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Feedback {
    ParseDiagnostic(ParseDiagnostic),
    TypeDiagnostic(TypeDiagnostic),
    Counterexample {
        property_id: String,
        ordered_steps: Vec<CounterexampleStep>,
        violated_at: usize,
    },
    Unknown {
        reason: String,
        bounds_or_backend: String,
    },
    Conflict {
        raw_backend_results: BTreeMap<String, String>,
    },
}

impl Feedback {
    #[must_use]
    pub const fn can_seed_repair(&self) -> bool {
        matches!(
            self,
            Self::ParseDiagnostic(_) | Self::TypeDiagnostic(_) | Self::Counterexample { .. }
        )
    }

    #[must_use]
    pub const fn result_class(&self) -> ResultClass {
        match self {
            Self::ParseDiagnostic(_) | Self::TypeDiagnostic(_) => ResultClass::Refuted,
            Self::Counterexample { .. } => ResultClass::Refuted,
            Self::Unknown { .. } => ResultClass::Unknown,
            Self::Conflict { .. } => ResultClass::Conflict,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckResult {
    pub class: ResultClass,
    pub feedback: Option<Feedback>,
    pub checker_id: String,
}

impl CheckResult {
    #[must_use]
    pub fn checked(class: ResultClass, checker_id: impl Into<String>) -> Self {
        assert!(
            !matches!(
                class,
                ResultClass::Refuted | ResultClass::Unknown | ResultClass::Conflict
            ),
            "non-success classes require structured feedback"
        );
        Self {
            class,
            feedback: None,
            checker_id: checker_id.into(),
        }
    }

    #[must_use]
    pub fn from_feedback(feedback: Feedback, checker_id: impl Into<String>) -> Self {
        Self {
            class: feedback.result_class(),
            feedback: Some(feedback),
            checker_id: checker_id.into(),
        }
    }

    #[must_use]
    pub fn evidence_is_consistent(&self) -> bool {
        match (&self.feedback, self.class) {
            (
                None,
                ResultClass::SyntaxAccepted | ResultClass::TypeAccepted | ResultClass::ModelChecked,
            ) => true,
            (Some(feedback), class) => feedback.result_class() == class,
            _ => false,
        }
    }
}
