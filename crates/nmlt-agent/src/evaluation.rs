use std::collections::BTreeMap;

use crate::artifact::{ArtifactRole, ArtifactSet, TrustedArtifact};
use crate::assistant::{AssistantInput, DeterministicAssistant, RepairAssistant};
use crate::authority::{
    AuthorityError, ByteSpan, CandidateFile, Edit, EditPolicy, Proposal, apply_proposal,
    validate_proposal,
};
use crate::digest::sha256_hex;
use crate::feedback::{
    CheckResult, CounterexampleStep, Feedback, ParseDiagnostic, ResultClass, TypeDiagnostic,
};
use crate::graph::{ArtifactGraph, ArtifactNode, Edge};

const CHECKER_ID: &str = "nmlt-agent-held-out-checker-v1";
const ASSISTANT_ID: &str = "nmlt-deterministic-protocol-baseline-v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompletionStage {
    Syntax,
    Type,
    Semantic,
}

impl CompletionStage {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::Type => "type",
            Self::Semantic => "semantic",
        }
    }

    const fn expected_class(self) -> ResultClass {
        match self {
            Self::Syntax => ResultClass::SyntaxAccepted,
            Self::Type => ResultClass::TypeAccepted,
            Self::Semantic => ResultClass::ModelChecked,
        }
    }
}

