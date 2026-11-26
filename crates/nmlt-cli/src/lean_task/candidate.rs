//! A bounded selection of native Lean tactics, retained as candidate data.
use super::{Result, err, name};
use nmlt_runtime::lean;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

// The context-only Lake editor retains all tactic support. Acceptance uses only
// the validated candidate's fixed imports under its unchanged process policy.
pub(super) const EDITOR_IMPORTS: &str = "import Lean.Elab.Tactic\n";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "schema", deny_unknown_fields)]
pub(super) enum Candidate {
    #[serde(rename = "nmlt-lean-proof-candidate-v1")]
    Term { task_sha256: String, proof: String },
    #[serde(rename = "nmlt-lean-proof-candidate-v2")]
    Automation {
        task_sha256: String,
        steps: Vec<Step>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tactic", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Step {
    Intro { names: Vec<String> },
    Exact { term: String },
    Apply { term: String },
    SimpOnly { lemmas: Vec<String> },
    SimpaOnly { lemmas: Vec<String> },
    Assumption {},
    Rfl {},
    Constructor {},
    Omega {},
    Grind {},
    Decide {},
}

fn names(values: &[String], locals: bool) -> Result<String> {
    if values.is_empty() || values.len() > 32 {
        return Err("tactic name list requires 1..32 names".into());
    }
    for value in values {
        if !name(value) || (locals && value.contains('.')) {
            return Err("tactic expects ordinary Lean names".into());
        }
        // Use the closed term parser's reserved-word rules as well.
        lean::render_proof_term(value).map_err(err)?;
    }
    Ok(values.join(if locals { " " } else { ", " }))
}

impl Step {
    fn module(&self) -> Option<&'static str> {
        match self {
            // These elaborators are supplied by the pinned Lean frontend.
            // Importing their implementation modules into the proof unnecessarily
            // retains those modules for independent export as well.
            Self::Intro { .. }
            | Self::Assumption {}
            | Self::Rfl {}
            | Self::Exact { .. }
            | Self::Apply { .. }
            | Self::Constructor {}
            | Self::Decide {} => None,
            Self::SimpOnly { .. } => Some("Lean.Elab.Tactic.Simp"),
            Self::SimpaOnly { .. } => Some("Lean.Elab.Tactic.Simpa"),
            Self::Omega {} => Some("Lean.Elab.Tactic.Omega"),
            Self::Grind {} => Some("Lean.Elab.Tactic.Grind"),
        }
    }

    fn render(&self) -> Result<String> {
        Ok(match self {
            Self::Intro { names: values } => format!("intro {}", names(values, true)?),
            Self::Exact { term } => {
                format!("exact {}", lean::render_proof_term(term).map_err(err)?)
            }
            Self::Apply { term } => {
                format!("apply {}", lean::render_proof_term(term).map_err(err)?)
            }
            Self::SimpOnly { lemmas } => format!("simp only [{}]", names(lemmas, false)?),
            Self::SimpaOnly { lemmas } => format!("simpa only [{}]", names(lemmas, false)?),
            Self::Assumption {} => "assumption".into(),
            Self::Rfl {} => "rfl".into(),
            Self::Constructor {} => "constructor".into(),
            Self::Omega {} => "omega".into(),
            Self::Grind {} => "grind".into(),
            Self::Decide {} => "decide".into(),
        })
    }

