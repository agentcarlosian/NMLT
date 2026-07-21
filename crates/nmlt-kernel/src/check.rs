use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use nmlt_certificate::{DerivationConclusion, DerivationWitness};
use nmlt_hir::{
    DeclarationFlavor, DefId, HirBinaryOp, HirNode, HirNodeKind, HirRoot, HirUnaryOp, LocalBinder,
    LocalId, ModuleId, Namespace, NodeId, ResolvedDeclaration, ResolvedProgram, ResolvedRef,
    SemanticPathSegment,
};
use nmlt_ir::{
    CoreBinaryOp, CoreNodeId, CoreProgram, CorePropertyKind, CoreTerm, CoreTermKind, CoreType,
    CoreUnaryOp,
};

use crate::KernelProfileId;
use crate::identity::{
    certificate_digest, derivation_digest, kernel_profile_id, policy_digest, ruleset_digest,
};
use crate::input::{RawCertificate, RawDerivationNode, RawObligation};

const FORMAT_VERSION: u16 = 1;
const CERTIFICATE_DOMAIN: &[u8] = b"NMLT-ELABORATION-CERTIFICATE\0v1\0";
const MAX_CERTIFICATE_BYTES: usize = 64 * 1024 * 1024;
const MAX_REQUIRED_ROOTS: usize = 524_288;
const MAX_DERIVATIONS: usize = 524_288;
const MAX_PREMISE_EDGES: usize = 2_097_152;
const MAX_PREMISES: usize = 32;
const MAX_DERIVATION_DEPTH: usize = 256;
const MAX_MAGNITUDE_BYTES: usize = 4_096;

/// Stable failure classes at the independent M9-v1 acceptance boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelCode {
    Version,
    StaleBinding,
    Ruleset,
    ResourcePolicy,
    ResourceLimit,
    NonCanonicalOrder,
    DuplicateKey,
    UnknownTag,
    MissingPremise,
    Cycle,
    Unreachable,
    Identity,
    Coverage,
    InvalidRule,
    CoreMismatch,
}

impl KernelCode {
    /// Stable machine-readable suffix used by kernel diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Version => "VERSION",
            Self::StaleBinding => "STALE_BINDING",
            Self::Ruleset => "RULESET",
            Self::ResourcePolicy => "RESOURCE_POLICY",
            Self::ResourceLimit => "RESOURCE_LIMIT",
            Self::NonCanonicalOrder => "NONCANONICAL_ORDER",
            Self::DuplicateKey => "DUPLICATE_KEY",
            Self::UnknownTag => "UNKNOWN_TAG",
            Self::MissingPremise => "MISSING_PREMISE",
            Self::Cycle => "CYCLE",
            Self::Unreachable => "UNREACHABLE",
            Self::Identity => "IDENTITY",
            Self::Coverage => "COVERAGE",
            Self::InvalidRule => "INVALID_RULE",
            Self::CoreMismatch => "CORE_MISMATCH",
        }
    }
}

/// One fail-closed kernel diagnostic. It never denotes property refutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelDiagnostic {
    code: KernelCode,
    origin: Option<NodeId>,
    detail: &'static str,
}

impl KernelDiagnostic {
    const fn new(code: KernelCode, detail: &'static str) -> Self {
        Self {
            code,
            origin: None,
            detail,
        }
    }

    const fn at(code: KernelCode, origin: NodeId, detail: &'static str) -> Self {
        Self {
            code,
            origin: Some(origin),
            detail,
        }
    }

    #[must_use]
    pub const fn code(&self) -> KernelCode {
        self.code
    }

    #[must_use]
    pub const fn origin(&self) -> Option<NodeId> {
        self.origin
    }

    #[must_use]
    pub const fn detail(&self) -> &'static str {
        self.detail
    }
}

impl fmt::Display for KernelDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "NMLT_KERNEL_{}: {}",
            self.code.as_str(),
            self.detail
        )?;
        if let Some(origin) = self.origin {
            write!(formatter, " at {origin}")?;
        }
        Ok(())
    }
}

impl std::error::Error for KernelDiagnostic {}

/// The only M9 type that grants downstream access to kernel-accepted core.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedProgram {
    resolved_program: ResolvedProgram,
    core_program: CoreProgram,
    certificate_digest: [u8; 32],
    ruleset_bundle_digest: [u8; 32],
    resource_policy_digest: [u8; 32],
    kernel_profile_id: KernelProfileId,
}

impl CheckedProgram {
    /// Exact resolved HIR whose judgments were independently replayed.
    #[must_use]
    pub const fn resolved_program(&self) -> &ResolvedProgram {
        &self.resolved_program
    }

    #[must_use]
    pub const fn core_program(&self) -> &CoreProgram {
        &self.core_program
    }

    #[must_use]
    pub const fn certificate_digest(&self) -> &[u8; 32] {
        &self.certificate_digest
    }

    #[must_use]
    pub const fn ruleset_bundle_digest(&self) -> &[u8; 32] {
        &self.ruleset_bundle_digest
    }

    #[must_use]
    pub const fn resource_policy_digest(&self) -> &[u8; 32] {
        &self.resource_policy_digest
    }

    #[must_use]
    pub const fn kernel_profile_id(&self) -> KernelProfileId {
        self.kernel_profile_id
    }
}

#[derive(Clone)]
struct DeclMeta<'a> {
    declaration: &'a ResolvedDeclaration,
    module: ModuleId,
    parent: Option<DefId>,
}

struct Index<'a> {
    declarations: BTreeMap<DefId, DeclMeta<'a>>,
    nodes: BTreeMap<NodeId, &'a HirNode>,
    roots: BTreeMap<DefId, Vec<&'a HirRoot>>,
    locals: BTreeMap<LocalId, &'a LocalBinder>,
}

impl<'a> Index<'a> {
    fn new(hir: &'a ResolvedProgram) -> Result<Self, KernelDiagnostic> {
        let mut declarations = BTreeMap::new();
        let mut nodes = BTreeMap::new();
        let mut roots: BTreeMap<DefId, Vec<&HirRoot>> = BTreeMap::new();
        let mut locals = BTreeMap::new();
        for module in hir.modules().values() {
            let ids_by_path = module
                .declarations()
                .values()
                .map(|declaration| (declaration.key().path.clone(), declaration.id()))
                .collect::<BTreeMap<_, _>>();
            for declaration in module.declarations().values() {
                let mut parent_path = declaration.key().path.clone();
                parent_path.segments.pop();
                let parent = ids_by_path.get(&parent_path).copied();
                if declarations
                    .insert(
                        declaration.id(),
                        DeclMeta {
                            declaration,
                            module: module.id(),
                            parent,
                        },
                    )
                    .is_some()
                {
                    return Err(KernelDiagnostic::new(
                        KernelCode::DuplicateKey,
                        "duplicate HIR declaration identity",
                    ));
                }
            }
            for (id, node) in module.hir_nodes() {
                if nodes.insert(*id, node).is_some() {
                    return Err(KernelDiagnostic::new(
                        KernelCode::DuplicateKey,
                        "duplicate HIR node identity",
                    ));
                }
            }
            for root in module.hir_roots() {
                roots.entry(root.owner()).or_default().push(root);
            }
            for (id, binder) in module.local_binders() {
                if locals.insert(*id, binder).is_some() {
                    return Err(KernelDiagnostic::new(
                        KernelCode::DuplicateKey,
                        "duplicate local identity",
                    ));
                }
            }
        }
        for values in roots.values_mut() {
            values.sort_by(|left, right| left.semantic_path().cmp(right.semantic_path()));
        }
        Ok(Self {
            declarations,
            nodes,
            roots,
            locals,
        })
    }

    fn node(&self, id: NodeId) -> Result<&'a HirNode, KernelDiagnostic> {
        self.nodes.get(&id).copied().ok_or(KernelDiagnostic::at(
            KernelCode::Coverage,
            id,
            "certificate names a missing HIR node",
        ))
    }

    fn roots(&self, owner: DefId) -> impl Iterator<Item = &'a HirRoot> + '_ {
        self.roots.get(&owner).into_iter().flatten().copied()
    }

    fn root_exact(
        &self,
        owner: DefId,
        segments: &[SemanticPathSegment],
    ) -> Result<&'a HirRoot, KernelDiagnostic> {
        self.roots(owner)
            .find(|root| root.semantic_path().segments() == segments)
            .ok_or(KernelDiagnostic::new(
                KernelCode::Coverage,
                "required semantic HIR root is missing",
            ))
    }
}

/// Independently check one untrusted certificate against exact HIR and core.
pub fn check(
    hir: &ResolvedProgram,
    core: &CoreProgram,
    certificate: &RawCertificate,
) -> Result<CheckedProgram, KernelDiagnostic> {
    check_envelope(hir, core, certificate)?;
    let index = Index::new(hir)?;
    let derivations = check_graph_and_coverage(&index, certificate)?;
    check_canonical_certificate(certificate)?;
    let obligations = derivations
        .values()
        .map(|node| (node.obligation, node.claimed_digest))
        .collect();
    let checker = SemanticChecker {
        hir,
        core,
        index,
        derivations,
        obligations,
    };
    checker.check_all()?;
    Ok(CheckedProgram {
        resolved_program: hir.clone(),
        core_program: core.clone(),
        certificate_digest: certificate.certificate_digest,
        ruleset_bundle_digest: certificate.ruleset_bundle_digest,
        resource_policy_digest: certificate.resource_policy_digest,
        kernel_profile_id: kernel_profile_id(),
    })
}