#[derive(Clone, Debug)]
struct HeldOutTask {
    id: &'static str,
    path: &'static str,
    source: &'static str,
    intent: &'static str,
    property: &'static str,
    oracle: &'static str,
    stage: CompletionStage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskEvaluation {
    pub task_id: String,
    pub stage: String,
    pub baseline_class: ResultClass,
    pub assisted_class: ResultClass,
    pub completed: bool,
    pub feedback_rounds: usize,
    pub localized_edit_count: usize,
    pub edited_byte_extent: usize,
    pub trusted_identities_unchanged: bool,
    pub negative_controls_retained: usize,
    pub negative_controls_killed: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StageMetrics {
    pub eligible: usize,
    pub baseline_completed: usize,
    pub assisted_completed: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EvaluationMetrics {
    pub task_count: usize,
    pub baseline_completed: usize,
    pub assisted_completed: usize,
    pub syntax: StageMetrics,
    pub typing: StageMetrics,
    pub semantic: StageMetrics,
    pub feedback_rounds: usize,
    pub localized_edit_count: usize,
    pub edited_byte_extent: usize,
    pub trusted_modification_attempts: usize,
    pub trusted_modification_rejections: usize,
    pub negative_controls_retained: usize,
    pub negative_controls_killed: usize,
    pub unknown_results_promoted: usize,
    pub conflict_results_promoted: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationReport {
    pub schema_version: String,
    pub suite_id: String,
    pub assistant_id: String,
    pub assistant_kind: String,
    pub checker_id: String,
    pub split: String,
    pub expected_patches_exposed: bool,
    pub tasks: Vec<TaskEvaluation>,
    pub metrics: EvaluationMetrics,
    pub graph: ArtifactGraph,
    pub artifact_graph_readback_verified: bool,
}

impl EvaluationReport {
    #[must_use]
    pub fn to_json(&self) -> String {
        let tasks = self
            .tasks
            .iter()
            .map(|task| {
                format!(
                    concat!(
                        "{{\"task_id\":\"{}\",\"stage\":\"{}\",",
                        "\"baseline_class\":\"{}\",\"assisted_class\":\"{}\",",
                        "\"completed\":{},\"feedback_rounds\":{},",
                        "\"localized_edit_count\":{},\"edited_byte_extent\":{},",
                        "\"trusted_identities_unchanged\":{},",
                        "\"negative_controls_retained\":{},\"negative_controls_killed\":{}}}"
                    ),
                    json_escape(&task.task_id),
                    task.stage,
                    task.baseline_class.as_str(),
                    task.assisted_class.as_str(),
                    task.completed,
                    task.feedback_rounds,
                    task.localized_edit_count,
                    task.edited_byte_extent,
                    task.trusted_identities_unchanged,
                    task.negative_controls_retained,
                    task.negative_controls_killed
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            concat!(
                "{{\"schema_version\":\"{}\",\"suite_id\":\"{}\",",
                "\"assistant\":{{\"id\":\"{}\",\"kind\":\"{}\"}},",
                "\"checker_id\":\"{}\",\"split\":\"{}\",",
                "\"expected_patches_exposed\":{},\"tasks\":[{}],",
                "\"metrics\":{},\"artifact_graph_readback_verified\":{},",
                "\"artifact_graph\":{}}}"
            ),
            self.schema_version,
            self.suite_id,
            self.assistant_id,
            self.assistant_kind,
            self.checker_id,
            self.split,
            self.expected_patches_exposed,
            tasks,
            metrics_json(&self.metrics),
            self.artifact_graph_readback_verified,
            self.graph.to_json()
        )
    }
}

fn held_out_tasks() -> [HeldOutTask; 3] {
    [
        HeldOutTask {
            id: "held-out-syntax-terminator",
            path: "benchmarks/agentic/candidates/held-out-syntax.nmlt",
            source: "system SyntaxHeldOut {\n  state ready: Bool = false\n}\n",
            intent: "Declare a Boolean readiness state in a well-formed system.\n",
            property: "Every state declaration in this task is explicitly terminated.\n",
            oracle: "Reject a state declaration whose terminator is deleted.\n",
            stage: CompletionStage::Syntax,
        },
        HeldOutTask {
            id: "held-out-type-boolean",
            path: "benchmarks/agentic/candidates/held-out-type.nmlt",
            source: "system TypeHeldOut {\n  state ready: Bool = 0;\n}\n",
            intent: "Represent readiness as a Boolean state.\n",
            property: "The initializer of ready has the declared Bool type.\n",
            oracle: "Reject numeric literals used as Boolean initializers.\n",
            stage: CompletionStage::Type,
        },
        HeldOutTask {
            id: "held-out-semantic-authority",
            path: "benchmarks/agentic/candidates/held-out-semantic.nmlt",
            source: concat!(
                "system SemanticHeldOut {\n",
                "  state armed: Bool = false;\n",
                "  state dispatched: Bool = false;\n",
                "  action authorize {\n",
                "    set armed = true;\n",
                "  }\n",
                "  action dispatch {\n",
                "    set dispatched = true;\n",
                "  }\n",
                "}\n"
            ),
            intent: "Dispatch is permitted only after an independently reachable authorization step.\n",
            property: "always(dispatched implies armed)\n",
            oracle: "Dispatch must remain reachable after authorization; deleting its authority guard must be refuted.\n",
            stage: CompletionStage::Semantic,
        },
    ]
}

fn frozen_artifacts(task: &HeldOutTask) -> Result<ArtifactSet, String> {
    let mut artifacts = ArtifactSet::default();
    artifacts.insert(TrustedArtifact::freeze(
        format!("intent:{}", task.id),
        ArtifactRole::Intent,
        format!("benchmarks/agentic/trusted/{}.intent.txt", task.id),
        task.intent.as_bytes().to_vec(),
    ))?;
    artifacts.insert(TrustedArtifact::freeze(
        format!("property:{}", task.id),
        ArtifactRole::Property,
        format!("benchmarks/agentic/trusted/{}.property.txt", task.id),
        task.property.as_bytes().to_vec(),
    ))?;
    artifacts.insert(TrustedArtifact::freeze(
        format!("oracle:{}", task.id),
        ArtifactRole::Oracle,
        format!("benchmarks/agentic/trusted/{}.oracle.txt", task.id),
        task.oracle.as_bytes().to_vec(),
    ))?;
    Ok(artifacts)
}

fn edit_policy(task: &HeldOutTask, artifacts: &ArtifactSet) -> Result<EditPolicy, AuthorityError> {
    let mut policy = EditPolicy::localized(2, 96);
    policy.allow_span(task.path, ByteSpan::new(0, task.source.len()));
    for artifact in artifacts.iter() {
        policy.protect_path(&artifact.path, &artifact.bytes);
    }
    if task.stage == CompletionStage::Semantic {
        let end = task
            .source
            .find('\n')
            .expect("frozen semantic fixture has a system header");
        policy.protect_span(task.path, ByteSpan::new(0, end), task.source)?;
    }
    Ok(policy)
}

fn check(task: &HeldOutTask, source: &str) -> CheckResult {
    if let Some(diagnostic) = syntax_diagnostic(source) {
        return CheckResult::from_feedback(Feedback::ParseDiagnostic(diagnostic), CHECKER_ID);
    }
    if task.stage == CompletionStage::Syntax {
        return CheckResult::checked(ResultClass::SyntaxAccepted, CHECKER_ID);
    }
    if let Some(diagnostic) = type_diagnostic(source) {
        return CheckResult::from_feedback(Feedback::TypeDiagnostic(diagnostic), CHECKER_ID);
    }
    if task.stage == CompletionStage::Type {
        return CheckResult::checked(ResultClass::TypeAccepted, CHECKER_ID);
    }
    semantic_check(source)
}

fn syntax_diagnostic(source: &str) -> Option<ParseDiagnostic> {
    let mut offset = 0_usize;
    for line in source.split_inclusive('\n') {
        let without_newline = line.trim_end_matches(['\n', '\r']);
        let trimmed = without_newline.trim();
        if trimmed.starts_with("state ") && !trimmed.ends_with(';') {
            let insertion = offset + without_newline.len();
            return Some(ParseDiagnostic {
                code: "NMLT1001".into(),
                primary_span: ByteSpan::new(insertion, insertion),
                related_spans: Vec::new(),
            });
        }
        offset += line.len();
    }
    None
}

fn type_diagnostic(source: &str) -> Option<TypeDiagnostic> {
    let mut offset = 0_usize;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with("state ") && trimmed.contains(": Bool") {
            let equals = line.find('=')?;
            let tail = &line[equals + 1..];
            let literal = tail.trim_start();
            let leading = tail.len() - literal.len();
            let token = literal.trim_end_matches([';', '\n', '\r', ' ']);
            if matches!(token, "0" | "1") {
                let start = offset + equals + 1 + leading;
                return Some(TypeDiagnostic {
                    code: "NMLT2101".into(),
                    declaration: trimmed
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("<unknown>")
                        .trim_end_matches(':')
                        .into(),
                    primary_span: ByteSpan::new(start, start + token.len()),
                    expected: "Bool".into(),
                    actual: "Nat".into(),
                });
            }
        }
        offset += line.len();
    }
    None
}

fn semantic_check(source: &str) -> CheckResult {
    let dispatch = action_body(source, "dispatch").unwrap_or_default();
    let authorize = action_body(source, "authorize").unwrap_or_default();
    let has_guard = dispatch.contains("require armed;");
    let dispatch_effect = dispatch.contains("set dispatched = true;");
    let authorization_effect = authorize.contains("set armed = true;");

    if !has_guard && dispatch_effect {
        let mut before = BTreeMap::new();
        before.insert("armed".into(), "false".into());
        before.insert("dispatched".into(), "false".into());
        let mut after = before.clone();
        after.insert("dispatched".into(), "true".into());
        return CheckResult::from_feedback(
            Feedback::Counterexample {
                property_id: format!(
                    "property:sha256:{}",
                    sha256_hex(b"always(dispatched implies armed)")
                ),
                ordered_steps: vec![CounterexampleStep {
                    index: 0,
                    action: "dispatch".into(),
                    before,
                    after,
                }],
                violated_at: 0,
            },
            CHECKER_ID,
        );
    }
    if has_guard && dispatch_effect && authorization_effect {
        // Authorization makes armed=true, so dispatch is both reachable and
        // safe. This is the semantic anti-vacuity control.
        return CheckResult::checked(ResultClass::ModelChecked, CHECKER_ID);
    }
    CheckResult::from_feedback(
        Feedback::Unknown {
            reason: "semantic anti-vacuity obligation was not established".into(),
            bounds_or_backend: "held-out finite protocol checker".into(),
        },
        CHECKER_ID,
    )
}

fn action_body<'a>(source: &'a str, action: &str) -> Option<&'a str> {
    let marker = format!("action {action}");
    let start = source.find(&marker)?;
    let open = start + source[start..].find('{')?;
    let mut depth = 0_usize;
    for (relative, character) in source[open..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(&source[open + 1..open + relative]);
                }
            }
            _ => {}
        }
    }
    None
}

