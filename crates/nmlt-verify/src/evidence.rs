use crate::certificate::{FiniteInvariantCertificate, check_finite_invariant_certificate};
use crate::identity::Sha256Id;
use crate::ir::{FiniteSafetyVc, eval};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TrustedComponent {
    pub name: String,
    pub version: String,
    pub digest: Sha256Id,
    pub role: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendIdentity {
    pub name: String,
    pub version: String,
    pub build_digest: Sha256Id,
    pub protocol: String,
    pub trusted_components: Vec<TrustedComponent>,
}

impl BackendIdentity {
    pub fn validate_exact(&self) -> Result<(), String> {
        for (field, value) in [
            ("backend name", self.name.as_str()),
            ("backend version", self.version.as_str()),
            ("backend protocol", self.protocol.as_str()),
        ] {
            if value.trim().is_empty() || value.eq_ignore_ascii_case("unknown") {
                return Err(format!("{field} must be exact, not {value:?}"));
            }
        }
        if self.trusted_components.is_empty() {
            return Err("at least one exact trusted component is required".to_owned());
        }
        for component in &self.trusted_components {
            if component.name.trim().is_empty()
                || component.version.trim().is_empty()
                || component.version.eq_ignore_ascii_case("unknown")
                || component.role.trim().is_empty()
            {
                return Err(format!(
                    "trusted component {:?} lacks an exact name, version, or role",
                    component.name
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineBinding {
    pub vc_digest: Sha256Id,
    pub model_id: Sha256Id,
    pub claim_id: Sha256Id,
    pub configuration_id: Sha256Id,
}

impl EngineBinding {
    pub fn for_vc(vc: &FiniteSafetyVc) -> Result<Self, String> {
        Ok(Self {
            vc_digest: vc.digest().map_err(|error| error.to_string())?,
            model_id: vc.identity.model.clone(),
            claim_id: vc.identity.claim.clone(),
            configuration_id: vc.identity.configuration.clone(),
        })
    }

    fn mismatch(&self, expected: &Self) -> Option<&'static str> {
        if self.vc_digest != expected.vc_digest {
            Some("verification-condition digest")
        } else if self.model_id != expected.model_id {
            Some("model identity")
        } else if self.claim_id != expected.claim_id {
            Some("claim identity")
        } else if self.configuration_id != expected.configuration_id {
            Some("configuration identity")
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResultScope {
    /// Complete reasoning over the exact finite domain and transition relation.
    CompleteFinite { states: usize },
    /// Search terminated at declared bounds. This can never support `proved`.
    Bounded { max_depth: usize, max_states: usize },
    /// Concrete generated executions only.
    Sampled {
        seed: u64,
        cases: usize,
        max_steps: usize,
    },
}

impl ResultScope {
    pub fn is_bounded(&self) -> bool {
        !matches!(self, Self::CompleteFinite { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalizedClass {
    Proved,
    ModelChecked,
    Tested,
    Refuted,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawStatus {
    Holds { requested_class: NormalizedClass },
    Refuted { witness_states: Vec<u64> },
    Unknown { reason: String },
    BackendFailure { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawEngineResult {
    pub engine: BackendIdentity,
    pub binding: EngineBinding,
    pub method: String,
    pub scope: ResultScope,
    pub status: RawStatus,
    pub certificate: Option<FiniteInvariantCertificate>,
    /// Exact backend bytes are retained even when normalization rejects them.
    pub raw_output: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedResult {
    pub engine_name: String,
    pub class: NormalizedClass,
    pub scope: ResultScope,
    pub reason: String,
    pub certificate_accepted: bool,
}

pub fn normalize_result(vc: &FiniteSafetyVc, raw: &RawEngineResult) -> NormalizedResult {
    let unknown = |reason: String| NormalizedResult {
        engine_name: raw.engine.name.clone(),
        class: NormalizedClass::Unknown,
        scope: raw.scope.clone(),
        reason,
        certificate_accepted: false,
    };

    if let Err(error) = raw.engine.validate_exact() {
        return unknown(format!("inexact backend identity: {error}"));
    }
    if raw.method.trim().is_empty() || raw.method.eq_ignore_ascii_case("unknown") {
        return unknown("verification method must be exact".to_owned());
    }
    let expected = match EngineBinding::for_vc(vc) {
        Ok(binding) => binding,
        Err(error) => return unknown(format!("invalid active VC: {error}")),
    };
    if let Some(field) = raw.binding.mismatch(&expected) {
        return unknown(format!("stale or mismatched {field}"));
    }

    match &raw.status {
        RawStatus::Unknown { reason } => unknown(reason.clone()),
        RawStatus::BackendFailure { reason } => unknown(format!("backend failure: {reason}")),
        RawStatus::Refuted { witness_states } => {
            if let Err(error) = check_refutation_witness(vc, witness_states) {
                return unknown(format!("refutation witness rejected: {error}"));
            }
            NormalizedResult {
                engine_name: raw.engine.name.clone(),
                class: NormalizedClass::Refuted,
                scope: raw.scope.clone(),
                reason: "independently replayed concrete violation witness".to_owned(),
                certificate_accepted: false,
            }
        }
        RawStatus::Holds {
            requested_class: NormalizedClass::Proved,
        } => {
            if raw.scope.is_bounded() {
                return unknown(
                    "bounded or sampled evidence cannot be promoted to proved".to_owned(),
                );
            }
            let Some(certificate) = &raw.certificate else {
                return unknown("proved requires an accepted proof certificate".to_owned());
            };
            match check_finite_invariant_certificate(vc, certificate) {
                Ok(()) => NormalizedResult {
                    engine_name: raw.engine.name.clone(),
                    class: NormalizedClass::Proved,
                    scope: raw.scope.clone(),
                    reason: "independently checked finite invariant certificate".to_owned(),
                    certificate_accepted: true,
                },
                Err(error) => unknown(format!("certificate rejected: {error}")),
            }
        }
        RawStatus::Holds {
            requested_class: NormalizedClass::ModelChecked,
        } => {
            let ResultScope::CompleteFinite { states } = &raw.scope else {
                return unknown(
                    "model_checked requires independently checked complete finite coverage"
                        .to_owned(),
                );
            };
            let Some(certificate) = &raw.certificate else {
                return unknown(
                    "model_checked requires a replayable finite coverage certificate".to_owned(),
                );
            };
            if certificate.invariant_states.len() != *states {
                return unknown(
                    "complete-finite state count differs from the coverage certificate".to_owned(),
                );
            }
            match check_finite_invariant_certificate(vc, certificate) {
                Ok(()) => NormalizedResult {
                    engine_name: raw.engine.name.clone(),
                    class: NormalizedClass::ModelChecked,
                    scope: raw.scope.clone(),
                    reason: "independently checked complete finite coverage certificate".to_owned(),
                    certificate_accepted: true,
                },
                Err(error) => unknown(format!("coverage certificate rejected: {error}")),
            }
        }
        RawStatus::Holds {
            requested_class: NormalizedClass::Tested,
        } => {
            if !matches!(raw.scope, ResultScope::Sampled { .. }) {
                return unknown("tested requires a sampled scope".to_owned());
            }
            NormalizedResult {
                engine_name: raw.engine.name.clone(),
                class: NormalizedClass::Tested,
                scope: raw.scope.clone(),
                reason: "generated concrete tests passed at the recorded scope".to_owned(),
                certificate_accepted: false,
            }
        }
        RawStatus::Holds { requested_class } => unknown(format!(
            "a holds result cannot request classification {requested_class:?}"
        )),
    }
}

fn check_refutation_witness(vc: &FiniteSafetyVc, witness: &[u64]) -> Result<(), String> {
    let Some(&first) = witness.first() else {
        return Err("witness is empty".to_owned());
    };
    if witness
        .iter()
        .any(|&state| state >= vc.state_count() as u64)
    {
        return Err("witness contains an out-of-range state".to_owned());
    }
    if !eval(&vc.initial, first as usize, 0) {
        return Err("first witness state is not initial".to_owned());
    }
    for edge in witness.windows(2) {
        if !eval(&vc.transition, edge[0] as usize, edge[1] as usize) {
            return Err(format!(
                "witness step {} -> {} is not a transition",
                edge[0], edge[1]
            ));
        }
    }
    let last = *witness.last().expect("nonempty witness") as usize;
    if eval(&vc.property, last, 0) {
        return Err("last witness state does not violate the property".to_owned());
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositeEvidence {
    pub vc_digest: Sha256Id,
    pub class: NormalizedClass,
    pub normalized: Vec<NormalizedResult>,
    pub raw_results: Vec<RawEngineResult>,
    pub disagreements: Vec<String>,
    pub trusted_components: Vec<TrustedComponent>,
}

/// Composes supplied results without erasing raw backend output. Any positive versus
/// negative disagreement, stale result, backend failure, or rejected certificate
/// makes the aggregate `unknown`.
pub fn compose_evidence(
    vc: &FiniteSafetyVc,
    raw_results: Vec<RawEngineResult>,
) -> Result<CompositeEvidence, String> {
    let vc_digest = vc.digest().map_err(|error| error.to_string())?;
    let normalized: Vec<_> = raw_results
        .iter()
        .map(|raw| normalize_result(vc, raw))
        .collect();
    let mut disagreements = Vec::new();
    let raw_has_positive = raw_results.iter().any(|result| {
        matches!(
            result.status,
            RawStatus::Holds {
                requested_class: NormalizedClass::Proved
                    | NormalizedClass::ModelChecked
                    | NormalizedClass::Tested
            }
        )
    });
    let raw_has_refuted = raw_results
        .iter()
        .any(|result| matches!(result.status, RawStatus::Refuted { .. }));
    let has_positive = normalized.iter().any(|result| {
        matches!(
            result.class,
            NormalizedClass::Proved | NormalizedClass::ModelChecked | NormalizedClass::Tested
        )
    });
    let has_refuted = normalized
        .iter()
        .any(|result| result.class == NormalizedClass::Refuted);
    if (has_positive && has_refuted) || (raw_has_positive && raw_has_refuted) {
        disagreements
            .push("at least one engine accepted the claim while another refuted it".to_owned());
    }
    if normalized.is_empty() {
        disagreements.push("no engine result was supplied".to_owned());
    }
    for result in &normalized {
        if result.class == NormalizedClass::Unknown {
            disagreements.push(format!("{}: {}", result.engine_name, result.reason));
        }
    }

    let class = if !disagreements.is_empty() {
        NormalizedClass::Unknown
    } else if has_refuted {
        NormalizedClass::Refuted
    } else if normalized
        .iter()
        .any(|result| result.class == NormalizedClass::Proved)
        && normalized.iter().all(|result| !result.scope.is_bounded())
    {
        NormalizedClass::Proved
    } else if normalized
        .iter()
        .any(|result| result.class == NormalizedClass::ModelChecked)
    {
        NormalizedClass::ModelChecked
    } else if normalized
        .iter()
        .any(|result| result.class == NormalizedClass::Tested)
    {
        NormalizedClass::Tested
    } else {
        NormalizedClass::Unknown
    };

    let mut components = BTreeSet::new();
    for raw in &raw_results {
        components.extend(raw.engine.trusted_components.iter().cloned());
    }
    Ok(CompositeEvidence {
        vc_digest,
        class,
        normalized,
        raw_results,
        disagreements,
        trusted_components: components.into_iter().collect(),
    })
}