fn check_envelope(
    hir: &ResolvedProgram,
    core: &CoreProgram,
    certificate: &RawCertificate,
) -> Result<(), KernelDiagnostic> {
    if certificate.format_version != FORMAT_VERSION {
        return Err(KernelDiagnostic::new(
            KernelCode::Version,
            "unsupported certificate format version",
        ));
    }
    for (actual, expected, detail) in [
        (
            certificate.source_set_digest,
            *hir.source_set_id().digest(),
            "source-set identity does not match HIR",
        ),
        (
            certificate.module_map_digest,
            *hir.module_map_id().digest(),
            "module-map identity does not match HIR",
        ),
        (
            certificate.surface_program_digest,
            *hir.surface_program_id().digest(),
            "surface-program identity does not match HIR",
        ),
        (
            certificate.resolved_hir_digest,
            *hir.resolution_id().digest(),
            "resolved-HIR identity does not match HIR",
        ),
        (
            certificate.core_program_digest,
            *core.id().digest(),
            "core-program identity does not match core",
        ),
    ] {
        if actual != expected {
            return Err(KernelDiagnostic::new(KernelCode::StaleBinding, detail));
        }
    }
    if core.resolved_hir_id() != hir.resolution_id() {
        return Err(KernelDiagnostic::new(
            KernelCode::StaleBinding,
            "core is bound to another resolved HIR",
        ));
    }
    if certificate.ruleset_bundle_digest != ruleset_digest() {
        return Err(KernelDiagnostic::new(
            KernelCode::Ruleset,
            "certificate ruleset is not the checker-selected M9-v1 bundle",
        ));
    }
    if certificate.resource_policy_digest != policy_digest() {
        return Err(KernelDiagnostic::new(
            KernelCode::ResourcePolicy,
            "certificate policy is not the checker-selected M9-v1 policy",
        ));
    }
    if certificate.required_roots.len() > MAX_REQUIRED_ROOTS {
        return Err(KernelDiagnostic::new(
            KernelCode::ResourceLimit,
            "required-root limit exceeded",
        ));
    }
    if certificate.derivations.len() > MAX_DERIVATIONS {
        return Err(KernelDiagnostic::new(
            KernelCode::ResourceLimit,
            "derivation-node limit exceeded",
        ));
    }
    let mut edges = 0usize;
    for node in &certificate.derivations {
        if node.premises.len() > MAX_PREMISES {
            return Err(KernelDiagnostic::at(
                KernelCode::ResourceLimit,
                node.obligation.origin,
                "premises-per-node limit exceeded",
            ));
        }
        edges = edges
            .checked_add(node.premises.len())
            .ok_or(KernelDiagnostic::new(
                KernelCode::ResourceLimit,
                "premise-edge count overflow",
            ))?;
    }
    if edges > MAX_PREMISE_EDGES {
        return Err(KernelDiagnostic::new(
            KernelCode::ResourceLimit,
            "premise-edge limit exceeded",
        ));
    }
    preflight_certificate_bytes(certificate)?;
    Ok(())
}

fn preflight_certificate_bytes(certificate: &RawCertificate) -> Result<usize, KernelDiagnostic> {
    let mut encoded_len = 0usize;
    add_certificate_bytes(&mut encoded_len, CERTIFICATE_DOMAIN.len())?;
    add_certificate_bytes(&mut encoded_len, 2 + 7 * 32 + 8)?;
    add_certificate_bytes(
        &mut encoded_len,
        certificate
            .required_roots
            .len()
            .checked_mul(1 + 32 + 32)
            .ok_or(KernelDiagnostic::new(
                KernelCode::ResourceLimit,
                "canonical certificate-byte count overflow",
            ))?,
    )?;
    add_certificate_bytes(&mut encoded_len, 8)?;

    for node in &certificate.derivations {
        add_certificate_bytes(&mut encoded_len, 32 + 2 + 1 + 32)?;
        add_certificate_bytes(
            &mut encoded_len,
            match &node.conclusion {
                DerivationConclusion::Type(ty) => 1 + encoded_core_type_bytes(ty),
                DerivationConclusion::Protocol(_) | DerivationConclusion::Definition(_) => 1 + 32,
                DerivationConclusion::Term { ty, .. } => 1 + 32 + encoded_core_type_bytes(ty),
            },
        )?;
        add_certificate_bytes(
            &mut encoded_len,
            match &node.witness {
                DerivationWitness::None => 1,
                DerivationWitness::Boolean(_) => 2,
                DerivationWitness::Magnitude { bytes, .. } => {
                    if bytes.len() > MAX_MAGNITUDE_BYTES {
                        return Err(KernelDiagnostic::at(
                            KernelCode::ResourceLimit,
                            node.obligation.origin,
                            "integer magnitude length exceeds policy",
                        ));
                    }
                    1 + 1 + 8 + bytes.len()
                }
                DerivationWitness::Definition(_) => 1 + 32,
                DerivationWitness::SystemDefinition { .. } => 1 + 32 + 32,
            },
        )?;
        add_certificate_bytes(&mut encoded_len, 8)?;
        add_certificate_bytes(
            &mut encoded_len,
            node.premises
                .len()
                .checked_mul(32)
                .ok_or(KernelDiagnostic::at(
                    KernelCode::ResourceLimit,
                    node.obligation.origin,
                    "canonical certificate-byte count overflow",
                ))?,
        )?;
    }
    Ok(encoded_len)
}

const fn encoded_core_type_bytes(ty: &CoreType) -> usize {
    match ty {
        CoreType::Bool | CoreType::Nat | CoreType::Int => 1,
        CoreType::Enum(_)
        | CoreType::Once { .. }
        | CoreType::StateProp { .. }
        | CoreType::TemporalProp { .. } => 1 + 32,
    }
}