fn negative_control_killed(task: &HeldOutTask, repaired: &str) -> bool {
    let mutant = match task.stage {
        CompletionStage::Syntax => repaired.replacen(';', "", 1),
        CompletionStage::Type => repaired.replacen("false", "0", 1),
        CompletionStage::Semantic => repaired.replacen("require armed;", "", 1),
    };
    !check(task, &mutant).class.is_success()
}

fn add_task_graph(
    graph: &mut ArtifactGraph,
    task: &HeldOutTask,
    artifacts: &ArtifactSet,
    baseline: &CheckResult,
    proposal: &Proposal,
    assisted: &CheckResult,
    baseline_candidate: &CandidateFile,
) -> Result<(), String> {
    for artifact in artifacts.iter() {
        graph.add_node(ArtifactNode {
            id: artifact.id.clone(),
            role: artifact.role,
            digest: artifact.digest.clone(),
            summary: format!("frozen {} for {}", artifact.role.as_str(), task.id),
            result_class: None,
        })?;
    }
    let candidate_id = format!("candidate:{}", task.id);
    let feedback_id = format!("feedback:{}", task.id);
    let evaluation_id = format!("evaluation:{}", task.id);
    graph.add_node(ArtifactNode {
        id: candidate_id.clone(),
        role: ArtifactRole::Candidate,
        digest: baseline_candidate.digest.clone(),
        summary: "held-out baseline candidate".into(),
        result_class: None,
    })?;
    graph.add_node(ArtifactNode {
        id: feedback_id.clone(),
        role: ArtifactRole::Feedback,
        digest: format!("sha256:{}", sha256_hex(format!("{baseline:?}").as_bytes())),
        summary: "checker-produced structured baseline feedback".into(),
        result_class: Some(baseline.class),
    })?;
    graph.add_node(ArtifactNode {
        id: proposal.proposal_id.clone(),
        role: ArtifactRole::Proposal,
        digest: proposal.proposal_id.trim_start_matches("proposal:").into(),
        summary: "untrusted localized proposal".into(),
        result_class: None,
    })?;
    graph.add_node(ArtifactNode {
        id: evaluation_id.clone(),
        role: ArtifactRole::Evaluation,
        digest: format!("sha256:{}", sha256_hex(format!("{assisted:?}").as_bytes())),
        summary: "isolated full recheck with controls".into(),
        result_class: Some(assisted.class),
    })?;
    let intent_id = format!("intent:{}", task.id);
    let property_id = format!("property:{}", task.id);
    let oracle_id = format!("oracle:{}", task.id);
    for edge in [
        Edge {
            from: intent_id,
            relation: "constrains".into(),
            to: property_id.clone(),
        },
        Edge {
            from: property_id,
            relation: "checked_in".into(),
            to: evaluation_id.clone(),
        },
        Edge {
            from: oracle_id,
            relation: "controls".into(),
            to: evaluation_id.clone(),
        },
        Edge {
            from: candidate_id,
            relation: "produced".into(),
            to: feedback_id.clone(),
        },
        Edge {
            from: feedback_id,
            relation: "seeded".into(),
            to: proposal.proposal_id.clone(),
        },
        Edge {
            from: proposal.proposal_id.clone(),
            relation: "rechecked_as".into(),
            to: evaluation_id,
        },
    ] {
        graph.add_edge(edge)?;
    }
    Ok(())
}

