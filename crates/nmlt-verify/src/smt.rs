use crate::certificate::FiniteInvariantCertificate;
use crate::evidence::{
    BackendIdentity, EngineBinding, NormalizedClass, RawEngineResult, RawStatus, ResultScope,
};
use crate::identity::Sha256Id;
use crate::ir::{BoolExpr, FiniteSafetyVc, IrError, StateRef};
use std::fmt;

pub const SMT_LIB_VERSION: &str = "SMT-LIB-2.7";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmtStatus {
    Sat,
    Unsat,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SmtProtocolError {
    InvalidVerificationCondition(IrError),
    MissingStatus,
    AmbiguousStatus,
    UnexpectedStatus(String),
}

impl fmt::Display for SmtProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVerificationCondition(error) => write!(f, "invalid VC: {error}"),
            Self::MissingStatus => write!(f, "SMT response did not contain a status line"),
            Self::AmbiguousStatus => write!(f, "SMT response contained multiple status lines"),
            Self::UnexpectedStatus(status) => write!(f, "unexpected SMT status {status:?}"),
        }
    }
}

impl std::error::Error for SmtProtocolError {}

/// Encodes failure of initiation or consecution. `unsat` says the source property is
/// inductive, but NMLT still requires a certificate accepted by its narrow checker.
pub fn encode_inductiveness_query(vc: &FiniteSafetyVc) -> Result<String, SmtProtocolError> {
    vc.validate()
        .map_err(SmtProtocolError::InvalidVerificationCondition)?;
    let digest = vc
        .digest()
        .map_err(SmtProtocolError::InvalidVerificationCondition)?;
    let mut output = String::new();
    output.push_str("; NMLT finite-safety inductiveness query\n");
    output.push_str(&format!("; protocol: {SMT_LIB_VERSION}\n"));
    output.push_str(&format!("; vc: {digest}\n"));
    output.push_str(&format!("; model: {}\n", vc.identity.model));
    output.push_str(&format!("; claim: {}\n", vc.identity.claim));
    output.push_str(&format!("; configuration: {}\n", vc.identity.configuration));
    output.push_str("(set-logic QF_UF)\n");
    output.push_str("(set-option :produce-proofs true)\n");
    for variable in &vc.variables {
        output.push_str(&format!("(declare-fun cur_{variable} () Bool)\n"));
        output.push_str(&format!("(declare-fun next_{variable} () Bool)\n"));
    }
    output.push_str("(assert (! (or (and ");
    write_smt(&vc.initial, vc, &mut output, false);
    output.push_str(" (not ");
    write_smt(&vc.property, vc, &mut output, false);
    output.push_str(")) (and ");
    write_smt(&vc.property, vc, &mut output, false);
    output.push(' ');
    write_smt(&vc.transition, vc, &mut output, false);
    output.push_str(" (not ");
    write_smt(&vc.property, vc, &mut output, true);
    output.push_str("))) :named nmlt_inductiveness_counterexample))\n");
    output.push_str("(check-sat)\n(get-proof)\n(get-info :version)\n(exit)\n");
    Ok(output)
}

fn write_smt(expression: &BoolExpr, vc: &FiniteSafetyVc, output: &mut String, prime: bool) {
    match expression {
        BoolExpr::Const(value) => output.push_str(if *value { "true" } else { "false" }),
        BoolExpr::Var(StateRef::Current(index)) => output.push_str(&format!(
            "{}_{}",
            if prime { "next" } else { "cur" },
            vc.variables[*index]
        )),
        BoolExpr::Var(StateRef::Next(index)) => {
            output.push_str(&format!("next_{}", vc.variables[*index]));
        }
        BoolExpr::Not(inner) => {
            output.push_str("(not ");
            write_smt(inner, vc, output, prime);
            output.push(')');
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
            output.push_str(if matches!(expression, BoolExpr::And(_)) {
                "and"
            } else {
                "or"
            });
            for item in items {
                output.push(' ');
                write_smt(item, vc, output, prime);
            }
            output.push(')');
        }
        BoolExpr::Implies(left, right) => {
            output.push_str("(=> ");
            write_smt(left, vc, output, prime);
            output.push(' ');
            write_smt(right, vc, output, prime);
            output.push(')');
        }
        BoolExpr::Iff(left, right) => {
            output.push_str("(= ");
            write_smt(left, vc, output, prime);
            output.push(' ');
            write_smt(right, vc, output, prime);
            output.push(')');
        }
    }
}

pub fn parse_smt_status(stdout: &[u8]) -> Result<SmtStatus, SmtProtocolError> {
    let text = String::from_utf8_lossy(stdout);
    let statuses: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|line| matches!(*line, "sat" | "unsat" | "unknown"))
        .collect();
    match statuses.as_slice() {
        [] => Err(SmtProtocolError::MissingStatus),
        ["sat"] => Ok(SmtStatus::Sat),
        ["unsat"] => Ok(SmtStatus::Unsat),
        ["unknown"] => Ok(SmtStatus::Unknown),
        [_] => Err(SmtProtocolError::UnexpectedStatus(statuses[0].to_owned())),
        _ => Err(SmtProtocolError::AmbiguousStatus),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmtBackendReturn {
    pub engine: BackendIdentity,
    pub binding: EngineBinding,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// Optional translation into NMLT's narrow, independently checked format.
    pub certificate: Option<FiniteInvariantCertificate>,
}

pub fn normalize_smt_return(vc: &FiniteSafetyVc, returned: SmtBackendReturn) -> RawEngineResult {
    let mut raw_output = returned.stdout.clone();
    raw_output.extend_from_slice(b"\n--- stderr ---\n");
    raw_output.extend_from_slice(&returned.stderr);
    let (status, certificate) = match parse_smt_status(&returned.stdout) {
        Ok(SmtStatus::Unsat) if returned.certificate.is_some() => (
            RawStatus::Holds {
                requested_class: NormalizedClass::Proved,
            },
            returned.certificate,
        ),
        Ok(SmtStatus::Unsat) => (
            RawStatus::Unknown {
                reason: "solver returned unsat without an NMLT-checkable certificate".to_owned(),
            },
            None,
        ),
        Ok(SmtStatus::Sat) => (
            RawStatus::Unknown {
                reason: "property is not inductive; SMT sat is not a reachability counterexample"
                    .to_owned(),
            },
            None,
        ),
        Ok(SmtStatus::Unknown) => (
            RawStatus::Unknown {
                reason: "SMT solver returned unknown".to_owned(),
            },
            None,
        ),
        Err(error) => (
            RawStatus::BackendFailure {
                reason: error.to_string(),
            },
            None,
        ),
    };
    RawEngineResult {
        engine: returned.engine,
        binding: returned.binding,
        method: "smt-lib-2.7-inductiveness/1".to_owned(),
        scope: ResultScope::CompleteFinite {
            states: vc.state_count(),
        },
        status,
        certificate,
        raw_output,
    }
}

pub fn query_identity(query: &str) -> Sha256Id {
    Sha256Id::digest(query.as_bytes())
}
