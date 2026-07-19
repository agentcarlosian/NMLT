use crate::identity::Sha256Id;
use crate::ir::{FiniteSafetyVc, IrError, eval};
use std::collections::BTreeSet;
use std::fmt;

pub const FINITE_INVARIANT_CERTIFICATE_VERSION: &str = "nmlt-finite-invariant/1";

/// A deliberately narrow proof object for a finite Boolean transition system.
///
/// The state list denotes an inductive set. The checker, not the producing engine,
/// establishes that it contains every initial state, implies the claim, and is
/// closed under the exact transition relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiniteInvariantCertificate {
    pub format: String,
    pub vc_digest: Sha256Id,
    pub model_id: Sha256Id,
    pub claim_id: Sha256Id,
    pub configuration_id: Sha256Id,
    pub invariant_states: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertificateError {
    InvalidVerificationCondition(IrError),
    UnsupportedFormat(String),
    StaleVerificationCondition {
        expected: Sha256Id,
        actual: Sha256Id,
    },
    StaleModel,
    StaleClaim,
    StaleConfiguration,
    StateOutOfRange(u64),
    StatesNotStrictlySorted,
    InitialStateMissing(u64),
    PropertyFalse(u64),
    NotClosed {
        from: u64,
        to: u64,
    },
}

impl fmt::Display for CertificateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVerificationCondition(error) => write!(f, "invalid VC: {error}"),
            Self::UnsupportedFormat(format) => {
                write!(f, "unsupported certificate format {format:?}")
            }
            Self::StaleVerificationCondition { expected, actual } => write!(
                f,
                "certificate is bound to {actual}, but the active VC is {expected}"
            ),
            Self::StaleModel => write!(f, "certificate model identity does not match the VC"),
            Self::StaleClaim => write!(f, "certificate claim identity does not match the VC"),
            Self::StaleConfiguration => {
                write!(
                    f,
                    "certificate configuration identity does not match the VC"
                )
            }
            Self::StateOutOfRange(state) => write!(f, "certificate state {state} is out of range"),
            Self::StatesNotStrictlySorted => {
                write!(f, "certificate states must be strictly sorted and unique")
            }
            Self::InitialStateMissing(state) => {
                write!(f, "initial state {state} is absent from the invariant")
            }
            Self::PropertyFalse(state) => {
                write!(f, "invariant state {state} does not satisfy the property")
            }
            Self::NotClosed { from, to } => write!(
                f,
                "invariant is not transition-closed: state {from} reaches omitted state {to}"
            ),
        }
    }
}

impl std::error::Error for CertificateError {}

/// Independently checks a finite invariant certificate. This code shares the VC
/// definition and Boolean evaluator, but no search or result-classification logic
/// with either producing engine.
pub fn check_finite_invariant_certificate(
    vc: &FiniteSafetyVc,
    certificate: &FiniteInvariantCertificate,
) -> Result<(), CertificateError> {
    vc.validate()
        .map_err(CertificateError::InvalidVerificationCondition)?;
    if certificate.format != FINITE_INVARIANT_CERTIFICATE_VERSION {
        return Err(CertificateError::UnsupportedFormat(
            certificate.format.clone(),
        ));
    }
    let expected = vc
        .digest()
        .map_err(CertificateError::InvalidVerificationCondition)?;
    if certificate.vc_digest != expected {
        return Err(CertificateError::StaleVerificationCondition {
            expected,
            actual: certificate.vc_digest.clone(),
        });
    }
    if certificate.model_id != vc.identity.model {
        return Err(CertificateError::StaleModel);
    }
    if certificate.claim_id != vc.identity.claim {
        return Err(CertificateError::StaleClaim);
    }
    if certificate.configuration_id != vc.identity.configuration {
        return Err(CertificateError::StaleConfiguration);
    }

    let state_count = vc.state_count() as u64;
    let mut previous = None;
    for &state in &certificate.invariant_states {
        if state >= state_count {
            return Err(CertificateError::StateOutOfRange(state));
        }
        if previous.is_some_and(|previous| previous >= state) {
            return Err(CertificateError::StatesNotStrictlySorted);
        }
        previous = Some(state);
    }
    let invariant: BTreeSet<usize> = certificate
        .invariant_states
        .iter()
        .map(|&state| state as usize)
        .collect();

    for state in 0..vc.state_count() {
        if eval(&vc.initial, state, 0) && !invariant.contains(&state) {
            return Err(CertificateError::InitialStateMissing(state as u64));
        }
    }
    for &state in &invariant {
        if !eval(&vc.property, state, 0) {
            return Err(CertificateError::PropertyFalse(state as u64));
        }
        for next in 0..vc.state_count() {
            if eval(&vc.transition, state, next) && !invariant.contains(&next) {
                return Err(CertificateError::NotClosed {
                    from: state as u64,
                    to: next as u64,
                });
            }
        }
    }
    Ok(())
}
