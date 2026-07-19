use std::collections::BTreeSet;

use crate::graph::{FiniteGraph, StateId, TransitionKind};
use crate::refinement::RefinementSpec;

const DOMAIN: &[u8] = b"NMLT-COINDUCTIVE-REFINEMENT-CERTIFICATE\0v1\0";

// Kept local so the independent temporal experiment does not alter the
// promoted compiler dependency graph. The known-vector test below guards the
// same audited SHA-256 primitive used by other NMLT identity domains.
fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    const INITIAL: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut padded = bytes.to_vec();
    let bit_len = (padded.len() as u64).wrapping_mul(8);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    let mut state = INITIAL;
    for block in padded.chunks_exact(64) {
        let mut words = [0_u32; 64];
        for (index, chunk) in block.chunks_exact(4).enumerate() {
            words[index] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sum1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sum0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *slot = slot.wrapping_add(value);
        }
    }

    let mut digest = [0_u8; 32];
    for (chunk, value) in digest.chunks_exact_mut(4).zip(state) {
        chunk.copy_from_slice(&value.to_be_bytes());
    }
    digest
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatePair {
    pub concrete: StateId,
    pub abstract_state: StateId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoinductiveCertificate {
    pub subject_id: [u8; 32],
    pub roots: Vec<StatePair>,
    pub relation: Vec<StatePair>,
    pub claimed_id: [u8; 32],
}

impl CoinductiveCertificate {
    #[must_use]
    pub fn canonical(
        subject_id: [u8; 32],
        mut roots: Vec<StatePair>,
        mut relation: Vec<StatePair>,
    ) -> Self {
        roots.sort();
        roots.dedup();
        relation.sort();
        relation.dedup();
        let mut certificate = Self {
            subject_id,
            roots,
            relation,
            claimed_id: [0; 32],
        };
        certificate.claimed_id = certificate.recomputed_id();
        certificate
    }

    #[must_use]
    pub fn recomputed_id(&self) -> [u8; 32] {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(DOMAIN);
        bytes.extend_from_slice(&self.subject_id);
        encode_pairs(&mut bytes, &self.roots);
        encode_pairs(&mut bytes, &self.relation);
        sha256_bytes(&bytes)
    }
}

fn encode_pairs(bytes: &mut Vec<u8>, pairs: &[StatePair]) {
    bytes.extend_from_slice(&(pairs.len() as u64).to_be_bytes());
    for pair in pairs {
        bytes.extend_from_slice(&(pair.concrete as u64).to_be_bytes());
        bytes.extend_from_slice(&(pair.abstract_state as u64).to_be_bytes());
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertificateIssue {
    StaleSubject,
    IdentityMismatch,
    NonCanonical,
    PairOutOfRange(StatePair),
    RootMissing(StatePair),
    ObservationMismatch(StatePair),
    ActionUnmapped(StatePair, String),
    OpenStep(StatePair, String, StateId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertificateReport {
    pub accepted: bool,
    pub checked_pairs: usize,
    pub issues: Vec<CertificateIssue>,
}

pub struct CoinductiveCertificateChecker;

impl CoinductiveCertificateChecker {
    /// Check a finite post-fixed one-sided simulation relation.
    ///
    /// The subject identity is supplied by the source/evidence layer and is
    /// checked before graph obligations. Acceptance is safety/refinement only;
    /// it does not transport fairness or rule out hidden divergence.
    pub fn check(
        expected_subject_id: [u8; 32],
        concrete: &FiniteGraph,
        abstract_graph: &FiniteGraph,
        spec: &RefinementSpec,
        certificate: &CoinductiveCertificate,
    ) -> CertificateReport {
        let mut issues = Vec::new();
        if certificate.subject_id != expected_subject_id {
            issues.push(CertificateIssue::StaleSubject);
        }
        if certificate.claimed_id != certificate.recomputed_id() {
            issues.push(CertificateIssue::IdentityMismatch);
        }
        if !strictly_sorted(&certificate.roots) || !strictly_sorted(&certificate.relation) {
            issues.push(CertificateIssue::NonCanonical);
        }
        let relation = certificate
            .relation
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        for root in &certificate.roots {
            if !relation.contains(root) {
                issues.push(CertificateIssue::RootMissing(*root));
            }
        }
        for pair in &certificate.relation {
            if pair.concrete >= concrete.states().len()
                || pair.abstract_state >= abstract_graph.states().len()
            {
                issues.push(CertificateIssue::PairOutOfRange(*pair));
                continue;
            }
            let concrete_observation = spec
                .concrete_observation
                .observe(concrete.state(pair.concrete));
            let abstract_observation = spec
                .abstract_observation
                .observe(abstract_graph.state(pair.abstract_state));
            match (concrete_observation, abstract_observation) {
                (Ok(left), Ok(right)) if left == right => {}
                _ => {
                    issues.push(CertificateIssue::ObservationMismatch(*pair));
                    continue;
                }
            }
            for &transition_id in concrete.outgoing_ids(pair.concrete) {
                let transition = concrete.transition(transition_id);
                let TransitionKind::Action(action) = &transition.kind else {
                    continue;
                };
                let Some(projected) = spec.actions.get(action) else {
                    issues.push(CertificateIssue::ActionUnmapped(*pair, action.clone()));
                    continue;
                };
                let closed = match projected {
                    None => relation.contains(&StatePair {
                        concrete: transition.to,
                        abstract_state: pair.abstract_state,
                    }),
                    Some(abstract_action) => abstract_graph
                        .outgoing_ids(pair.abstract_state)
                        .iter()
                        .copied()
                        .any(|abstract_transition_id| {
                            let abstract_transition =
                                abstract_graph.transition(abstract_transition_id);
                            abstract_transition.kind.action() == Some(abstract_action)
                                && relation.contains(&StatePair {
                                    concrete: transition.to,
                                    abstract_state: abstract_transition.to,
                                })
                        }),
                };
                if !closed {
                    issues.push(CertificateIssue::OpenStep(
                        *pair,
                        action.clone(),
                        transition.to,
                    ));
                }
            }
        }
        CertificateReport {
            accepted: issues.is_empty(),
            checked_pairs: certificate.relation.len(),
            issues,
        }
    }
}

fn strictly_sorted(values: &[StatePair]) -> bool {
    values.windows(2).all(|window| window[0] < window[1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ModelState, Transition, Value};
    use crate::observation::{ActionHiding, ObservationMap};
    use std::collections::BTreeMap;

    fn state(visible: bool) -> ModelState {
        BTreeMap::from([("visible".to_owned(), Value::Bool(visible))])
    }

    fn fixture() -> (FiniteGraph, FiniteGraph, RefinementSpec) {
        let concrete = FiniteGraph::new(
            vec![state(false), state(false), state(true)],
            vec![0],
            vec![
                Transition::action(0, "cache", 1),
                Transition::action(1, "publish", 2),
            ],
        )
        .unwrap();
        let abstract_graph = FiniteGraph::new(
            vec![state(false), state(true)],
            vec![0],
            vec![Transition::action(0, "commit", 1)],
        )
        .unwrap();
        let spec = RefinementSpec {
            state_map: vec![0, 0, 1],
            concrete_observation: ObservationMap::identity(["visible"]),
            abstract_observation: ObservationMap::identity(["visible"]),
            actions: ActionHiding::new([("cache", None::<&str>), ("publish", Some("commit"))]),
        };
        (concrete, abstract_graph, spec)
    }

    fn accepted(subject: [u8; 32]) -> CoinductiveCertificate {
        CoinductiveCertificate::canonical(
            subject,
            vec![StatePair {
                concrete: 0,
                abstract_state: 0,
            }],
            vec![
                StatePair {
                    concrete: 0,
                    abstract_state: 0,
                },
                StatePair {
                    concrete: 1,
                    abstract_state: 0,
                },
                StatePair {
                    concrete: 2,
                    abstract_state: 1,
                },
            ],
        )
    }

    #[test]
    fn local_sha256_matches_the_standard_vector() {
        assert_eq!(
            sha256_bytes(b"abc"),
            [
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                0xf2, 0x00, 0x15, 0xad,
            ]
        );
    }

    #[test]
    fn accepts_a_closed_identity_bound_postfixed_relation() {
        let (concrete, abstract_graph, spec) = fixture();
        let certificate = accepted([7; 32]);
        let report = CoinductiveCertificateChecker::check(
            [7; 32],
            &concrete,
            &abstract_graph,
            &spec,
            &certificate,
        );
        assert!(report.accepted, "{:#?}", report.issues);
    }

    #[test]
    fn stale_forged_and_open_certificates_fail_closed() {
        let (concrete, abstract_graph, spec) = fixture();
        let certificate = accepted([7; 32]);
        assert!(matches!(
            CoinductiveCertificateChecker::check(
                [8; 32],
                &concrete,
                &abstract_graph,
                &spec,
                &certificate,
            )
            .issues
            .as_slice(),
            [CertificateIssue::StaleSubject]
        ));

        let mut forged = certificate.clone();
        forged.claimed_id[0] ^= 1;
        assert!(
            CoinductiveCertificateChecker::check(
                [7; 32],
                &concrete,
                &abstract_graph,
                &spec,
                &forged,
            )
            .issues
            .contains(&CertificateIssue::IdentityMismatch)
        );

        let open = CoinductiveCertificate::canonical(
            [7; 32],
            vec![StatePair {
                concrete: 0,
                abstract_state: 0,
            }],
            vec![StatePair {
                concrete: 0,
                abstract_state: 0,
            }],
        );
        assert!(CoinductiveCertificateChecker::check(
            [7; 32], &concrete, &abstract_graph, &spec, &open,
        ).issues.iter().any(|issue| matches!(issue, CertificateIssue::OpenStep(..))));
    }
}
