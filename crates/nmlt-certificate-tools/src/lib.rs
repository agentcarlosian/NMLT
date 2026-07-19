//! Untrusted preprocessing and metrics for raw NMLT certificates.
//!
//! Nothing in this crate confers checked status. Every transformed certificate
//! must still be independently replayed by `nmlt-kernel`.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use nmlt_kernel::{RawCertificate, RawDerivationNode};

/// Structural proof-DAG measurements, independent of semantic acceptance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProofMetrics {
    pub required_roots: usize,
    pub derivation_nodes: usize,
    pub reachable_nodes: usize,
    pub unreachable_nodes: usize,
    pub premise_edges: usize,
    pub maximum_depth: usize,
    pub maximum_fan_in: usize,
    pub canonical_bytes: usize,
}

/// What an untrusted simplification changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimplificationReport {
    pub before: ProofMetrics,
    pub after: ProofMetrics,
    pub removed_nodes: usize,
}

/// A malformed proof graph that the simplifier refuses to guess about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimplifyError {
    DuplicateDigest([u8; 32]),
    MissingDerivation([u8; 32]),
    ReachableCycle([u8; 32]),
}

impl fmt::Display for SimplifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, digest) = match self {
            Self::DuplicateDigest(digest) => ("duplicate derivation digest", digest),
            Self::MissingDerivation(digest) => ("missing derivation", digest),
            Self::ReachableCycle(digest) => ("reachable derivation cycle", digest),
        };
        write!(formatter, "{kind}: ")?;
        for byte in digest {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl std::error::Error for SimplifyError {}

struct Graph<'a> {
    nodes: BTreeMap<[u8; 32], &'a RawDerivationNode>,
}

impl<'a> Graph<'a> {
    fn new(certificate: &'a RawCertificate) -> Result<Self, SimplifyError> {
        let mut nodes = BTreeMap::new();
        for node in &certificate.derivations {
            if nodes.insert(node.claimed_digest, node).is_some() {
                return Err(SimplifyError::DuplicateDigest(node.claimed_digest));
            }
        }
        Ok(Self { nodes })
    }

    fn reachable_depths(
        &self,
        certificate: &RawCertificate,
    ) -> Result<BTreeMap<[u8; 32], usize>, SimplifyError> {
        let mut depths = BTreeMap::new();
        let mut visiting = BTreeSet::new();
        for root in &certificate.required_roots {
            let mut stack = vec![(root.derivation_digest, false)];
            while let Some((digest, expanded)) = stack.pop() {
                if depths.contains_key(&digest) {
                    continue;
                }
                let node = self
                    .nodes
                    .get(&digest)
                    .ok_or(SimplifyError::MissingDerivation(digest))?;
                if expanded {
                    let depth = node
                        .premises
                        .iter()
                        .map(|premise| depths.get(premise).copied().unwrap_or(0) + 1)
                        .max()
                        .unwrap_or(1);
                    visiting.remove(&digest);
                    depths.insert(digest, depth);
                    continue;
                }
                if !visiting.insert(digest) {
                    return Err(SimplifyError::ReachableCycle(digest));
                }
                stack.push((digest, true));
                for premise in node.premises.iter().rev() {
                    if !depths.contains_key(premise) {
                        stack.push((*premise, false));
                    }
                }
            }
        }
        Ok(depths)
    }
}

/// Measure the claimed proof graph. This is structural analysis, not checking.
pub fn measure(certificate: &RawCertificate) -> Result<ProofMetrics, SimplifyError> {
    let graph = Graph::new(certificate)?;
    let depths = graph.reachable_depths(certificate)?;
    let premise_edges = certificate
        .derivations
        .iter()
        .map(|node| node.premises.len())
        .sum();
    Ok(ProofMetrics {
        required_roots: certificate.required_roots.len(),
        derivation_nodes: certificate.derivations.len(),
        reachable_nodes: depths.len(),
        unreachable_nodes: certificate.derivations.len().saturating_sub(depths.len()),
        premise_edges,
        maximum_depth: depths.values().copied().max().unwrap_or(0),
        maximum_fan_in: certificate
            .derivations
            .iter()
            .map(|node| node.premises.len())
            .max()
            .unwrap_or(0),
        canonical_bytes: certificate.to_canonical_bytes().len(),
    })
}

/// Prune nodes unreachable from required roots and restore canonical order.
///
/// The result remains untrusted. In particular, this function intentionally
/// does not validate claimed node digests, judgments, witnesses, or rules.
pub fn simplify(
    mut certificate: RawCertificate,
) -> Result<(RawCertificate, SimplificationReport), SimplifyError> {
    let before = measure(&certificate)?;
    let reachable = Graph::new(&certificate)?.reachable_depths(&certificate)?;
    certificate
        .derivations
        .retain(|node| reachable.contains_key(&node.claimed_digest));
    certificate
        .derivations
        .sort_by_key(|node| node.claimed_digest);
    certificate.required_roots.sort_by_key(|root| {
        (
            root.obligation.judgment_tag,
            root.obligation.origin,
            root.derivation_digest,
        )
    });
    certificate.recompute_claimed_certificate_digest();
    let after = measure(&certificate)?;
    Ok((
        certificate,
        SimplificationReport {
            before,
            after,
            removed_nodes: before.derivation_nodes - after.derivation_nodes,
        },
    ))
}