    fn parse(line: &str) -> Result<Self> {
        let (head, rest) = line.split_once(' ').unwrap_or((line, ""));
        let rest = rest.trim();
        let fixed = match head {
            "assumption" => Some(Self::Assumption {}),
            "rfl" => Some(Self::Rfl {}),
            "constructor" => Some(Self::Constructor {}),
            "omega" => Some(Self::Omega {}),
            "grind" => Some(Self::Grind {}),
            "decide" => Some(Self::Decide {}),
            _ => None,
        };
        if let Some(step) = fixed {
            return if rest.is_empty() {
                Ok(step)
            } else {
                Err("fixed tactic does not accept extra syntax".into())
            };
        }
        Ok(match head {
            "intro" => Self::Intro { names: rest.split_whitespace().map(str::to_owned).collect() },
            "exact" => Self::Exact { term: rest.into() },
            "apply" => Self::Apply { term: rest.into() },
            "simp" | "simpa" => {
                let body = rest.strip_prefix("only [").and_then(|s| s.strip_suffix(']'))
                    .ok_or("simplification requires an explicit 'only [lemma, ...]' list")?;
                let lemmas = body.split(',').map(|s| s.trim().to_owned()).collect();
                if head == "simp" { Self::SimpOnly { lemmas } } else { Self::SimpaOnly { lemmas } }
            }
            _ => return Err("unsupported candidate tactic; use intro, exact, apply, simp only, simpa only, assumption, rfl, constructor, omega, grind or decide".into()),
        })
    }
}

impl Candidate {
    pub fn imports(&self) -> String {
        match self {
            Self::Term { .. } => String::new(),
            Self::Automation { steps, .. } => steps
                .iter()
                .filter_map(Step::module)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(|module| format!("import {module}\n"))
                .collect(),
        }
    }

    pub fn from_file(digest: &str, source: &str, target: &super::Target) -> Result<Self> {
        let normalized;
        let source = if source.contains("\r\n") {
            normalized = source.replace("\r\n", "\n");
            normalized.as_str()
        } else {
            source
        };
        if source.starts_with("import ") {
            // Accept a familiar full Lean file only when every byte outside its
            // proof body still matches the task's generated checking wrapper.
            const MARKER: &str = "NMLTCandidateBodyPlaceholder";
            let template = super::source::proof(target, MARKER, "")?;
            let (before, after) = template.split_once(MARKER).ok_or("missing proof marker")?;
            const ROOT: &str = "import NMLTTask\n";
            let wrapper = before.strip_prefix(ROOT).ok_or("missing root import")?;
            if let Some((imports, rest)) = source
                .strip_prefix(ROOT)
                .and_then(|s| s.split_once(wrapper))
                && let Some(body) = rest.strip_suffix(after)
            {
                let candidate = Self::from_proof(digest, body)?;
                // Only adapter-generated wrappers are accepted: a term
                // wrapper, the unchanged editor draft, or this candidate's
                // exact fixed imports. No user-selected module is retained.
                if imports.is_empty() || imports == EDITOR_IMPORTS || imports == candidate.imports()
                {
                    return Ok(candidate);
                }
            }
            return Err("edited Lean file changed the task wrapper; revise the task explicitly or supply only the proof body".into());
        }
        Self::from_proof(digest, source)
    }

    pub fn from_proof(digest: &str, source: &str) -> Result<Self> {
        if source.is_empty() || source.len() > 8192 || !source.is_ascii() {
            return Err("candidate proof file requires 1..8192 ASCII bytes".into());
        }
        let source = source.trim();
        let candidate =
            if source == "by" || source.starts_with("by\n") || source.starts_with("by\r\n") {
                let steps = source
                    .lines()
                    .skip(1)
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(Step::parse)
                    .collect::<Result<Vec<_>>>()?;
                Self::Automation {
                    task_sha256: digest.into(),
                    steps,
                }
            } else {
                Self::Term {
                    task_sha256: digest.into(),
                    proof: source.into(),
                }
            };
        candidate.validate(digest)?;
        Ok(candidate)
    }