fn rejected_authority_probes(
    task: &HeldOutTask,
    candidate: &CandidateFile,
    policy: &EditPolicy,
    artifacts: &ArtifactSet,
) -> (usize, usize) {
    let candidates = BTreeMap::from([(candidate.path.clone(), candidate.clone())]);
    let full = ByteSpan::new(0, candidate.source.len());
    let probes = [
        Proposal::localized(
            "probe:property-weakening",
            vec![Edit {
                role: ArtifactRole::Property,
                path: format!("benchmarks/agentic/trusted/{}.property.txt", task.id),
                span: ByteSpan::new(0, 1),
                replacement: "x".into(),
            }],
            "attempt specification weakening",
        ),
        Proposal::localized(
            "probe:oracle-drop",
            vec![Edit {
                role: ArtifactRole::Oracle,
                path: format!("benchmarks/agentic/trusted/{}.oracle.txt", task.id),
                span: ByteSpan::new(0, 1),
                replacement: String::new(),
            }],
            "attempt negative-control deletion",
        ),
        Proposal::localized(
            "probe:path-traversal",
            vec![Edit::candidate(
                "../trusted/property.txt",
                ByteSpan::new(0, 0),
                "weaken",
            )],
            "attempt path traversal",
        ),
        Proposal::localized(
            "probe:symlink-like",
            vec![Edit::candidate(
                "benchmarks/agentic/candidates/link/../../trusted/oracle.txt",
                ByteSpan::new(0, 0),
                "drop",
            )],
            "attempt symlink-like escape",
        ),
        Proposal::localized(
            "probe:whole-file",
            vec![Edit::candidate(
                &candidate.path,
                full,
                "system Replaced {}\n",
            )],
            "attempt whole-file replacement",
        ),
        Proposal {
            proposal_id: "probe:forged-result".into(),
            edits: vec![Edit::candidate(&candidate.path, ByteSpan::new(0, 0), " ")],
            rationale: "attempt result forgery".into(),
            claimed_result: Some(ResultClass::ModelChecked),
        },
    ];
    let mut attempts = probes.len();
    let mut rejections = probes
        .iter()
        .filter(|proposal| validate_proposal(&candidates, policy, proposal).is_err())
        .count();

    // Dropping an oracle artifact is also rejected even though it is not an
    // edit proposal: exact set readback must match the frozen set.
    attempts += 1;
    let mut dropped = ArtifactSet::default();
    for artifact in artifacts
        .iter()
        .filter(|artifact| artifact.role != ArtifactRole::Oracle)
    {
        let _ = dropped.insert(artifact.clone());
    }
    if dropped.verify_frozen(artifacts).is_err() {
        rejections += 1;
    }
    (attempts, rejections)
}

