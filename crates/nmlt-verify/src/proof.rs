use crate::certificate::FiniteInvariantCertificate;
use crate::evidence::{
    BackendIdentity, EngineBinding, NormalizedClass, RawEngineResult, RawStatus, ResultScope,
};
use crate::identity::Sha256Id;
use crate::ir::{BoolExpr, FiniteSafetyVc, IrError, StateRef};

pub const PROOF_ASSISTANT_PROTOCOL: &str = "nmlt-proof-assistant/1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProofAssistant {
    Lean4 { version: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofAssistantExport {
    pub protocol: String,
    pub assistant: ProofAssistant,
    pub vc_digest: Sha256Id,
    pub model_id: Sha256Id,
    pub claim_id: Sha256Id,
    pub configuration_id: Sha256Id,
    /// Generated definitions and an obligation, never a proof or accepted result.
    pub source: String,
}

/// Emits a closed finite Boolean VC and an inductiveness proposition for Lean 4.
/// The generated file deliberately contains neither `axiom` nor `sorry`: a backend
/// may solve the proposition, but acceptance still requires the narrow return below.
pub fn export_lean4_inductiveness(
    vc: &FiniteSafetyVc,
    lean_version: impl Into<String>,
) -> Result<ProofAssistantExport, IrError> {
    vc.validate()?;
    let vc_digest = vc.digest()?;
    let mut source = format!(
        "/- Generated NMLT proof request.\nVC: {vc_digest}\nModel: {}\nClaim: {}\nConfiguration: {}\n-/\n\n",
        vc.identity.model, vc.identity.claim, vc.identity.configuration
    );
    source.push_str(&format!(
        "def nmltStateCount : Nat := {}\n",
        vc.state_count()
    ));
    source.push_str("def nmltInitial (current : Nat) : Bool := ");
    write_lean(&vc.initial, &mut source, false);
    source.push('\n');
    source.push_str("def nmltTransition (current next : Nat) : Bool := ");
    write_lean(&vc.transition, &mut source, false);
    source.push('\n');
    source.push_str("def nmltProperty (current : Nat) : Bool := ");
    write_lean(&vc.property, &mut source, false);
    source.push_str("\n\n");
    source.push_str(
        "def NMLTInductiveObligation : Prop :=\n  (∀ current, current < nmltStateCount → nmltInitial current = true → nmltProperty current = true) ∧\n  (∀ current next, current < nmltStateCount → next < nmltStateCount →\n    nmltProperty current = true → nmltTransition current next = true →\n    nmltProperty next = true)\n",
    );
    Ok(ProofAssistantExport {
        protocol: PROOF_ASSISTANT_PROTOCOL.to_owned(),
        assistant: ProofAssistant::Lean4 {
            version: lean_version.into(),
        },
        vc_digest,
        model_id: vc.identity.model.clone(),
        claim_id: vc.identity.claim.clone(),
        configuration_id: vc.identity.configuration.clone(),
        source,
    })
}

fn write_lean(expression: &BoolExpr, output: &mut String, prime: bool) {
    match expression {
        BoolExpr::Const(value) => output.push_str(if *value { "true" } else { "false" }),
        BoolExpr::Var(StateRef::Current(index)) => output.push_str(&format!(
            "Nat.testBit {} {}",
            if prime { "next" } else { "current" },
            index
        )),
        BoolExpr::Var(StateRef::Next(index)) => {
            output.push_str(&format!("Nat.testBit next {index}"));
        }
        BoolExpr::Not(inner) => {
            output.push_str("(!(");
            write_lean(inner, output, prime);
            output.push_str("))");
        }
        BoolExpr::And(items) | BoolExpr::Or(items) => {
            if items.is_empty() {
                output.push_str(if matches!(expression, BoolExpr::And(_)) {
                    "true"
                } else {
                    "false"
                });
                return;
            }
            output.push('(');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push_str(if matches!(expression, BoolExpr::And(_)) {
                        " && "
                    } else {
                        " || "
                    });
                }
                write_lean(item, output, prime);
            }
            output.push(')');
        }
        BoolExpr::Implies(left, right) => {
            output.push_str("((!(");
            write_lean(left, output, prime);
            output.push_str(")) || ");
            write_lean(right, output, prime);
            output.push(')');
        }
        BoolExpr::Iff(left, right) => {
            output.push('(');
            write_lean(left, output, prime);
            output.push_str(" == ");
            write_lean(right, output, prime);
            output.push(')');
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofAssistantReturn {
    pub engine: BackendIdentity,
    pub binding: EngineBinding,
    pub certificate: Option<FiniteInvariantCertificate>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Adapts proof-assistant output without trusting prose, exit status, or an opaque
/// theorem name. Only a returned certificate accepted later by `normalize_result`
/// can become `proved`; absence remains `unknown`.
pub fn normalize_proof_assistant_return(
    vc: &FiniteSafetyVc,
    returned: ProofAssistantReturn,
) -> RawEngineResult {
    let mut raw_output = returned.stdout;
    raw_output.extend_from_slice(b"\n--- stderr ---\n");
    raw_output.extend_from_slice(&returned.stderr);
    let has_certificate = returned.certificate.is_some();
    RawEngineResult {
        engine: returned.engine,
        binding: returned.binding,
        method: PROOF_ASSISTANT_PROTOCOL.to_owned(),
        scope: ResultScope::CompleteFinite {
            states: vc.state_count(),
        },
        status: if has_certificate {
            RawStatus::Holds {
                requested_class: NormalizedClass::Proved,
            }
        } else {
            RawStatus::Unknown {
                reason: "proof backend returned no NMLT-checkable certificate".to_owned(),
            }
        },
        certificate: returned.certificate,
        raw_output,
    }
}