    pub fn validate(&self, digest: &str) -> Result<String> {
        let (Self::Term { task_sha256, .. } | Self::Automation { task_sha256, .. }) = self;
        if task_sha256 != digest {
            return Err("candidate does not refer to the selected task hash".into());
        }
        match self {
            Self::Term { proof, .. } => lean::render_proof_term(proof).map_err(err),
            Self::Automation { steps, .. } => {
                if steps.is_empty() || steps.len() > 32 {
                    return Err("candidate automation requires 1..32 steps".into());
                }
                let mut source = String::from("by\n");
                for step in steps {
                    source.push_str("  ");
                    source.push_str(&step.render()?);
                    source.push('\n');
                }
                if source.len() > 8192 {
                    return Err("candidate automation exceeds 8192 rendered bytes".into());
                }
                Ok(source)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_automation_is_explicit_bounded_and_cannot_change_commands() {
        let candidate =
            Candidate::from_proof("pin", "by\n  intro n\n  simpa only [Nat.zero_add]\n").unwrap();
        assert_eq!(
            candidate.validate("pin").unwrap(),
            "by\n  intro n\n  simpa only [Nat.zero_add]\n"
        );
        assert!(candidate.validate("other").is_err());
        for source in [
            "by\n sorry",
            "by\n run_tac trivial",
            "by\n rfl; sorry",
            "by\n exact True.intro\naxiom bad : False",
            "by\n simp",
            "by\n intro by",
            "by\n simp only [x]\nset_option maxRecDepth 1000",
            "by\n exact (by sorry)",
        ] {
            assert!(Candidate::from_proof("pin", source).is_err(), "{source}");
        }
        assert!(Candidate::from_proof("pin", &format!("by\n{}", "rfl\n".repeat(33))).is_err());
        assert!(serde_json::from_str::<Candidate>(r#"{"schema":"nmlt-lean-proof-candidate-v2","task_sha256":"pin","steps":[{"tactic":"rfl","statement":"True"}]}"#).is_err());
        assert!(serde_json::from_str::<Candidate>(r#"{"schema":"nmlt-lean-proof-candidate-v2","task_sha256":"pin","steps":[],"statement":"True"}"#).is_err());
    }

    #[test]
    fn imports_are_fixed_by_steps_and_full_wrappers_cannot_add_modules() {
        let target = super::super::Target {
            declaration: "Example.goal".into(),
            module: "Example".into(),
            type_repr: "True".into(),
            type_lean: "True".into(),
            level_params: vec![],
            type_references: vec![],
        };
        for body in [
            "by\n intro n\n constructor\n exact True.intro\n exact True.intro\n",
            "by\n grind\n",
            "by\n decide\n",
            "by\n simpa only [Nat.zero_add]\n",
        ] {
            let candidate = Candidate::from_proof("pin", body).unwrap();
            let imports = candidate.imports();
            assert!(
                !imports
                    .lines()
                    .any(|line| line == "import Lean.Elab.Tactic")
            );
            let rendered = candidate.validate("pin").unwrap();
            for header in [imports.as_str(), EDITOR_IMPORTS, ""] {
                let source = super::super::source::proof(&target, &rendered, header).unwrap();
                let extracted = Candidate::from_file("pin", &source, &target).unwrap();
                assert_eq!(extracted.validate("pin").unwrap(), rendered);
                assert_eq!(extracted.imports(), imports);
                let crlf_source = source.replace('\n', "\r\n");
                let extracted_crlf = Candidate::from_file("pin", &crlf_source, &target).unwrap();
                assert_eq!(extracted_crlf.validate("pin").unwrap(), rendered);
                assert_eq!(extracted_crlf.imports(), imports);
                assert!(
                    Candidate::from_file(
                        "pin",
                        &crlf_source
                            .replace("import NMLTTask\r\n", "import NMLTTask\r\nimport Evil\r\n"),
                        &target
                    )
                    .is_err()
                );
                assert!(
                    Candidate::from_file(
                        "pin",
                        &source.replace("NMLTTask.target", "True"),
                        &target
                    )
                    .is_err()
                );
                assert!(
                    Candidate::from_file(
                        "pin",
                        &source.replace("import NMLTTask\n", "import NMLTTask\nimport Evil\n"),
                        &target
                    )
                    .is_err()
                );
            }
        }
        for source in ["by\n grind only [True.intro]\n", "by\n decide +native\n"] {
            assert!(Candidate::from_proof("pin", source).is_err());
        }
        let exact = Candidate::from_proof("pin", "by\n exact True.intro\n").unwrap();
        assert_eq!(exact.imports(), "");
        assert_eq!(
            Candidate::from_proof("pin", "True.intro")
                .unwrap()
                .imports(),
            ""
        );
    }
}