pub fn evaluate_held_out_suite() -> Result<EvaluationReport, String> {
    let assistant = DeterministicAssistant;
    let mut reports = Vec::new();
    let mut graph = ArtifactGraph::new("nmlt-agentic-held-out-v1:artifact-graph");
    let mut metrics = EvaluationMetrics::default();

    for task in held_out_tasks() {
        metrics.task_count += 1;
        let artifacts = frozen_artifacts(&task)?;
        let frozen = artifacts.clone();
        let policy = edit_policy(&task, &artifacts).map_err(|error| error.to_string())?;
        let candidate = CandidateFile::new(task.path, task.source);
        let candidates = BTreeMap::from([(candidate.path.clone(), candidate.clone())]);
        let actual_protected = artifacts
            .iter()
            .map(|artifact| (artifact.path.as_str(), artifact.bytes.as_slice()))
            .collect::<Vec<_>>();
        policy
            .verify_protected_paths(actual_protected)
            .map_err(|error| error.to_string())?;

        let baseline = check(&task, &candidate.source);
        if !baseline.evidence_is_consistent() {
            return Err(format!("inconsistent baseline evidence for {}", task.id));
        }
        let feedback = baseline
            .feedback
            .clone()
            .ok_or_else(|| format!("held-out baseline {} unexpectedly completed", task.id))?;
        let input = AssistantInput::bounded(
            task.id,
            task.path,
            &candidate.source,
            policy
                .editable_spans
                .get(task.path)
                .cloned()
                .unwrap_or_default(),
            feedback,
        );
        let proposal = assistant
            .propose(&input)
            .ok_or_else(|| format!("baseline assistant produced no proposal for {}", task.id))?;
        let applied = apply_proposal(&candidates, &policy, &proposal)
            .map_err(|error| format!("authority rejection for {}: {error}", task.id))?;
        let repaired = &applied
            .files
            .get(task.path)
            .ok_or_else(|| "repaired candidate disappeared".to_owned())?
            .source;
        let assisted = check(&task, repaired);
        if !assisted.evidence_is_consistent() {
            return Err(format!("inconsistent assisted evidence for {}", task.id));
        }
        frozen.verify_frozen(&artifacts)?;
        let completed = assisted.class == task.stage.expected_class();
        let killed = usize::from(negative_control_killed(&task, repaired));
        let (attempts, rejections) =
            rejected_authority_probes(&task, &candidate, &policy, &artifacts);

        metrics.feedback_rounds += 1;
        metrics.localized_edit_count += proposal.edits.len();
        metrics.edited_byte_extent += applied.edited_bytes;
        metrics.trusted_modification_attempts += attempts;
        metrics.trusted_modification_rejections += rejections;
        metrics.negative_controls_retained += 1;
        metrics.negative_controls_killed += killed;
        if completed {
            metrics.assisted_completed += 1;
        }
        let stage_metrics = match task.stage {
            CompletionStage::Syntax => &mut metrics.syntax,
            CompletionStage::Type => &mut metrics.typing,
            CompletionStage::Semantic => &mut metrics.semantic,
        };
        stage_metrics.eligible += 1;
        if baseline.class == task.stage.expected_class() {
            stage_metrics.baseline_completed += 1;
            metrics.baseline_completed += 1;
        }
        if completed {
            stage_metrics.assisted_completed += 1;
        }

        add_task_graph(
            &mut graph, &task, &artifacts, &baseline, &proposal, &assisted, &candidate,
        )?;
        reports.push(TaskEvaluation {
            task_id: task.id.into(),
            stage: task.stage.as_str().into(),
            baseline_class: baseline.class,
            assisted_class: assisted.class,
            completed,
            feedback_rounds: 1,
            localized_edit_count: proposal.edits.len(),
            edited_byte_extent: applied.edited_bytes,
            trusted_identities_unchanged: true,
            negative_controls_retained: 1,
            negative_controls_killed: killed,
        });
    }

    // Unknown and conflict are terminal protocol states. The deterministic
    // assistant must never turn either into a proposal or success.
    for feedback in [
        Feedback::Unknown {
            reason: "bound exhausted".into(),
            bounds_or_backend: "depth=3".into(),
        },
        Feedback::Conflict {
            raw_backend_results: BTreeMap::from([
                ("engine-a".into(), "model_checked".into()),
                ("engine-b".into(), "refuted".into()),
            ]),
        },
    ] {
        let input = AssistantInput::bounded(
            "terminal-feedback-control",
            "benchmarks/agentic/candidates/control.nmlt",
            "system Control {}\n",
            vec![ByteSpan::new(0, 18)],
            feedback.clone(),
        );
        if assistant.propose(&input).is_some() {
            match feedback {
                Feedback::Unknown { .. } => metrics.unknown_results_promoted += 1,
                Feedback::Conflict { .. } => metrics.conflict_results_promoted += 1,
                _ => {}
            }
        }
    }

    graph.validate()?;
    let readback = ArtifactGraph::from_wire(&graph.to_wire())?;
    let artifact_graph_readback_verified = readback == graph;
    if !artifact_graph_readback_verified {
        return Err("artifact graph readback changed the graph".into());
    }
    Ok(EvaluationReport {
        schema_version: "1.0.0".into(),
        suite_id: "nmlt-agentic-held-out-v1".into(),
        assistant_id: ASSISTANT_ID.into(),
        assistant_kind: "deterministic_protocol_conformance_baseline_not_llm_evidence".into(),
        checker_id: CHECKER_ID.into(),
        split: "held_out".into(),
        expected_patches_exposed: false,
        tasks: reports,
        metrics,
        graph,
        artifact_graph_readback_verified,
    })
}

