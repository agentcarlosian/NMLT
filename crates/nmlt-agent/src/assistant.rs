use crate::authority::{ByteSpan, Edit, Proposal};
use crate::digest::sha256_hex;
use crate::feedback::Feedback;

/// The complete information visible to a repair assistant.
///
/// Trusted intent/property/oracle bytes, expected patches, and expected
/// outcomes are intentionally absent. The authority gate and evaluator retain
/// those objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssistantInput {
    task_id: String,
    candidate_path: String,
    candidate_source: String,
    editable_spans: Vec<ByteSpan>,
    feedback: Feedback,
}

impl AssistantInput {
    #[must_use]
    pub fn bounded(
        task_id: impl Into<String>,
        candidate_path: impl Into<String>,
        candidate_source: impl Into<String>,
        editable_spans: Vec<ByteSpan>,
        feedback: Feedback,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            candidate_path: candidate_path.into(),
            candidate_source: candidate_source.into(),
            editable_spans,
            feedback,
        }
    }

    #[must_use]
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    #[must_use]
    pub fn candidate_path(&self) -> &str {
        &self.candidate_path
    }

    #[must_use]
    pub fn candidate_source(&self) -> &str {
        &self.candidate_source
    }

    #[must_use]
    pub fn feedback(&self) -> &Feedback {
        &self.feedback
    }

    fn span_is_editable(&self, span: ByteSpan) -> bool {
        self.editable_spans
            .iter()
            .any(|allowed| allowed.start <= span.start && span.end <= allowed.end)
    }
}

pub trait RepairAssistant {
    fn propose(&self, input: &AssistantInput) -> Option<Proposal>;
}

/// A deterministic protocol-conformance baseline.
///
/// This is deliberately a few general, local transformations. It is not a
/// language model and its benchmark score is not evidence about LLM ability.
#[derive(Clone, Copy, Debug, Default)]
pub struct DeterministicAssistant;

impl DeterministicAssistant {
    fn proposal(input: &AssistantInput, edit: Edit, rationale: &str) -> Proposal {
        let material = format!(
            "{}\0{}\0{}\0{}\0{}",
            input.task_id, edit.path, edit.span.start, edit.span.end, edit.replacement
        );
        Proposal::localized(
            format!("proposal:sha256:{}", sha256_hex(material.as_bytes())),
            vec![edit],
            rationale,
        )
    }

    fn parse_repair(input: &AssistantInput, span: ByteSpan, code: &str) -> Option<Proposal> {
        if code != "NMLT1001" || !span.is_empty() || !input.span_is_editable(span) {
            return None;
        }
        Some(Self::proposal(
            input,
            Edit::candidate(&input.candidate_path, span, ";"),
            "insert the locally reported missing declaration terminator",
        ))
    }

    fn type_repair(
        input: &AssistantInput,
        span: ByteSpan,
        expected: &str,
        actual: &str,
    ) -> Option<Proposal> {
        if !input.span_is_editable(span)
            || expected != "Bool"
            || actual != "Nat"
            || span.start > span.end
            || span.end > input.candidate_source.len()
            || !input.candidate_source.is_char_boundary(span.start)
            || !input.candidate_source.is_char_boundary(span.end)
        {
            return None;
        }
        let replacement = match &input.candidate_source[span.start..span.end] {
            "0" => "false",
            "1" => "true",
            _ => return None,
        };
        Some(Self::proposal(
            input,
            Edit::candidate(&input.candidate_path, span, replacement),
            "replace a diagnostic-local numeric Boolean surrogate with a Boolean literal",
        ))
    }

    fn counterexample_repair(input: &AssistantInput, violated_at: usize) -> Option<Proposal> {
        let Feedback::Counterexample { ordered_steps, .. } = &input.feedback else {
            return None;
        };
        let step = ordered_steps
            .iter()
            .find(|step| step.index == violated_at)
            .or_else(|| ordered_steps.last())?;

        // Infer a necessary guard candidate from a Boolean fact that was false
        // before the violating action and stayed false afterward. No property
        // text or expected patch is present in this interface.
        let guard = step.before.iter().find_map(|(name, before)| {
            let after = step.after.get(name)?;
            (before == "false" && after == "false").then_some(name)
        })?;
        if !guard
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return None;
        }

        let action_marker = format!("action {}", step.action);
        let action_start = input.candidate_source.find(&action_marker)?;
        let brace_relative = input.candidate_source[action_start..].find('{')?;
        let insertion = action_start + brace_relative + 1;
        let span = ByteSpan::new(insertion, insertion);
        if !input.span_is_editable(span) {
            return None;
        }
        Some(Self::proposal(
            input,
            Edit::candidate(
                &input.candidate_path,
                span,
                format!("\n    require {guard};"),
            ),
            "add a local guard inferred from the violating transition's unchanged false fact",
        ))
    }
}

impl RepairAssistant for DeterministicAssistant {
    fn propose(&self, input: &AssistantInput) -> Option<Proposal> {
        match &input.feedback {
            Feedback::ParseDiagnostic(diagnostic) => {
                Self::parse_repair(input, diagnostic.primary_span, &diagnostic.code)
            }
            Feedback::TypeDiagnostic(diagnostic) => Self::type_repair(
                input,
                diagnostic.primary_span,
                &diagnostic.expected,
                &diagnostic.actual,
            ),
            Feedback::Counterexample { violated_at, .. } => {
                Self::counterexample_repair(input, *violated_at)
            }
            Feedback::Unknown { .. } | Feedback::Conflict { .. } => None,
        }
    }
}