fn add_certificate_bytes(
    encoded_len: &mut usize,
    additional: usize,
) -> Result<(), KernelDiagnostic> {
    *encoded_len = encoded_len
        .checked_add(additional)
        .ok_or(KernelDiagnostic::new(
            KernelCode::ResourceLimit,
            "canonical certificate-byte count overflow",
        ))?;
    if *encoded_len > MAX_CERTIFICATE_BYTES {
        return Err(KernelDiagnostic::new(
            KernelCode::ResourceLimit,
            "canonical certificate-byte limit exceeded",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod certificate_size_tests {
    use nmlt_certificate::{DerivationConclusion, DerivationWitness};
    use nmlt_hir::NodeId;
    use nmlt_ir::CoreType;

    use super::{
        FORMAT_VERSION, KernelCode, MAX_CERTIFICATE_BYTES, RawCertificate, RawDerivationNode,
        RawObligation, add_certificate_bytes, preflight_certificate_bytes,
    };

    fn certificate_with_witness(witness: DerivationWitness) -> RawCertificate {
        RawCertificate {
            format_version: FORMAT_VERSION,
            source_set_digest: [0; 32],
            module_map_digest: [0; 32],
            surface_program_digest: [0; 32],
            resolved_hir_digest: [0; 32],
            core_program_digest: [0; 32],
            ruleset_bundle_digest: [0; 32],
            resource_policy_digest: [0; 32],
            required_roots: Vec::new(),
            derivations: vec![RawDerivationNode {
                claimed_digest: [0; 32],
                rule_tag: 1,
                obligation: RawObligation {
                    judgment_tag: 1,
                    origin: NodeId::from_untrusted_digest([0; 32]),
                },
                conclusion: DerivationConclusion::Type(CoreType::Bool),
                witness,
                premises: vec![[0; 32], [1; 32]],
            }],
            certificate_digest: [0; 32],
        }
    }

    #[test]
    fn structural_preflight_matches_canonical_encoding_length() {
        let certificate = certificate_with_witness(DerivationWitness::Magnitude {
            negative: false,
            bytes: vec![1, 2, 3],
        });
        assert_eq!(
            preflight_certificate_bytes(&certificate).unwrap(),
            certificate.to_canonical_bytes().len()
        );
    }

    #[test]
    fn certificate_byte_limit_distinguishes_exact_boundary_from_plus_one() {
        let mut encoded_len = MAX_CERTIFICATE_BYTES - 1;
        add_certificate_bytes(&mut encoded_len, 1).unwrap();
        assert_eq!(encoded_len, MAX_CERTIFICATE_BYTES);
        let error = add_certificate_bytes(&mut encoded_len, 1).unwrap_err();
        assert_eq!(error.code(), KernelCode::ResourceLimit);
    }

    #[test]
    fn oversized_programmatic_magnitude_fails_preflight() {
        let certificate = certificate_with_witness(DerivationWitness::Magnitude {
            negative: false,
            bytes: vec![0; 4_097],
        });
        let error = preflight_certificate_bytes(&certificate).unwrap_err();
        assert_eq!(error.code(), KernelCode::ResourceLimit);
    }
}

fn check_canonical_certificate(certificate: &RawCertificate) -> Result<(), KernelDiagnostic> {
    require_strict_order(
        certificate
            .required_roots
            .iter()
            .map(|root| root.obligation),
        "required roots are not in canonical order",
    )?;
    require_strict_order(
        certificate
            .derivations
            .iter()
            .map(|node| node.claimed_digest),
        "derivations are not in canonical digest order",
    )?;
    let (actual_certificate, encoded_len) = certificate_digest(certificate);
    if encoded_len > MAX_CERTIFICATE_BYTES {
        return Err(KernelDiagnostic::new(
            KernelCode::ResourceLimit,
            "canonical certificate-byte limit exceeded",
        ));
    }
    if actual_certificate != certificate.certificate_digest {
        return Err(KernelDiagnostic::new(
            KernelCode::Identity,
            "certificate content identity is noncanonical",
        ));
    }
    Ok(())
}

fn require_strict_order<T: Ord>(
    values: impl IntoIterator<Item = T>,
    detail: &'static str,
) -> Result<(), KernelDiagnostic> {
    let mut previous = None;
    for value in values {
        if previous.as_ref().is_some_and(|item| item >= &value) {
            return Err(KernelDiagnostic::new(KernelCode::NonCanonicalOrder, detail));
        }
        previous = Some(value);
    }
    Ok(())
}

fn check_graph_and_coverage<'a>(
    index: &Index<'_>,
    certificate: &'a RawCertificate,
) -> Result<BTreeMap<[u8; 32], &'a RawDerivationNode>, KernelDiagnostic> {
    let mut derivations = BTreeMap::new();
    let mut obligations = BTreeMap::new();
    for node in &certificate.derivations {
        known_judgment(node.obligation.judgment_tag)?;
        known_rule(node.rule_tag)?;
        if derivations.insert(node.claimed_digest, node).is_some()
            || obligations
                .insert(node.obligation, node.claimed_digest)
                .is_some()
        {
            return Err(KernelDiagnostic::at(
                KernelCode::DuplicateKey,
                node.obligation.origin,
                "duplicate derivation identity or obligation",
            ));
        }
    }
    for node in &certificate.derivations {
        for premise in &node.premises {
            if !derivations.contains_key(premise) {
                return Err(KernelDiagnostic::at(
                    KernelCode::MissingPremise,
                    node.obligation.origin,
                    "derivation premise is absent",
                ));
            }
        }
    }
    check_acyclic_depth(&derivations)?;

    let mut root_map = BTreeMap::new();
    for root in &certificate.required_roots {
        known_judgment(root.obligation.judgment_tag)?;
        if root_map
            .insert(root.obligation, root.derivation_digest)
            .is_some()
        {
            return Err(KernelDiagnostic::new(
                KernelCode::DuplicateKey,
                "duplicate required-root obligation",
            ));
        }
        let node = derivations
            .get(&root.derivation_digest)
            .ok_or(KernelDiagnostic::new(
                KernelCode::MissingPremise,
                "required root names a missing derivation",
            ))?;
        if node.obligation != root.obligation {
            return Err(KernelDiagnostic::at(
                KernelCode::Coverage,
                root.obligation.origin,
                "required root and derivation obligation disagree",
            ));
        }
    }
    let expected = expected_roots(index)?;
    if root_map.keys().copied().collect::<BTreeSet<_>>() != expected {
        return Err(KernelDiagnostic::new(
            KernelCode::Coverage,
            "certificate roots are not a bijection with HIR roots",
        ));
    }

    let mut reachable = BTreeSet::new();
    let mut pending = root_map.values().copied().collect::<Vec<_>>();
    while let Some(id) = pending.pop() {
        if reachable.insert(id) {
            pending.extend(derivations[&id].premises.iter().copied());
        }
    }
    if reachable.len() != derivations.len() {
        return Err(KernelDiagnostic::new(
            KernelCode::Unreachable,
            "certificate contains a derivation unreachable from required roots",
        ));
    }
    let covered = derivations
        .values()
        .map(|node| node.obligation.origin)
        .collect::<BTreeSet<_>>();
    if covered != index.nodes.keys().copied().collect() {
        return Err(KernelDiagnostic::new(
            KernelCode::Coverage,
            "derivation subjects do not cover exactly the HIR node origins",
        ));
    }
    for node in derivations.values() {
        if derivation_digest(node) != node.claimed_digest {
            return Err(KernelDiagnostic::at(
                KernelCode::Identity,
                node.obligation.origin,
                "derivation content identity is noncanonical",
            ));
        }
    }
    Ok(derivations)
}

fn check_acyclic_depth(
    derivations: &BTreeMap<[u8; 32], &RawDerivationNode>,
) -> Result<(), KernelDiagnostic> {
    let mut completed = BTreeSet::new();
    for start in derivations.keys().copied() {
        if completed.contains(&start) {
            continue;
        }
        let mut active = BTreeSet::new();
        let mut stack = vec![(start, 0usize)];
        active.insert(start);
        while !stack.is_empty() {
            if stack.len() > MAX_DERIVATION_DEPTH {
                return Err(KernelDiagnostic::new(
                    KernelCode::ResourceLimit,
                    "derivation depth exceeds the checker policy",
                ));
            }
            let (id, next) = stack.last_mut().ok_or(KernelDiagnostic::new(
                KernelCode::Cycle,
                "derivation traversal stack changed unexpectedly",
            ))?;
            let node = derivations[id];
            if *next == node.premises.len() {
                let finished = *id;
                stack.pop();
                active.remove(&finished);
                completed.insert(finished);
                continue;
            }
            let premise = node.premises[*next];
            *next += 1;
            if completed.contains(&premise) {
                continue;
            }
            if !active.insert(premise) {
                return Err(KernelDiagnostic::new(
                    KernelCode::Cycle,
                    "derivation graph contains a cycle",
                ));
            }
            stack.push((premise, 0));
        }
    }
    Ok(())
}

fn expected_roots(index: &Index<'_>) -> Result<BTreeSet<RawObligation>, KernelDiagnostic> {
    index
        .roots
        .values()
        .flatten()
        .map(|root| {
            let judgment_tag = match root.semantic_path().segments() {
                [SemanticPathSegment::DeclaredType]
                | [
                    SemanticPathSegment::ActionParameter(_),
                    SemanticPathSegment::DeclaredType,
                ] => 4,
                [SemanticPathSegment::Initializer]
                | [SemanticPathSegment::Guard(_)]
                | [SemanticPathSegment::UpdateValue(_)] => 2,
                [SemanticPathSegment::UpdateTarget(_)] => 6,
                [SemanticPathSegment::Output(_)] | [SemanticPathSegment::ObservationItem(_)] => 1,
                [SemanticPathSegment::Consume(_)] => 7,
                [SemanticPathSegment::PropertyBody] => 3,
                _ => {
                    return Err(KernelDiagnostic::new(
                        KernelCode::Coverage,
                        "HIR root has no frozen M9-v1 judgment",
                    ));
                }
            };
            Ok(RawObligation {
                judgment_tag,
                origin: root.node(),
            })
        })
        .collect()
}

fn known_judgment(tag: u8) -> Result<(), KernelDiagnostic> {
    if (1..=8).contains(&tag) {
        Ok(())
    } else {
        Err(KernelDiagnostic::new(
            KernelCode::UnknownTag,
            "unknown judgment tag",
        ))
    }
}

fn known_rule(tag: u16) -> Result<(), KernelDiagnostic> {
    if matches!(
        tag,
        1..=6 | 10..=16 | 20..=34 | 40..=45 | 50..=52
    ) {
        Ok(())
    } else {
        Err(KernelDiagnostic::new(
            KernelCode::UnknownTag,
            "unknown elaboration-rule tag",
        ))
    }
}

struct SemanticChecker<'a> {
    hir: &'a ResolvedProgram,
    core: &'a CoreProgram,
    index: Index<'a>,
    derivations: BTreeMap<[u8; 32], &'a RawDerivationNode>,
    obligations: BTreeMap<RawObligation, [u8; 32]>,
}

impl<'a> SemanticChecker<'a> {
    fn check_all(&self) -> Result<(), KernelDiagnostic> {
        for node in self.derivations.values() {
            self.check_rule(node)?;
        }
        self.check_core_coverage()?;
        self.check_aggregate_core()
    }

    fn check_rule(&self, node: &RawDerivationNode) -> Result<(), KernelDiagnostic> {
        match node.rule_tag {
            1..=6 => self.check_type_rule(node),
            10..=16 => self.check_atomic_value_rule(node),
            20..=34 => self.check_operator_rule(node),
            40..=45 => self.check_formula_rule(node),
            50..=52 => self.check_definition_use_rule(node),
            _ => Err(KernelDiagnostic::at(
                KernelCode::UnknownTag,
                node.obligation.origin,
                "unknown rule reached semantic checking",
            )),
        }
    }

    fn premise(
        &self,
        node: &RawDerivationNode,
        index: usize,
    ) -> Result<&'a RawDerivationNode, KernelDiagnostic> {
        node.premises
            .get(index)
            .and_then(|id| self.derivations.get(id).copied())
            .ok_or(KernelDiagnostic::at(
                KernelCode::InvalidRule,
                node.obligation.origin,
                "rule has the wrong premise shape",
            ))
    }

    fn derivation_for(
        &self,
        judgment_tag: u8,
        origin: NodeId,
    ) -> Result<&'a RawDerivationNode, KernelDiagnostic> {
        let obligation = RawObligation {
            judgment_tag,
            origin,
        };
        self.obligations
            .get(&obligation)
            .and_then(|id| self.derivations.get(id).copied())
            .ok_or(KernelDiagnostic::at(
                KernelCode::Coverage,
                origin,
                "required derivation obligation is absent",
            ))
    }

    fn term_conclusion(
        &self,
        node: &RawDerivationNode,
    ) -> Result<(CoreNodeId, CoreType, &CoreTerm), KernelDiagnostic> {
        let DerivationConclusion::Term { node: id, ty } = &node.conclusion else {
            return Err(KernelDiagnostic::at(
                KernelCode::InvalidRule,
                node.obligation.origin,
                "term rule has a non-term conclusion",
            ));
        };
        let term = self.core.terms().get(id).ok_or(KernelDiagnostic::at(
            KernelCode::CoreMismatch,
            node.obligation.origin,
            "derivation names a missing core term",
        ))?;
        if term.ty() != ty || term.origin() != node.obligation.origin {
            return Err(KernelDiagnostic::at(
                KernelCode::CoreMismatch,
                node.obligation.origin,
                "derivation conclusion disagrees with core term",
            ));
        }
        Ok((*id, ty.clone(), term))
    }

    fn check_core_coverage(&self) -> Result<(), KernelDiagnostic> {
        let concluded = self
            .derivations
            .values()
            .filter_map(|node| match node.conclusion {
                DerivationConclusion::Term { node, .. } => Some(node),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        if concluded != self.core.terms().keys().copied().collect() {
            return Err(KernelDiagnostic::new(
                KernelCode::Coverage,
                "derivation conclusions do not cover exactly the core terms",
            ));
        }
        Ok(())
    }

    // Implemented below as independent rule and aggregate reconstruction.
    fn check_type_rule(&self, node: &RawDerivationNode) -> Result<(), KernelDiagnostic> {
        check_type_rule(self, node)
    }
    fn check_atomic_value_rule(&self, node: &RawDerivationNode) -> Result<(), KernelDiagnostic> {
        check_atomic_value_rule(self, node)
    }
    fn check_operator_rule(&self, node: &RawDerivationNode) -> Result<(), KernelDiagnostic> {
        check_operator_rule(self, node)
    }
    fn check_formula_rule(&self, node: &RawDerivationNode) -> Result<(), KernelDiagnostic> {
        check_formula_rule(self, node)
    }
    fn check_definition_use_rule(&self, node: &RawDerivationNode) -> Result<(), KernelDiagnostic> {
        check_definition_use_rule(self, node)
    }
    fn check_aggregate_core(&self) -> Result<(), KernelDiagnostic> {
        check_aggregate_core(self)
    }
}

fn invalid(node: &RawDerivationNode, detail: &'static str) -> KernelDiagnostic {
    KernelDiagnostic::at(KernelCode::InvalidRule, node.obligation.origin, detail)
}

fn require_premises(node: &RawDerivationNode, expected: usize) -> Result<(), KernelDiagnostic> {
    if node.premises.len() != expected {
        return Err(invalid(node, "rule has the wrong number of premises"));
    }
    Ok(())
}

fn require_none_witness(node: &RawDerivationNode) -> Result<(), KernelDiagnostic> {
    if node.witness != DerivationWitness::None {
        return Err(invalid(node, "rule carries a forbidden witness"));
    }
    Ok(())
}

fn require_obligation(node: &RawDerivationNode, allowed: &[u8]) -> Result<(), KernelDiagnostic> {
    if !allowed.contains(&node.obligation.judgment_tag) {
        return Err(invalid(node, "rule uses the wrong judgment kind"));
    }
    Ok(())
}

fn direct_core_id(origin: NodeId) -> Result<CoreNodeId, KernelDiagnostic> {
    CoreNodeId::from_origin(origin, &[]).map_err(|_| {
        KernelDiagnostic::at(
            KernelCode::CoreMismatch,
            origin,
            "cannot derive direct core-node identity",
        )
    })
}

fn require_term(
    checker: &SemanticChecker<'_>,
    node: &RawDerivationNode,
    expected_id: CoreNodeId,
    expected_type: &CoreType,
    expected_kind: &CoreTermKind,
) -> Result<(), KernelDiagnostic> {
    let (id, ty, term) = checker.term_conclusion(node)?;
    if id != expected_id
        || &ty != expected_type
        || term.owner() != checker.index.node(node.obligation.origin)?.owner()
        || term.kind() != expected_kind
    {
        return Err(KernelDiagnostic::at(
            KernelCode::CoreMismatch,
            node.obligation.origin,
            "rule reconstruction disagrees with core term",
        ));
    }
    Ok(())
}

fn type_conclusion(node: &RawDerivationNode) -> Result<&CoreType, KernelDiagnostic> {
    let DerivationConclusion::Type(ty) = &node.conclusion else {
        return Err(invalid(node, "type rule has a non-type conclusion"));
    };
    Ok(ty)
}

fn premise_term(
    checker: &SemanticChecker<'_>,
    node: &RawDerivationNode,
    index: usize,
) -> Result<(CoreNodeId, CoreType), KernelDiagnostic> {
    let premise = checker.premise(node, index)?;
    let (id, ty, _) = checker.term_conclusion(premise)?;
    Ok((id, ty))
}

fn declaration_namespace(
    checker: &SemanticChecker<'_>,
    definition: DefId,
) -> Result<Namespace, KernelDiagnostic> {
    checker
        .index
        .declarations
        .get(&definition)
        .and_then(|meta| meta.declaration.key().namespace())
        .ok_or(KernelDiagnostic::new(
            KernelCode::Coverage,
            "certificate names an unknown definition",
        ))
}

fn check_type_rule(
    checker: &SemanticChecker<'_>,
    node: &RawDerivationNode,
) -> Result<(), KernelDiagnostic> {
    require_obligation(node, &[4, 5])?;
    let hir = checker.index.node(node.obligation.origin)?;
    match node.rule_tag {
        1..=3 => {
            require_obligation(node, &[4])?;
            require_premises(node, 0)?;
            require_none_witness(node)?;
            let expected = match node.rule_tag {
                1 if matches!(hir.kind(), HirNodeKind::TypeBool) => CoreType::Bool,
                2 if matches!(hir.kind(), HirNodeKind::TypeNat) => CoreType::Nat,
                3 if matches!(hir.kind(), HirNodeKind::TypeInt) => CoreType::Int,
                _ => return Err(invalid(node, "primitive type rule does not match HIR")),
            };
            if type_conclusion(node)? != &expected {
                return Err(invalid(node, "primitive type conclusion is wrong"));
            }
        }
        4 => {
            require_obligation(node, &[4])?;
            require_premises(node, 0)?;
            let HirNodeKind::TypeNamed(ResolvedRef::Definition(definition)) = hir.kind() else {
                return Err(invalid(node, "enum type rule does not match HIR"));
            };
            if declaration_namespace(checker, *definition)? != Namespace::Type
                || type_conclusion(node)? != &CoreType::Enum(*definition)
                || node.witness != DerivationWitness::Definition(*definition)
            {
                return Err(invalid(node, "enum type reconstruction failed"));
            }
        }
        5 => {
            require_obligation(node, &[5])?;
            require_premises(node, 0)?;
            require_none_witness(node)?;
            if !matches!(hir.kind(), HirNodeKind::ProtocolTag { .. })
                || node.conclusion != DerivationConclusion::Protocol(hir.id())
            {
                return Err(invalid(node, "protocol rule does not match HIR"));
            }
        }
        6 => {
            require_obligation(node, &[4])?;
            require_premises(node, 1)?;
            require_none_witness(node)?;
            let HirNodeKind::TypeOnce { protocol } = hir.kind() else {
                return Err(invalid(node, "Once type rule does not match HIR"));
            };
            let premise = checker.premise(node, 0)?;
            if premise.obligation
                != (RawObligation {
                    judgment_tag: 5,
                    origin: *protocol,
                })
                || premise.conclusion != DerivationConclusion::Protocol(*protocol)
                || type_conclusion(node)?
                    != &(CoreType::Once {
                        protocol: *protocol,
                    })
            {
                return Err(invalid(
                    node,
                    "Once protocol premise or conclusion is wrong",
                ));
            }
        }
        _ => return Err(invalid(node, "unknown type rule")),
    }
    Ok(())
}

fn local_type(checker: &SemanticChecker<'_>, local: LocalId) -> Result<CoreType, KernelDiagnostic> {
    let binder = checker
        .index
        .locals
        .get(&local)
        .ok_or(KernelDiagnostic::new(
            KernelCode::Coverage,
            "local reference names a missing binder",
        ))?;
    let derivation = checker.derivation_for(4, binder.declared_type())?;
    Ok(type_conclusion(derivation)?.clone())
}

fn state_type(checker: &SemanticChecker<'_>, state: DefId) -> Result<CoreType, KernelDiagnostic> {
    let root = checker
        .index
        .root_exact(state, &[SemanticPathSegment::DeclaredType])?;
    Ok(type_conclusion(checker.derivation_for(4, root.node())?)?.clone())
}

fn check_atomic_value_rule(
    checker: &SemanticChecker<'_>,
    node: &RawDerivationNode,
) -> Result<(), KernelDiagnostic> {
    let hir = checker.index.node(node.obligation.origin)?;
    match node.rule_tag {
        10 => {
            require_obligation(node, &[1, 2])?;
            require_premises(node, 0)?;
            let HirNodeKind::BoolLiteral(value) = hir.kind() else {
                return Err(invalid(node, "Boolean literal rule does not match HIR"));
            };
            if node.witness != DerivationWitness::Boolean(*value) {
                return Err(invalid(node, "Boolean literal witness is wrong"));
            }
            require_term(
                checker,
                node,
                direct_core_id(hir.id())?,
                &CoreType::Bool,
                &CoreTermKind::Bool(*value),
            )?;
        }
        11 | 12 => {
            if node.rule_tag == 11 {
                require_obligation(node, &[1, 2])?;
            } else {
                require_obligation(node, &[2])?;
            }
            require_premises(node, 0)?;
            let HirNodeKind::NaturalLiteral { magnitude } = hir.kind() else {
                return Err(invalid(node, "numeric literal rule does not match HIR"));
            };
            let (ty, kind) = if node.rule_tag == 11 {
                (
                    CoreType::Nat,
                    CoreTermKind::Nat {
                        magnitude: magnitude.clone(),
                    },
                )
            } else {
                (
                    CoreType::Int,
                    CoreTermKind::Int {
                        negative: false,
                        magnitude: magnitude.clone(),
                    },
                )
            };
            if node.witness
                != (DerivationWitness::Magnitude {
                    negative: false,
                    bytes: magnitude.clone(),
                })
            {
                return Err(invalid(node, "numeric literal witness is wrong"));
            }
            require_term(checker, node, direct_core_id(hir.id())?, &ty, &kind)?;
        }
        13 => {
            require_obligation(node, &[1, 2])?;
            require_premises(node, 0)?;
            require_none_witness(node)?;
            let HirNodeKind::Reference(ResolvedRef::Local(local)) = hir.kind() else {
                return Err(invalid(node, "local-reference rule does not match HIR"));
            };
            if checker.index.locals[local].owner() != hir.owner() {
                return Err(invalid(node, "local reference crosses its action owner"));
            }
            let ty = local_type(checker, *local)?;
            require_term(
                checker,
                node,
                direct_core_id(hir.id())?,
                &ty,
                &CoreTermKind::Local(*local),
            )?;
        }
        14 => {
            require_obligation(node, &[1, 2])?;
            require_premises(node, 0)?;
            let HirNodeKind::Reference(ResolvedRef::StateField { system, state }) = hir.kind()
            else {
                return Err(invalid(node, "state-reference rule does not match HIR"));
            };
            let ty = state_type(checker, *state)?;
            let meta = checker
                .index
                .declarations
                .get(state)
                .ok_or(invalid(node, "state-reference definition is missing"))?;
            if meta.parent != Some(*system)
                || meta.declaration.key().namespace() != Some(Namespace::State)
            {
                return Err(invalid(node, "state-reference system binding is wrong"));
            }
            if node.witness
                != (DerivationWitness::SystemDefinition {
                    system: *system,
                    definition: *state,
                })
            {
                return Err(invalid(node, "state-reference witness is wrong"));
            }
            require_term(
                checker,
                node,
                direct_core_id(hir.id())?,
                &ty,
                &CoreTermKind::State {
                    system: *system,
                    state: *state,
                },
            )?;
        }
        15 => {
            require_obligation(node, &[1, 2])?;
            require_premises(node, 0)?;
            let HirNodeKind::Reference(ResolvedRef::Constructor {
                enumeration,
                constructor,
            }) = hir.kind()
            else {
                return Err(invalid(node, "constructor rule does not match HIR"));
            };
            let meta = checker
                .index
                .declarations
                .get(constructor)
                .ok_or(invalid(node, "constructor-reference definition is missing"))?;
            if meta.parent != Some(*enumeration)
                || meta.declaration.key().namespace() != Some(Namespace::Constructor)
            {
                return Err(invalid(node, "constructor enumeration binding is wrong"));
            }
            if node.witness
                != (DerivationWitness::SystemDefinition {
                    system: *enumeration,
                    definition: *constructor,
                })
            {
                return Err(invalid(node, "constructor witness is wrong"));
            }
            require_term(
                checker,
                node,
                direct_core_id(hir.id())?,
                &CoreType::Enum(*enumeration),
                &CoreTermKind::Constructor {
                    enumeration: *enumeration,
                    constructor: *constructor,
                },
            )?;
        }
        16 => {
            require_obligation(node, &[2])?;
            require_premises(node, 1)?;
            require_none_witness(node)?;
            let premise = checker.premise(node, 0)?;
            if premise.obligation
                != (RawObligation {
                    judgment_tag: 1,
                    origin: node.obligation.origin,
                })
                || premise.conclusion != node.conclusion
            {
                return Err(invalid(
                    node,
                    "check-from-synthesis premise or conclusion is wrong",
                ));
            }
            checker.term_conclusion(node)?;
        }
        _ => return Err(invalid(node, "unknown atomic value rule")),
    }
    Ok(())
}

fn map_unary(operator: HirUnaryOp) -> CoreUnaryOp {
    match operator {
        HirUnaryOp::Not => CoreUnaryOp::Not,
        HirUnaryOp::Negate => CoreUnaryOp::Negate,
    }
}

fn map_binary(operator: HirBinaryOp) -> CoreBinaryOp {
    match operator {
        HirBinaryOp::Or => CoreBinaryOp::Or,
        HirBinaryOp::And => CoreBinaryOp::And,
        HirBinaryOp::Implies => CoreBinaryOp::Implies,
        HirBinaryOp::Equal => CoreBinaryOp::Equal,
        HirBinaryOp::NotEqual => CoreBinaryOp::NotEqual,
        HirBinaryOp::Less => CoreBinaryOp::Less,
        HirBinaryOp::LessEqual => CoreBinaryOp::LessEqual,
        HirBinaryOp::Greater => CoreBinaryOp::Greater,
        HirBinaryOp::GreaterEqual => CoreBinaryOp::GreaterEqual,
        HirBinaryOp::Add => CoreBinaryOp::Add,
        HirBinaryOp::Subtract => CoreBinaryOp::Subtract,
        HirBinaryOp::Multiply => CoreBinaryOp::Multiply,
    }
}

fn binary_rule_tag(operator: HirBinaryOp) -> u16 {
    match operator {
        HirBinaryOp::Or => 22,
        HirBinaryOp::And => 23,
        HirBinaryOp::Implies => 24,
        HirBinaryOp::Equal => 25,
        HirBinaryOp::NotEqual => 26,
        HirBinaryOp::Less => 27,
        HirBinaryOp::LessEqual => 28,
        HirBinaryOp::Greater => 29,
        HirBinaryOp::GreaterEqual => 30,
        HirBinaryOp::Add => 31,
        HirBinaryOp::Subtract => 32,
        HirBinaryOp::Multiply => 33,
    }
}

fn formula_type(ty: &CoreType) -> Option<(DefId, bool)> {
    match ty {
        CoreType::StateProp { system } => Some((*system, false)),
        CoreType::TemporalProp { system } => Some((*system, true)),
        _ => None,
    }
}

fn check_operator_rule(
    checker: &SemanticChecker<'_>,
    node: &RawDerivationNode,
) -> Result<(), KernelDiagnostic> {
    require_none_witness(node)?;
    let hir = checker.index.node(node.obligation.origin)?;
    match node.rule_tag {
        20 | 21 => {
            require_premises(node, 1)?;
            let HirNodeKind::Unary { operator, operand } = hir.kind() else {
                return Err(invalid(node, "unary rule does not match HIR"));
            };
            let expected_rule = match operator {
                HirUnaryOp::Not => 20,
                HirUnaryOp::Negate => 21,
            };
            if node.rule_tag != expected_rule {
                return Err(invalid(node, "unary rule tag is wrong"));
            }
            let premise = checker.premise(node, 0)?;
            if premise.obligation.origin != *operand {
                return Err(invalid(node, "unary premise has the wrong HIR origin"));
            }
            let expected_premise_judgment = if node.obligation.judgment_tag == 3 {
                3
            } else {
                2
            };
            if premise.obligation.judgment_tag != expected_premise_judgment {
                return Err(invalid(node, "unary premise has the wrong judgment"));
            }
            let (operand_id, operand_ty) = premise_term(checker, node, 0)?;
            let result_ty = match operator {
                HirUnaryOp::Not => {
                    require_obligation(node, &[1, 2, 3])?;
                    if !matches!(
                        operand_ty,
                        CoreType::Bool | CoreType::StateProp { .. } | CoreType::TemporalProp { .. }
                    ) {
                        return Err(invalid(node, "not premise has an invalid type"));
                    }
                    operand_ty.clone()
                }
                HirUnaryOp::Negate => {
                    require_obligation(node, &[1, 2])?;
                    if operand_ty != CoreType::Int {
                        return Err(invalid(node, "negation premise is not Int"));
                    }
                    CoreType::Int
                }
            };
            require_term(
                checker,
                node,
                direct_core_id(hir.id())?,
                &result_ty,
                &CoreTermKind::Unary {
                    operator: map_unary(*operator),
                    operand: operand_id,
                },
            )?;
        }
        22..=33 => {
            require_premises(node, 2)?;
            let HirNodeKind::Binary {
                operator,
                left,
                right,
            } = hir.kind()
            else {
                return Err(invalid(node, "binary rule does not match HIR"));
            };
            if node.rule_tag != binary_rule_tag(*operator) {
                return Err(invalid(node, "binary rule tag is wrong"));
            }
            let left_premise = checker.premise(node, 0)?;
            let right_premise = checker.premise(node, 1)?;
            if left_premise.obligation.origin != *left || right_premise.obligation.origin != *right
            {
                return Err(invalid(node, "binary premise origin or order is wrong"));
            }
            let expected_premise_judgment = if node.obligation.judgment_tag == 3 {
                3
            } else {
                2
            };
            if left_premise.obligation.judgment_tag != expected_premise_judgment
                || right_premise.obligation.judgment_tag != expected_premise_judgment
            {
                return Err(invalid(node, "binary premise has the wrong judgment"));
            }
            let (left_id, left_ty) = premise_term(checker, node, 0)?;
            let (right_id, right_ty) = premise_term(checker, node, 1)?;
            let result_ty = match operator {
                HirBinaryOp::Or | HirBinaryOp::And | HirBinaryOp::Implies => {
                    if node.obligation.judgment_tag == 3 {
                        let Some((left_system, left_temporal)) = formula_type(&left_ty) else {
                            return Err(invalid(node, "logical formula left premise is unindexed"));
                        };
                        let Some((right_system, right_temporal)) = formula_type(&right_ty) else {
                            return Err(invalid(
                                node,
                                "logical formula right premise is unindexed",
                            ));
                        };
                        if left_system != right_system {
                            return Err(invalid(node, "logical formula systems disagree"));
                        }
                        if left_temporal || right_temporal {
                            CoreType::TemporalProp {
                                system: left_system,
                            }
                        } else {
                            CoreType::StateProp {
                                system: left_system,
                            }
                        }
                    } else {
                        require_obligation(node, &[1, 2])?;
                        if left_ty != CoreType::Bool || right_ty != CoreType::Bool {
                            return Err(invalid(node, "Boolean connective premises are not Bool"));
                        }
                        CoreType::Bool
                    }
                }
                HirBinaryOp::Equal | HirBinaryOp::NotEqual => {
                    require_obligation(node, &[1, 2])?;
                    if left_ty != right_ty || !left_ty.is_scalar() {
                        return Err(invalid(node, "equality premises have incompatible types"));
                    }
                    CoreType::Bool
                }
                HirBinaryOp::Less
                | HirBinaryOp::LessEqual
                | HirBinaryOp::Greater
                | HirBinaryOp::GreaterEqual => {
                    require_obligation(node, &[1, 2])?;
                    if left_ty != right_ty || !matches!(left_ty, CoreType::Nat | CoreType::Int) {
                        return Err(invalid(node, "comparison premises have incompatible types"));
                    }
                    CoreType::Bool
                }
                HirBinaryOp::Add | HirBinaryOp::Multiply => {
                    require_obligation(node, &[1, 2])?;
                    if left_ty != right_ty || !matches!(left_ty, CoreType::Nat | CoreType::Int) {
                        return Err(invalid(node, "arithmetic premises have incompatible types"));
                    }
                    left_ty.clone()
                }
                HirBinaryOp::Subtract => {
                    require_obligation(node, &[1, 2])?;
                    if left_ty != CoreType::Int || right_ty != CoreType::Int {
                        return Err(invalid(node, "subtraction premises are not Int"));
                    }
                    CoreType::Int
                }
            };
            require_term(
                checker,
                node,
                direct_core_id(hir.id())?,
                &result_ty,
                &CoreTermKind::Binary {
                    operator: map_binary(*operator),
                    left: left_id,
                    right: right_id,
                },
            )?;
        }
        34 => {
            require_obligation(node, &[1, 2])?;
            require_premises(node, 1)?;
            let HirNodeKind::IntFromNat { operand } = hir.kind() else {
                return Err(invalid(node, "IntFromNat rule does not match HIR"));
            };
            if checker.premise(node, 0)?.obligation.origin != *operand {
                return Err(invalid(node, "IntFromNat premise has the wrong origin"));
            }
            if checker.premise(node, 0)?.obligation.judgment_tag != 2 {
                return Err(invalid(node, "IntFromNat premise is not a check judgment"));
            }
            let (operand_id, operand_ty) = premise_term(checker, node, 0)?;
            if operand_ty != CoreType::Nat {
                return Err(invalid(node, "IntFromNat premise is not Nat"));
            }
            require_term(
                checker,
                node,
                direct_core_id(hir.id())?,
                &CoreType::Int,
                &CoreTermKind::IntFromNat {
                    operand: operand_id,
                },
            )?;
        }
        _ => return Err(invalid(node, "unknown operator rule")),
    }
    Ok(())
}

fn check_formula_rule(
    checker: &SemanticChecker<'_>,
    node: &RawDerivationNode,
) -> Result<(), KernelDiagnostic> {
    require_obligation(node, &[3])?;
    let hir = checker.index.node(node.obligation.origin)?;
    match node.rule_tag {
        40 => {
            require_premises(node, 1)?;
            require_none_witness(node)?;
            if matches!(
                hir.kind(),
                HirNodeKind::TypeBool
                    | HirNodeKind::TypeNat
                    | HirNodeKind::TypeInt
                    | HirNodeKind::TypeNamed(_)
                    | HirNodeKind::TypeOnce { .. }
                    | HirNodeKind::ProtocolTag { .. }
                    | HirNodeKind::Always { .. }
                    | HirNodeKind::Eventually { .. }
                    | HirNodeKind::Next { .. }
                    | HirNodeKind::Until { .. }
                    | HirNodeKind::Enabled { .. }
            ) {
                return Err(invalid(node, "state-predicate insertion wraps invalid HIR"));
            }
            let premise = checker.premise(node, 0)?;
            if premise.obligation
                != (RawObligation {
                    judgment_tag: 2,
                    origin: hir.id(),
                })
            {
                return Err(invalid(
                    node,
                    "state predicate lacks its Boolean check premise",
                ));
            }
            let (condition, condition_ty) = premise_term(checker, node, 0)?;
            if condition_ty != CoreType::Bool {
                return Err(invalid(node, "state predicate condition is not Bool"));
            }
            let (_, result_ty, term) = checker.term_conclusion(node)?;
            let CoreType::StateProp { system } = result_ty else {
                return Err(invalid(node, "state predicate conclusion is not StateProp"));
            };
            require_term(
                checker,
                node,
                CoreNodeId::from_origin(hir.id(), &[1]).map_err(|_| {
                    KernelDiagnostic::at(
                        KernelCode::CoreMismatch,
                        hir.id(),
                        "state-predicate insertion identity is invalid",
                    )
                })?,
                &result_ty,
                &CoreTermKind::StatePredicate { system, condition },
            )?;
            if !matches!(term.kind(), CoreTermKind::StatePredicate { .. }) {
                return Err(invalid(node, "state-predicate core constructor is missing"));
            }
        }
        41..=43 => {
            require_premises(node, 1)?;
            require_none_witness(node)?;
            let property_origin = match (node.rule_tag, hir.kind()) {
                (41, HirNodeKind::Always { property })
                | (42, HirNodeKind::Eventually { property })
                | (43, HirNodeKind::Next { property }) => *property,
                _ => return Err(invalid(node, "temporal-unary rule does not match HIR")),
            };
            let premise = checker.premise(node, 0)?;
            if premise.obligation
                != (RawObligation {
                    judgment_tag: 3,
                    origin: property_origin,
                })
            {
                return Err(invalid(
                    node,
                    "temporal-unary premise has the wrong obligation",
                ));
            }
            let (property, property_ty) = premise_term(checker, node, 0)?;
            let Some((system, _)) = formula_type(&property_ty) else {
                return Err(invalid(node, "temporal-unary premise is not a proposition"));
            };
            let kind = match node.rule_tag {
                41 => CoreTermKind::Always { system, property },
                42 => CoreTermKind::Eventually { system, property },
                43 => CoreTermKind::Next { system, property },
                _ => return Err(invalid(node, "temporal-unary rule tag is invalid")),
            };
            require_term(
                checker,
                node,
                direct_core_id(hir.id())?,
                &CoreType::TemporalProp { system },
                &kind,
            )?;
        }
        44 => {
            require_premises(node, 2)?;
            require_none_witness(node)?;
            let HirNodeKind::Until { left, right } = hir.kind() else {
                return Err(invalid(node, "until rule does not match HIR"));
            };
            let left_premise = checker.premise(node, 0)?;
            let right_premise = checker.premise(node, 1)?;
            if left_premise.obligation
                != (RawObligation {
                    judgment_tag: 3,
                    origin: *left,
                })
                || right_premise.obligation
                    != (RawObligation {
                        judgment_tag: 3,
                        origin: *right,
                    })
            {
                return Err(invalid(node, "until premise origins or order are wrong"));
            }
            let (left_id, left_ty) = premise_term(checker, node, 0)?;
            let (right_id, right_ty) = premise_term(checker, node, 1)?;
            let Some((left_system, _)) = formula_type(&left_ty) else {
                return Err(invalid(node, "until left premise is not a proposition"));
            };
            let Some((right_system, _)) = formula_type(&right_ty) else {
                return Err(invalid(node, "until right premise is not a proposition"));
            };
            if left_system != right_system {
                return Err(invalid(node, "until premise systems disagree"));
            }
            require_term(
                checker,
                node,
                direct_core_id(hir.id())?,
                &CoreType::TemporalProp {
                    system: left_system,
                },
                &CoreTermKind::Until {
                    system: left_system,
                    left: left_id,
                    right: right_id,
                },
            )?;
        }
        45 => {
            require_premises(node, 1)?;
            let HirNodeKind::Enabled {
                action_origin,
                action: ResolvedRef::Definition(action),
            } = hir.kind()
            else {
                return Err(invalid(node, "enabled rule does not match HIR"));
            };
            let premise = checker.premise(node, 0)?;
            if premise.obligation
                != (RawObligation {
                    judgment_tag: 8,
                    origin: *action_origin,
                })
                || premise.conclusion != DerivationConclusion::Definition(*action)
            {
                return Err(invalid(node, "enabled action-use premise is wrong"));
            }
            let meta = checker
                .index
                .declarations
                .get(action)
                .ok_or(KernelDiagnostic::new(
                    KernelCode::Coverage,
                    "enabled action definition is missing",
                ))?;
            if meta.declaration.key().namespace() != Some(Namespace::Action) {
                return Err(invalid(node, "enabled target is not an action"));
            }
            let system = meta
                .parent
                .ok_or(invalid(node, "enabled action has no system"))?;
            if node.witness
                != (DerivationWitness::SystemDefinition {
                    system,
                    definition: *action,
                })
            {
                return Err(invalid(node, "enabled witness is wrong"));
            }
            require_term(
                checker,
                node,
                direct_core_id(hir.id())?,
                &CoreType::StateProp { system },
                &CoreTermKind::Enabled {
                    system,
                    action: *action,
                },
            )?;
        }
        _ => return Err(invalid(node, "unknown formula rule")),
    }
    Ok(())
}

fn check_definition_use_rule(
    checker: &SemanticChecker<'_>,
    node: &RawDerivationNode,
) -> Result<(), KernelDiagnostic> {
    require_premises(node, 0)?;
    let (judgment, namespace) = match node.rule_tag {
        50 => (6, Namespace::State),
        51 => (7, Namespace::Capability),
        52 => (8, Namespace::Action),
        _ => return Err(invalid(node, "unknown definition-use rule")),
    };
    require_obligation(node, &[judgment])?;
    let hir = checker.index.node(node.obligation.origin)?;
    let HirNodeKind::Reference(reference) = hir.kind() else {
        return Err(invalid(
            node,
            "definition-use rule does not match reference HIR",
        ));
    };
    let definition = reference
        .terminal_definition()
        .ok_or(invalid(node, "definition-use reference has no definition"))?;
    if declaration_namespace(checker, definition)? != namespace
        || node.conclusion != DerivationConclusion::Definition(definition)
        || node.witness != DerivationWitness::Definition(definition)
    {
        return Err(invalid(node, "definition-use reconstruction failed"));
    }
    Ok(())
}

fn derivation_term_id(
    checker: &SemanticChecker<'_>,
    judgment_tag: u8,
    origin: NodeId,
) -> Result<(CoreNodeId, CoreType), KernelDiagnostic> {
    let derivation = checker.derivation_for(judgment_tag, origin)?;
    let (id, ty, _) = checker.term_conclusion(derivation)?;
    Ok((id, ty.clone()))
}

fn check_aggregate_core(checker: &SemanticChecker<'_>) -> Result<(), KernelDiagnostic> {
    let expected_module_ids = checker
        .hir
        .modules()
        .values()
        .map(|module| module.id())
        .collect::<BTreeSet<_>>();
    if checker
        .core
        .modules()
        .keys()
        .copied()
        .collect::<BTreeSet<_>>()
        != expected_module_ids
    {
        return Err(KernelDiagnostic::new(
            KernelCode::CoreMismatch,
            "core module set differs from resolved HIR",
        ));
    }

    for hir_module in checker.hir.modules().values() {
        let core_module = &checker.core.modules()[&hir_module.id()];
        let expected_imports = hir_module
            .imports()
            .iter()
            .map(|import| import.module_id())
            .collect::<BTreeSet<_>>();
        if core_module.imports() != &expected_imports {
            return Err(KernelDiagnostic::new(
                KernelCode::CoreMismatch,
                "core imports differ from resolved HIR",
            ));
        }
        let module_declarations = checker
            .index
            .declarations
            .iter()
            .filter(|(_, meta)| meta.module == hir_module.id())
            .collect::<Vec<_>>();
        for (_, meta) in &module_declarations {
            let namespace = meta
                .declaration
                .key()
                .namespace()
                .ok_or(KernelDiagnostic::new(
                    KernelCode::Coverage,
                    "resolved declaration has no namespace",
                ))?;
            if !matches!(
                namespace,
                Namespace::Type
                    | Namespace::Constructor
                    | Namespace::System
                    | Namespace::State
                    | Namespace::Action
                    | Namespace::Capability
                    | Namespace::Property
            ) {
                return Err(KernelDiagnostic::new(
                    KernelCode::CoreMismatch,
                    "resolved declaration is outside the M9-v1 core fragment",
                ));
            }
        }

        let expected_enums = module_declarations
            .iter()
            .filter(|(_, meta)| meta.declaration.key().namespace() == Some(Namespace::Type))
            .map(|(id, _)| **id)
            .collect::<BTreeSet<_>>();
        if core_module
            .enumerations()
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            != expected_enums
        {
            return Err(KernelDiagnostic::new(
                KernelCode::CoreMismatch,
                "core enumeration set differs from resolved HIR",
            ));
        }
        for enumeration in &expected_enums {
            let expected_constructors = module_declarations
                .iter()
                .filter(|(_, meta)| {
                    meta.parent == Some(*enumeration)
                        && meta.declaration.key().namespace() == Some(Namespace::Constructor)
                })
                .map(|(id, _)| **id)
                .collect::<BTreeSet<_>>();
            if core_module.enumerations()[enumeration].constructors() != &expected_constructors {
                return Err(KernelDiagnostic::new(
                    KernelCode::CoreMismatch,
                    "core enumeration constructors differ from resolved HIR",
                ));
            }
        }

        let expected_systems = module_declarations
            .iter()
            .filter(|(_, meta)| meta.declaration.key().namespace() == Some(Namespace::System))
            .map(|(id, _)| **id)
            .collect::<BTreeSet<_>>();
        if core_module
            .systems()
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            != expected_systems
        {
            return Err(KernelDiagnostic::new(
                KernelCode::CoreMismatch,
                "core system set differs from resolved HIR",
            ));
        }
        for system in expected_systems {
            check_system(
                checker,
                &module_declarations,
                system,
                &core_module.systems()[&system],
            )?;
        }
    }
    Ok(())
}

fn check_system(
    checker: &SemanticChecker<'_>,
    declarations: &[(&DefId, &DeclMeta<'_>)],
    system: DefId,
    core_system: &nmlt_ir::CoreSystem,
) -> Result<(), KernelDiagnostic> {
    let members = declarations
        .iter()
        .filter(|(_, meta)| meta.parent == Some(system))
        .map(|(id, meta)| (**id, *meta))
        .collect::<Vec<_>>();
    let state_ids = members
        .iter()
        .filter(|(_, meta)| meta.declaration.key().namespace() == Some(Namespace::State))
        .map(|(id, _)| *id)
        .collect::<BTreeSet<_>>();
    if core_system.state().keys().copied().collect::<BTreeSet<_>>() != state_ids {
        return Err(KernelDiagnostic::new(
            KernelCode::CoreMismatch,
            "core state set differs from resolved HIR",
        ));
    }
    for state in &state_ids {
        let type_root = checker
            .index
            .root_exact(*state, &[SemanticPathSegment::DeclaredType])?;
        let init_root = checker
            .index
            .root_exact(*state, &[SemanticPathSegment::Initializer])?;
        let ty = type_conclusion(checker.derivation_for(4, type_root.node())?)?;
        let (initializer, initializer_ty) = derivation_term_id(checker, 2, init_root.node())?;
        let core_state = &core_system.state()[state];
        if core_state.id() != *state
            || core_state.ty() != ty
            || initializer_ty != *ty
            || core_state.initializer() != initializer
        {
            return Err(KernelDiagnostic::new(
                KernelCode::CoreMismatch,
                "core state declaration differs from HIR derivations",
            ));
        }
    }

    let capability_ids = members
        .iter()
        .filter(|(_, meta)| meta.declaration.key().namespace() == Some(Namespace::Capability))
        .map(|(id, _)| *id)
        .collect::<BTreeSet<_>>();
    if core_system
        .capabilities()
        .keys()
        .copied()
        .collect::<BTreeSet<_>>()
        != capability_ids
    {
        return Err(KernelDiagnostic::new(
            KernelCode::CoreMismatch,
            "core capability set differs from resolved HIR",
        ));
    }
    for capability in &capability_ids {
        let type_root = checker
            .index
            .root_exact(*capability, &[SemanticPathSegment::DeclaredType])?;
        let CoreType::Once { protocol } =
            type_conclusion(checker.derivation_for(4, type_root.node())?)?
        else {
            return Err(KernelDiagnostic::new(
                KernelCode::CoreMismatch,
                "capability does not derive a Once type",
            ));
        };
        let core_capability = &core_system.capabilities()[capability];
        if core_capability.id() != *capability || core_capability.protocol() != *protocol {
            return Err(KernelDiagnostic::new(
                KernelCode::CoreMismatch,
                "core capability differs from HIR derivation",
            ));
        }
    }

    let action_ids = members
        .iter()
        .filter(|(_, meta)| meta.declaration.key().namespace() == Some(Namespace::Action))
        .map(|(id, _)| *id)
        .collect::<BTreeSet<_>>();
    if core_system
        .actions()
        .keys()
        .copied()
        .collect::<BTreeSet<_>>()
        != action_ids
    {
        return Err(KernelDiagnostic::new(
            KernelCode::CoreMismatch,
            "core action set differs from resolved HIR",
        ));
    }
    for action in &action_ids {
        check_action(
            checker,
            system,
            *action,
            &state_ids,
            &core_system.actions()[action],
        )?;
    }

    let property_ids = members
        .iter()
        .filter(|(_, meta)| meta.declaration.key().namespace() == Some(Namespace::Property))
        .map(|(id, meta)| (*id, meta.declaration.flavor()))
        .collect::<BTreeMap<_, _>>();
    if core_system
        .properties()
        .keys()
        .copied()
        .collect::<BTreeSet<_>>()
        != property_ids.keys().copied().collect()
    {
        return Err(KernelDiagnostic::new(
            KernelCode::CoreMismatch,
            "core property set differs from resolved HIR",
        ));
    }
    for (property, flavor) in property_ids {
        let root = checker
            .index
            .root_exact(property, &[SemanticPathSegment::PropertyBody])?;
        let (body, ty) = derivation_term_id(checker, 3, root.node())?;
        let expected_kind = match flavor {
            DeclarationFlavor::SafetyProperty => CorePropertyKind::Safety,
            DeclarationFlavor::TemporalProperty => CorePropertyKind::Temporal,
            DeclarationFlavor::Ordinary => {
                return Err(KernelDiagnostic::new(
                    KernelCode::CoreMismatch,
                    "property declaration has no semantic flavor",
                ));
            }
        };
        let core_property = &core_system.properties()[&property];
        if ty != (CoreType::TemporalProp { system })
            || core_property.id() != property
            || core_property.system() != system
            || core_property.kind() != expected_kind
            || core_property.body() != body
        {
            return Err(KernelDiagnostic::new(
                KernelCode::CoreMismatch,
                "core property differs from HIR derivation",
            ));
        }
    }

    let observation_roots = checker
        .index
        .roots(system)
        .filter(|root| {
            matches!(
                root.semantic_path().segments(),
                [SemanticPathSegment::ObservationItem(_)]
            )
        })
        .collect::<Vec<_>>();
    if observation_roots.is_empty() {
        if !core_system.observations().is_empty() {
            return Err(KernelDiagnostic::new(
                KernelCode::CoreMismatch,
                "core contains an observation absent from HIR",
            ));
        }
    } else {
        if core_system.observations().len() != 1 {
            return Err(KernelDiagnostic::new(
                KernelCode::CoreMismatch,
                "core observation contract count differs from HIR",
            ));
        }
        let mut items = Vec::new();
        for root in &observation_roots {
            let (item, ty) = derivation_term_id(checker, 1, root.node())?;
            if !ty.is_scalar() {
                return Err(KernelDiagnostic::at(
                    KernelCode::InvalidRule,
                    root.node(),
                    "observation item does not synthesize a scalar type",
                ));
            }
            items.push(item);
        }
        let observation = &core_system.observations()[0];
        if observation.owner() != system
            || observation.origin() != observation_roots[0].node()
            || observation.items() != items
        {
            return Err(KernelDiagnostic::new(
                KernelCode::CoreMismatch,
                "core observation differs from HIR derivations",
            ));
        }
    }
    Ok(())
}

fn check_action(
    checker: &SemanticChecker<'_>,
    system: DefId,
    action: DefId,
    all_state: &BTreeSet<DefId>,
    core_action: &nmlt_ir::CoreAction,
) -> Result<(), KernelDiagnostic> {
    let expected_parameters = checker
        .index
        .locals
        .values()
        .filter(|binder| binder.owner() == action)
        .map(|binder| Ok((binder.id(), local_type(checker, binder.id())?)))
        .collect::<Result<BTreeMap<_, _>, KernelDiagnostic>>()?;
    if core_action.id() != action
        || core_action.system() != system
        || core_action.parameters().len() != expected_parameters.len()
    {
        return Err(KernelDiagnostic::new(
            KernelCode::CoreMismatch,
            "core action header differs from resolved HIR",
        ));
    }
    for (local, ty) in &expected_parameters {
        let Some(parameter) = core_action.parameters().get(local) else {
            return Err(KernelDiagnostic::new(
                KernelCode::CoreMismatch,
                "core action parameter is missing",
            ));
        };
        if parameter.id() != *local || parameter.ty() != ty {
            return Err(KernelDiagnostic::new(
                KernelCode::CoreMismatch,
                "core action parameter type differs from HIR",
            ));
        }
    }

    let mut guards = Vec::new();
    let mut updates = BTreeMap::new();
    let mut outputs = Vec::new();
    let mut consumes = BTreeSet::new();
    for root in checker.index.roots(action) {
        match root.semantic_path().segments() {
            [SemanticPathSegment::Guard(_)] => {
                let (guard, ty) = derivation_term_id(checker, 2, root.node())?;
                if ty != CoreType::Bool {
                    return Err(KernelDiagnostic::at(
                        KernelCode::InvalidRule,
                        root.node(),
                        "action guard does not check as Bool",
                    ));
                }
                guards.push(guard);
            }
            [SemanticPathSegment::UpdateTarget(state)] => {
                let derivation = checker.derivation_for(6, root.node())?;
                if derivation.conclusion != DerivationConclusion::Definition(*state)
                    || !all_state.contains(state)
                {
                    return Err(invalid(derivation, "action update target is invalid"));
                }
            }
            [SemanticPathSegment::UpdateValue(state)] => {
                let (value, ty) = derivation_term_id(checker, 2, root.node())?;
                if ty != state_type(checker, *state)? || updates.insert(*state, value).is_some() {
                    return Err(KernelDiagnostic::at(
                        KernelCode::InvalidRule,
                        root.node(),
                        "action update value or target multiplicity is invalid",
                    ));
                }
            }
            [SemanticPathSegment::Output(_)] => {
                let (output, ty) = derivation_term_id(checker, 1, root.node())?;
                if !ty.is_scalar() {
                    return Err(KernelDiagnostic::at(
                        KernelCode::InvalidRule,
                        root.node(),
                        "action output does not synthesize a scalar type",
                    ));
                }
                outputs.push(output);
            }
            [SemanticPathSegment::Consume(_)] => {
                let derivation = checker.derivation_for(7, root.node())?;
                let DerivationConclusion::Definition(capability) = derivation.conclusion else {
                    return Err(invalid(
                        derivation,
                        "consume conclusion is not a capability",
                    ));
                };
                let meta = checker
                    .index
                    .declarations
                    .get(&capability)
                    .ok_or(invalid(derivation, "consumed capability is missing"))?;
                if meta.parent != Some(system)
                    || meta.declaration.key().namespace() != Some(Namespace::Capability)
                    || !consumes.insert(capability)
                {
                    return Err(invalid(
                        derivation,
                        "capability is foreign, non-capability, or consumed twice",
                    ));
                }
            }
            [
                SemanticPathSegment::ActionParameter(_),
                SemanticPathSegment::DeclaredType,
            ] => {}
            _ => {
                return Err(KernelDiagnostic::at(
                    KernelCode::Coverage,
                    root.node(),
                    "action has an unsupported semantic root",
                ));
            }
        }
    }
    let frames = all_state
        .difference(&updates.keys().copied().collect())
        .copied()
        .collect::<BTreeSet<_>>();
    if core_action.guards() != guards
        || core_action.updates() != &updates
        || core_action.frames() != &frames
        || core_action.outputs() != outputs
        || core_action.consumes() != &consumes
    {
        return Err(KernelDiagnostic::new(
            KernelCode::CoreMismatch,
            "core action body differs from HIR derivations",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod adversarial_tests {
    use nmlt_elaborate::elaborate;
    use nmlt_hir::{project_source_module, resolve_modules};

    use super::{KernelCode, certificate_digest, check, derivation_digest};
    use crate::RawCertificate;

    #[test]
    fn canonically_resealed_semantic_forgery_is_rejected() {
        let projected = project_source_module(
            "Forgery",
            "src/forgery.nmlt",
            b"system Forgery {\n state ready: Bool = true\n safety Safe = always(ready)\n observe ready\n}\n",
        );
        assert!(projected.projection_issues().is_empty());
        let hir = resolve_modules(vec![projected]).unwrap();
        let artifact = elaborate(&hir).unwrap();
        let mut raw = RawCertificate::from_artifact(&artifact);

        let node_index = raw
            .derivations
            .iter()
            .position(|node| {
                node.rule_tag == 10
                    && raw
                        .required_roots
                        .iter()
                        .any(|root| root.derivation_digest == node.claimed_digest)
            })
            .unwrap();
        let node = &mut raw.derivations[node_index];
        let old_digest = node.claimed_digest;
        node.rule_tag = 11;
        node.claimed_digest = derivation_digest(node);
        let new_digest = node.claimed_digest;
        raw.required_roots
            .iter_mut()
            .find(|root| root.derivation_digest == old_digest)
            .unwrap()
            .derivation_digest = new_digest;
        raw.derivations
            .sort_by_key(|derivation| derivation.claimed_digest);
        raw.certificate_digest = certificate_digest(&raw).0;

        assert_eq!(
            check(&hir, artifact.core_program(), &raw)
                .unwrap_err()
                .code(),
            KernelCode::InvalidRule
        );
    }
}