fn metrics_json(metrics: &EvaluationMetrics) -> String {
    format!(
        concat!(
            "{{\"task_count\":{},\"baseline_completed\":{},\"assisted_completed\":{},",
            "\"by_stage\":{{\"syntax\":{},\"type\":{},\"semantic\":{}}},",
            "\"feedback_rounds\":{},\"localized_edit_count\":{},",
            "\"edited_byte_extent\":{},\"trusted_modification_attempts\":{},",
            "\"trusted_modification_rejections\":{},",
            "\"negative_controls_retained\":{},\"negative_controls_killed\":{},",
            "\"unknown_results_promoted\":{},\"conflict_results_promoted\":{}}}"
        ),
        metrics.task_count,
        metrics.baseline_completed,
        metrics.assisted_completed,
        stage_json(&metrics.syntax),
        stage_json(&metrics.typing),
        stage_json(&metrics.semantic),
        metrics.feedback_rounds,
        metrics.localized_edit_count,
        metrics.edited_byte_extent,
        metrics.trusted_modification_attempts,
        metrics.trusted_modification_rejections,
        metrics.negative_controls_retained,
        metrics.negative_controls_killed,
        metrics.unknown_results_promoted,
        metrics.conflict_results_promoted
    )
}

fn stage_json(metrics: &StageMetrics) -> String {
    format!(
        "{{\"eligible\":{},\"baseline_completed\":{},\"assisted_completed\":{}}}",
        metrics.eligible, metrics.baseline_completed, metrics.assisted_completed
    )
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::{check, held_out_tasks, syntax_diagnostic};
    use crate::feedback::ResultClass;

    #[test]
    fn syntax_only_fix_does_not_promote_semantic_refutation() {
        let task = held_out_tasks()
            .into_iter()
            .find(|task| task.id == "held-out-semantic-authority")
            .unwrap();
        let malformed =
            task.source
                .replacen("state armed: Bool = false;", "state armed: Bool = false", 1);
        let diagnostic = syntax_diagnostic(&malformed).unwrap();
        let mut syntax_fixed = malformed;
        syntax_fixed.insert(diagnostic.primary_span.start, ';');
        // Even after syntax completion, dispatch-before-authorize is refuted.
        assert_eq!(check(&task, &syntax_fixed).class, ResultClass::Refuted);
    }
}
