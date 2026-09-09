//! Pinned Init proof terms and legacy templates under an empty-axiom policy.
//! This is captured checker-process evidence, not an independent proof kernel.
use crate::{
    Adapter, Dispatch, Error, RESPONSE_SCHEMA, Response, ResponseOutcome, Value, ValueType,
    process, sha256,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const REPORT: &str = "'NMLTJob.checked_target' does not depend on any axioms";
const CONTRACT: &str = "nmlt-lean-zero-add-v1; stdin; threads=1; memory=512; heartbeats=200000; accepted-strategies=existing-lemma|induction; exact-empty-axiom-report";
const TERM_CONTRACT: &str =
    "nmlt-init-terms-v1; closed-term-grammar; statement:Prop; empty-axioms; exact-bin-lib-tree";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Candidate {
    Template { strategy: Strategy },
    Terms { statement: String, proof: String },
}
impl Candidate {
    pub fn source(&self) -> Result<String, Error> {
        match self {
            Self::Template { strategy } => Ok(source(*strategy)),
            Self::Terms { statement, proof } => {
                let statement = crate::lean_term::render(statement)?;
                let proof = crate::lean_term::render(proof)?;
                Ok(format!(
                    "import Init\nset_option autoImplicit false\nset_option maxHeartbeats 200000\ndef NMLTJob.target : Prop := {statement}\ntheorem NMLTJob.checked_target : NMLTJob.target := {proof}\n#print axioms NMLTJob.checked_target\n"
                ))
            }
        }
    }
    fn can_accept(&self) -> bool {
        !matches!(
            self,
            Self::Template {
                strategy: Strategy::WrongTerm | Strategy::Admitted
            }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    WrongTerm,
    ExistingLemma,
    Induction,
    Admitted,
}

pub fn source(strategy: Strategy) -> String {
    let proof = match strategy {
        Strategy::WrongTerm => "  intro n\n  exact n",
        Strategy::ExistingLemma => "  intro n\n  exact Nat.zero_add n",
        Strategy::Induction => {
            "  intro n\n  induction n with\n  | zero => rfl\n  | succ n ih => exact congrArg Nat.succ ih"
        }
        Strategy::Admitted => "  sorry",
    };
    format!(
        "import Init\nset_option autoImplicit false\nset_option maxHeartbeats 200000\ntheorem NMLTJob.checked_target : (forall n : Nat, 0 + n = n) := by\n{proof}\n#print axioms NMLTJob.checked_target\n"
    )
}
fn contract_digest() -> String {
    sha256(
        format!(
            "{CONTRACT}\n{TERM_CONTRACT}\n{}\n{}{}{}{}",
            process::CONTRACT,
            source(Strategy::WrongTerm),
            source(Strategy::ExistingLemma),
            source(Strategy::Induction),
            source(Strategy::Admitted)
        )
        .as_bytes(),
    )
}
pub fn version() -> &'static str {
    include_str!("../../../mechanization/lean/lean-toolchain")
        .trim()
        .trim_start_matches('v')
}
pub fn adapter() -> Adapter {
    Adapter {
        name: "lean-init".into(),
        version: 1,
        input_type: ValueType::Text,
        output_type: ValueType::Text,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub executable_sha256: String,
    pub installation_sha256: String,
    pub version: String,
    pub contract_sha256: String,
}

pub struct Toolchain {
    executable: PathBuf,
    identity: Identity,
    files: Vec<crate::identity::FileIdentity>,
}
impl Toolchain {
    /// Pin every bin/lib file, including imported artifacts and dynamic libraries.
    /// The OS loader and a concurrently mutable filesystem remain trusted.
    pub fn open(executable: &Path) -> Result<Self, Error> {
        let executable = executable.canonicalize()?;
        let digest = executable_digest(&executable)?;
        let files = installation(&executable)?;
        let manifest = serde_json::to_vec(&files)?;
        if manifest.len() > 24 * 1024 * 1024 {
            return Err(Error("Lean installation manifest exceeds 24 MiB".into()));
        }
        let installation_sha256 = sha256(&manifest);
        let mut command = clean_command(&executable)?;
        command.arg("--version");
        let mut child = process::Process::start(command, vec![], Duration::from_secs(5))
            .map_err(|e| Error(format!("Lean version probe failed: {e:?}")))?;
        let output = child
            .wait()
            .as_ref()
            .map_err(|e| Error(format!("Lean version probe failed: {e:?}")))?;
        let reported = std::str::from_utf8(&output.stdout)
            .map_err(|_| Error("Lean version is not UTF-8".into()))?
            .trim();
        if output.exit_code != Some(0)
            || !output.stderr.is_empty()
            || !reported.starts_with(&format!("Lean (version {},", version()))
        {
            return Err(Error(format!(
                "Lean version mismatch; require {}",
                version()
            )));
        }
        if executable_digest(&executable)? != digest {
            return Err(Error("Lean executable changed during preflight".into()));
        }
        Ok(Self {
            executable,
            files,
            identity: Identity {
                executable_sha256: digest,
                installation_sha256,
                version: reported.into(),
                contract_sha256: contract_digest(),
            },
        })
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn executable(&self) -> &Path {
        &self.executable
    }
    pub fn files(&self) -> &[crate::identity::FileIdentity] {
        &self.files
    }
    pub(crate) fn prepare_candidate(
        &self,
        candidate: Candidate,
    ) -> Result<(Request, Command, Vec<u8>), Error> {
        let request = Request::new(self.identity.clone(), candidate)?;
        if executable_digest(&self.executable)? != self.identity.executable_sha256 {
            return Err(Error("Lean executable changed after preflight".into()));
        }
        if installation(&self.executable)? != self.files {
            return Err(Error(
                "Lean installation dependencies changed after preflight".into(),
            ));
        }
        let source = request.candidate.source()?;
        let mut command = clean_command(&self.executable)?;
        command.args(["--stdin", "--threads=1", "--memory=512"]);
        Ok((request, command, source.into_bytes()))
    }
}
fn installation(executable: &Path) -> Result<Vec<crate::identity::FileIdentity>, Error> {
    let bin = executable
        .parent()
        .ok_or_else(|| Error("Lean bin directory missing".into()))?;
    if bin.file_name().is_none_or(|n| n != "bin") {
        return Err(Error(
            "Lean must be the direct executable in its installation's bin directory".into(),
        ));
    }
    crate::identity::tree(
        bin.parent()
            .ok_or_else(|| Error("Lean installation missing".into()))?,
        &["bin", "lib"],
    )
}
pub(crate) fn executable_digest(path: &Path) -> Result<String, Error> {
    crate::identity::file(path, 256 * 1024 * 1024).map(|(_, digest)| digest)
}
fn clean_command(executable: &Path) -> Result<Command, Error> {
    let mut command = Command::new(executable);
    command.env_clear().current_dir(
        executable
            .parent()
            .ok_or_else(|| Error("Lean executable has no directory".into()))?,
    );
    #[cfg(windows)]
    for name in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    Ok(command)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub toolchain: Identity,
    pub candidate: Candidate,
    pub source_sha256: String,
}
impl Request {
    pub fn new(toolchain: Identity, candidate: Candidate) -> Result<Self, Error> {
        let source_sha256 = sha256(candidate.source()?.as_bytes());
        let request = Self {
            schema: "nmlt-lean-request-v2".into(),
            toolchain,
            candidate,
            source_sha256,
        };
        request.validate()?;
        Ok(request)
    }
    pub fn input(&self) -> Result<Value, Error> {
        self.validate()?;
        let json = serde_json::to_string(self)?;
        if json.len() > 4096 {
            return Err(Error(
                "Lean request exceeds 4096-byte protocol bound".into(),
            ));
        }
        Ok(Value::Text(json))
    }
    fn validate(&self) -> Result<(), Error> {
        if self.schema != "nmlt-lean-request-v2"
            || !crate::valid_digest(&self.toolchain.executable_sha256)
            || !crate::valid_digest(&self.toolchain.installation_sha256)
            || !self
                .toolchain
                .version
                .starts_with(&format!("Lean (version {},", version()))
            || self.toolchain.version.len() > 1024
            || self.toolchain.contract_sha256 != contract_digest()
            || self.source_sha256 != sha256(self.candidate.source()?.as_bytes())
        {
            return Err(Error(
                "Lean request identity or generated source mismatch".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Accepted,
    Rejected,
    PolicyFailure,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub request: Request,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub verdict: Verdict,
}
fn verdict(
    candidate: &Candidate,
    code: Option<i32>,
    stdout: &str,
    stderr: &str,
) -> Result<Verdict, Error> {
    if code == Some(0) {
        Ok(
            if candidate.can_accept() && stderr.is_empty() && stdout.trim() == REPORT {
                Verdict::Accepted
            } else {
                Verdict::PolicyFailure
            },
        )
    } else if code == Some(1) && (stdout.contains("error:") || stderr.contains("error:")) {
        Ok(Verdict::Rejected)
    } else {
        Err(Error(
            "Lean process did not produce a supported checker outcome".into(),
        ))
    }
}
pub fn evidence(request: Request, output: &process::Output) -> Result<Evidence, Error> {
    request.validate()?;
    if output.stdout.len() > process::PIPE_BYTES || output.stderr.len() > process::PIPE_BYTES {
        return Err(Error("Lean diagnostic output exceeds bound".into()));
    }
    let stdout = String::from_utf8(output.stdout.clone())
        .map_err(|_| Error("Lean stdout is not UTF-8".into()))?;
    let stderr = String::from_utf8(output.stderr.clone())
        .map_err(|_| Error("Lean stderr is not UTF-8".into()))?;
    let verdict = verdict(&request.candidate, output.exit_code, &stdout, &stderr)?;
    Ok(Evidence {
        request,
        exit_code: output.exit_code,
        stdout,
        stderr,
        verdict,
    })
}

/// Check captured response consistency. Does not launch Lean or authenticate
/// the output producer, and is deliberately distinct from a fresh Lean check.
pub fn validate(dispatch: &Dispatch, evidence: &Evidence) -> Result<Response, Error> {
    evidence.request.validate()?;
    if dispatch.binding.adapter != adapter()
        || dispatch.input != evidence.request.input()?
        || dispatch.binding.input_sha256 != sha256(&serde_json::to_vec(&dispatch.input)?)
        || evidence.stdout.len() > process::PIPE_BYTES
        || evidence.stderr.len() > process::PIPE_BYTES
        || evidence.verdict
            != verdict(
                &evidence.request.candidate,
                evidence.exit_code,
                &evidence.stdout,
                &evidence.stderr,
            )?
    {
        return Err(Error(
            "Lean evidence binding, bounds, or verdict mismatch".into(),
        ));
    }
    let outcome = match evidence.verdict {
        Verdict::Accepted => ResponseOutcome::Completed {
            value: Value::Text(evidence.request.source_sha256.clone()),
        },
        Verdict::Rejected => ResponseOutcome::Failed {
            message:
                "Lean rejected the submitted candidate; no conclusion about the target's truth"
                    .into(),
        },
        Verdict::PolicyFailure => ResponseOutcome::Failed {
            message: "Lean output failed the exact empty-axiom policy".into(),
        },
    };
    Ok(Response {
        schema: RESPONSE_SCHEMA.into(),
        binding: dispatch.binding.clone(),
        outcome,
        observed_work: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(strategy: Strategy) -> (Request, Dispatch) {
        let request = Request {
            schema: "nmlt-lean-request-v2".into(),
            toolchain: Identity {
                executable_sha256: sha256(b"test checker"),
                installation_sha256: sha256(b"test installation"),
                version: format!("Lean (version {}, test)", version()),
                contract_sha256: contract_digest(),
            },
            candidate: Candidate::Template { strategy },
            source_sha256: sha256(source(strategy).as_bytes()),
        };
        let input = request.input().unwrap();
        let dispatch = Dispatch {
            binding: crate::Binding {
                attempt: crate::AttemptId {
                    run: "test".into(),
                    task: "proof".into(),
                    slot: 0,
                    generation: 1,
                },
                adapter: adapter(),
                dispatched_by: "test".into(),
                context_sha256: sha256(b"context"),
                input_sha256: sha256(&serde_json::to_vec(&input).unwrap()),
            },
            input,
        };
        (request, dispatch)
    }
    #[test]
    fn general_terms_bind_target_candidate_and_installation() {
        let (template, mut dispatch) = fixture(Strategy::ExistingLemma);
        let request = Request::new(
            template.toolchain,
            Candidate::Terms {
                statement: "forall n : Nat, n + 0 = n".into(),
                proof: "fun n => Nat.add_zero n".into(),
            },
        )
        .unwrap();
        dispatch.input = request.input().unwrap();
        dispatch.binding.input_sha256 = sha256(&serde_json::to_vec(&dispatch.input).unwrap());
        let accepted = evidence(
            request,
            &process::Output {
                exit_code: Some(0),
                stdout: REPORT.as_bytes().to_vec(),
                stderr: vec![],
            },
        )
        .unwrap();
        assert_eq!(accepted.verdict, Verdict::Accepted);
        assert!(validate(&dispatch, &accepted).is_ok());
        for field in ["statement", "proof", "installation"] {
            let mut changed = accepted.clone();
            match field {
                "statement" => {
                    let Candidate::Terms { statement, .. } = &mut changed.request.candidate else {
                        unreachable!()
                    };
                    *statement = "False".into();
                }
                "proof" => {
                    let Candidate::Terms { proof, .. } = &mut changed.request.candidate else {
                        unreachable!()
                    };
                    *proof = "False.elim".into();
                }
                _ => changed.request.toolchain.installation_sha256 = sha256(b"changed library"),
            }
            assert!(validate(&dispatch, &changed).is_err());
        }
        let policy = evidence(
            accepted.request,
            &process::Output {
                exit_code: Some(0),
                stdout: b"'NMLTJob.checked_target' depends on axioms: [propext]".to_vec(),
                stderr: vec![],
            },
        )
        .unwrap();
        assert_eq!(policy.verdict, Verdict::PolicyFailure);
    }

    #[test]
    fn exact_axiom_report_is_required_for_acceptance() {
        let (request, dispatch) = fixture(Strategy::ExistingLemma);
        for (stdout, stderr, expected) in [
            (format!("{REPORT}\n"), "", Verdict::Accepted),
            (
                format!("warning: declaration uses sorry\n{REPORT}\n"),
                "",
                Verdict::PolicyFailure,
            ),
            (
                "'NMLTJob.checked_target' depends on axioms: [sorryAx]".into(),
                "",
                Verdict::PolicyFailure,
            ),
            (
                "'Other.target' does not depend on any axioms".into(),
                "",
                Verdict::PolicyFailure,
            ),
            (REPORT.into(), "warning", Verdict::PolicyFailure),
            (String::new(), "", Verdict::PolicyFailure),
        ] {
            let evidence = evidence(
                request.clone(),
                &process::Output {
                    exit_code: Some(0),
                    stdout: stdout.into_bytes(),
                    stderr: stderr.as_bytes().to_vec(),
                },
            )
            .unwrap();
            assert_eq!(evidence.verdict, expected);
            let response = validate(&dispatch, &evidence).unwrap();
            assert_eq!(response.observed_work, None);
            assert_eq!(
                matches!(response.outcome, ResponseOutcome::Completed { .. }),
                expected == Verdict::Accepted
            );
        }
    }
    #[test]
    fn rejection_is_not_acceptance_or_a_refutation() {
        let (request, dispatch) = fixture(Strategy::WrongTerm);
        let evidence = evidence(
            request,
            &process::Output {
                exit_code: Some(1),
                stdout: b"<stdin>: error: type mismatch".to_vec(),
                stderr: vec![],
            },
        )
        .unwrap();
        assert_eq!(evidence.verdict, Verdict::Rejected);
        assert!(
            matches!(validate(&dispatch, &evidence).unwrap().outcome, ResponseOutcome::Failed {message} if message.contains("no conclusion"))
        );
    }
    #[test]
    fn output_identity_verdict_and_bounds_cannot_be_mutated() {
        let (request, dispatch) = fixture(Strategy::ExistingLemma);
        let accepted = evidence(
            request,
            &process::Output {
                exit_code: Some(0),
                stdout: REPORT.as_bytes().to_vec(),
                stderr: vec![],
            },
        )
        .unwrap();
        let mut changed = accepted.clone();
        changed.request.candidate = Candidate::Template {
            strategy: Strategy::Admitted,
        };
        assert!(validate(&dispatch, &changed).is_err());
        let mut changed = accepted.clone();
        changed.request.source_sha256 = sha256(b"other");
        assert!(validate(&dispatch, &changed).is_err());
        let mut changed = accepted.clone();
        changed.request.toolchain.version = "Lean (version 0.0.0, other)".into();
        assert!(validate(&dispatch, &changed).is_err());
        let mut changed = accepted.clone();
        changed.verdict = Verdict::Rejected;
        assert!(validate(&dispatch, &changed).is_err());
        let mut changed = accepted.clone();
        changed.stdout = "x".repeat(process::PIPE_BYTES + 1);
        assert!(validate(&dispatch, &changed).is_err());
        let mut wrong_dispatch = dispatch.clone();
        wrong_dispatch.binding.input_sha256 = sha256(b"wrong");
        assert!(validate(&wrong_dispatch, &accepted).is_err());
        let mut wrong_dispatch = dispatch;
        wrong_dispatch.binding.adapter = crate::worker::adapter();
        assert!(validate(&wrong_dispatch, &accepted).is_err());
    }
    #[test]
    fn unexpected_process_exits_or_non_utf8_are_host_failures() {
        let (request, _) = fixture(Strategy::ExistingLemma);
        for output in [
            process::Output {
                exit_code: None,
                stdout: vec![],
                stderr: vec![],
            },
            process::Output {
                exit_code: Some(2),
                stdout: REPORT.as_bytes().to_vec(),
                stderr: vec![],
            },
            process::Output {
                exit_code: Some(0),
                stdout: vec![255],
                stderr: vec![],
            },
        ] {
            assert!(evidence(request.clone(), &output).is_err());
        }
    }
    #[test]
    fn negative_control_templates_cannot_be_promoted_by_an_empty_report() {
        for strategy in [Strategy::WrongTerm, Strategy::Admitted] {
            let (request, dispatch) = fixture(strategy);
            let evidence = evidence(
                request,
                &process::Output {
                    exit_code: Some(0),
                    stdout: REPORT.as_bytes().to_vec(),
                    stderr: vec![],
                },
            )
            .unwrap();
            assert_eq!(evidence.verdict, Verdict::PolicyFailure);
            assert!(matches!(
                validate(&dispatch, &evidence).unwrap().outcome,
                ResponseOutcome::Failed { .. }
            ));
        }
    }
}
