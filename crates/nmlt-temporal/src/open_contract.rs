//! Canonical finite payload types and assume/guarantee predicates for M11-001b.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const PAYLOAD_TYPE_DOMAIN: &[u8] = b"NMLT-PAYLOAD-TYPE\0v1\0";
const PAYLOAD_PREDICATE_DOMAIN: &[u8] = b"NMLT-PAYLOAD-PREDICATE\0v1\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PayloadTypeId([u8; 32]);

impl PayloadTypeId {
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for PayloadTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "nmlt-payload-type-v1:sha256:{}", hex(&self.0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PayloadPredicateId([u8; 32]);

impl PayloadPredicateId {
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for PayloadPredicateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "nmlt-payload-predicate-v1:sha256:{}", hex(&self.0))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PayloadTypeError {
    InvalidName(String),
    EmptyVariants,
    InvalidVariant(String),
    DuplicateVariant(String),
}

impl fmt::Display for PayloadTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(name) => write!(f, "invalid finite payload type name {name:?}"),
            Self::EmptyVariants => write!(f, "a finite payload type needs at least one variant"),
            Self::InvalidVariant(variant) => {
                write!(f, "invalid finite payload variant {variant:?}")
            }
            Self::DuplicateVariant(variant) => {
                write!(f, "finite payload variant {variant:?} is duplicated")
            }
        }
    }
}

impl std::error::Error for PayloadTypeError {}

/// A finite nominal payload type with a canonical order-independent identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PayloadType {
    name: String,
    variants: BTreeSet<String>,
    id: PayloadTypeId,
}

impl PayloadType {
    pub fn enumeration<N, I, V>(name: N, variants: I) -> Result<Self, PayloadTypeError>
    where
        N: Into<String>,
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        let name = name.into();
        if !canonical_atom(&name) {
            return Err(PayloadTypeError::InvalidName(name));
        }
        let mut canonical_variants = BTreeSet::new();
        for variant in variants {
            let variant = variant.into();
            if !canonical_atom(&variant) {
                return Err(PayloadTypeError::InvalidVariant(variant));
            }
            if !canonical_variants.insert(variant.clone()) {
                return Err(PayloadTypeError::DuplicateVariant(variant));
            }
        }
        if canonical_variants.is_empty() {
            return Err(PayloadTypeError::EmptyVariants);
        }
        let id = payload_type_id(&name, &canonical_variants);
        Ok(Self {
            name,
            variants: canonical_variants,
            id,
        })
    }

    #[must_use]
    pub fn unit() -> Self {
        Self::enumeration("Unit", ["unit"]).expect("the built-in unit payload is canonical")
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn variants(&self) -> &BTreeSet<String> {
        &self.variants
    }

    #[must_use]
    pub fn id(&self) -> PayloadTypeId {
        self.id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PayloadPredicateError {
    InvalidValue(String),
    DuplicateValue(String),
    ValueOutsideType {
        payload_type: PayloadTypeId,
        value: String,
    },
}

impl fmt::Display for PayloadPredicateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue(value) => write!(f, "invalid finite payload value {value:?}"),
            Self::DuplicateValue(value) => {
                write!(f, "finite payload predicate repeats {value:?}")
            }
            Self::ValueOutsideType {
                payload_type,
                value,
            } => write!(f, "payload value {value:?} is not in {payload_type}"),
        }
    }
}

impl std::error::Error for PayloadPredicateError {}

/// A canonical finite predicate represented by the accepted subset of a type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PayloadPredicate {
    payload_type: PayloadTypeId,
    accepted: BTreeSet<String>,
    id: PayloadPredicateId,
}

impl PayloadPredicate {
    pub fn new<I, V>(payload_type: &PayloadType, accepted: I) -> Result<Self, PayloadPredicateError>
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        let mut canonical_values = BTreeSet::new();
        for value in accepted {
            let value = value.into();
            if !canonical_atom(&value) {
                return Err(PayloadPredicateError::InvalidValue(value));
            }
            if !payload_type.variants().contains(&value) {
                return Err(PayloadPredicateError::ValueOutsideType {
                    payload_type: payload_type.id(),
                    value,
                });
            }
            if !canonical_values.insert(value.clone()) {
                return Err(PayloadPredicateError::DuplicateValue(value));
            }
        }
        let id = payload_predicate_id(payload_type.id(), &canonical_values);
        Ok(Self {
            payload_type: payload_type.id(),
            accepted: canonical_values,
            id,
        })
    }

    #[must_use]
    pub fn all(payload_type: &PayloadType) -> Self {
        Self::new(payload_type, payload_type.variants().iter().cloned())
            .expect("a payload type contains only canonical variants")
    }

    #[must_use]
    pub fn payload_type(&self) -> PayloadTypeId {
        self.payload_type
    }

    #[must_use]
    pub fn accepted(&self) -> &BTreeSet<String> {
        &self.accepted
    }

    #[must_use]
    pub fn id(&self) -> PayloadPredicateId {
        self.id
    }

    #[must_use]
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.payload_type == other.payload_type && self.accepted.is_subset(&other.accepted)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FiniteContractError {
    InvalidAction(String),
    DuplicateAssumption(String),
    DuplicateGuarantee(String),
}

impl fmt::Display for FiniteContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAction(action) => write!(f, "invalid contract action {action:?}"),
            Self::DuplicateAssumption(action) => {
                write!(f, "assumption for action {action:?} is duplicated")
            }
            Self::DuplicateGuarantee(action) => {
                write!(f, "guarantee for action {action:?} is duplicated")
            }
        }
    }
}

impl std::error::Error for FiniteContractError {}

/// Total input assumptions and output guarantees are validated by `OpenSystem`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FiniteContract {
    assumptions: BTreeMap<String, PayloadPredicate>,
    guarantees: BTreeMap<String, PayloadPredicate>,
}

impl FiniteContract {
    pub fn new<A, G, AS, GS>(assumptions: A, guarantees: G) -> Result<Self, FiniteContractError>
    where
        A: IntoIterator<Item = (AS, PayloadPredicate)>,
        G: IntoIterator<Item = (GS, PayloadPredicate)>,
        AS: Into<String>,
        GS: Into<String>,
    {
        let assumptions = collect_clauses(assumptions, true)?;
        let guarantees = collect_clauses(guarantees, false)?;
        Ok(Self {
            assumptions,
            guarantees,
        })
    }

    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn assumptions(&self) -> &BTreeMap<String, PayloadPredicate> {
        &self.assumptions
    }

    #[must_use]
    pub fn guarantees(&self) -> &BTreeMap<String, PayloadPredicate> {
        &self.guarantees
    }
}

fn collect_clauses<I, S>(
    clauses: I,
    assumption: bool,
) -> Result<BTreeMap<String, PayloadPredicate>, FiniteContractError>
where
    I: IntoIterator<Item = (S, PayloadPredicate)>,
    S: Into<String>,
{
    let mut result = BTreeMap::new();
    for (action, predicate) in clauses {
        let action = action.into();
        if action.is_empty() {
            return Err(FiniteContractError::InvalidAction(action));
        }
        if result.insert(action.clone(), predicate).is_some() {
            return Err(if assumption {
                FiniteContractError::DuplicateAssumption(action)
            } else {
                FiniteContractError::DuplicateGuarantee(action)
            });
        }
    }
    Ok(result)
}

fn canonical_atom(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        && value.len() <= 255
}

fn payload_type_id(name: &str, variants: &BTreeSet<String>) -> PayloadTypeId {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(PAYLOAD_TYPE_DOMAIN);
    encode_text(&mut bytes, name);
    bytes.extend_from_slice(&(variants.len() as u64).to_be_bytes());
    for variant in variants {
        encode_text(&mut bytes, variant);
    }
    PayloadTypeId(sha256(&bytes))
}

fn payload_predicate_id(
    payload_type: PayloadTypeId,
    values: &BTreeSet<String>,
) -> PayloadPredicateId {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(PAYLOAD_PREDICATE_DOMAIN);
    bytes.extend_from_slice(&payload_type.digest());
    bytes.extend_from_slice(&(values.len() as u64).to_be_bytes());
    for value in values {
        encode_text(&mut bytes, value);
    }
    PayloadPredicateId(sha256(&bytes))
}

fn encode_text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    result
}

fn sha256(input: &[u8]) -> [u8; 32] {
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
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut padded = input.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    let mut hash = INITIAL;
    for chunk in padded.chunks_exact(64) {
        let mut words = [0_u32; 64];
        for (index, bytes) in chunk.chunks_exact(4).enumerate() {
            words[index] = u32::from_be_bytes(bytes.try_into().expect("four-byte word"));
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
        let mut work = hash;
        for index in 0..64 {
            let big1 =
                work[4].rotate_right(6) ^ work[4].rotate_right(11) ^ work[4].rotate_right(25);
            let choose = (work[4] & work[5]) ^ (!work[4] & work[6]);
            let temp1 = work[7]
                .wrapping_add(big1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let big0 =
                work[0].rotate_right(2) ^ work[0].rotate_right(13) ^ work[0].rotate_right(22);
            let majority = (work[0] & work[1]) ^ (work[0] & work[2]) ^ (work[1] & work[2]);
            let temp2 = big0.wrapping_add(majority);
            work = [
                temp1.wrapping_add(temp2),
                work[0],
                work[1],
                work[2],
                work[3].wrapping_add(temp1),
                work[4],
                work[5],
                work[6],
            ];
        }
        for index in 0..8 {
            hash[index] = hash[index].wrapping_add(work[index]);
        }
    }
    let mut output = [0_u8; 32];
    for (index, word) in hash.into_iter().enumerate() {
        output[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_identity_is_order_independent_but_nominal() {
        let first = PayloadType::enumeration("Message", ["ok", "error"]).unwrap();
        let reordered = PayloadType::enumeration("Message", ["error", "ok"]).unwrap();
        let renamed = PayloadType::enumeration("OtherMessage", ["error", "ok"]).unwrap();
        assert_eq!(first.id(), reordered.id());
        assert_ne!(first.id(), renamed.id());
        assert!(
            first
                .id()
                .to_string()
                .starts_with("nmlt-payload-type-v1:sha256:")
        );
    }

    #[test]
    fn predicate_rejects_out_of_type_and_duplicate_values() {
        let ty = PayloadType::enumeration("Message", ["ok", "error"]).unwrap();
        assert!(matches!(
            PayloadPredicate::new(&ty, ["missing"]),
            Err(PayloadPredicateError::ValueOutsideType { .. })
        ));
        assert_eq!(
            PayloadPredicate::new(&ty, ["ok", "ok"]),
            Err(PayloadPredicateError::DuplicateValue("ok".to_owned()))
        );
    }

    #[test]
    fn predicate_inclusion_requires_exact_payload_identity() {
        let ty = PayloadType::enumeration("Message", ["ok", "error"]).unwrap();
        let renamed = PayloadType::enumeration("OtherMessage", ["ok", "error"]).unwrap();
        let narrow = PayloadPredicate::new(&ty, ["ok"]).unwrap();
        let wide = PayloadPredicate::all(&ty);
        let substituted = PayloadPredicate::all(&renamed);
        assert!(narrow.is_subset_of(&wide));
        assert!(!wide.is_subset_of(&narrow));
        assert!(!narrow.is_subset_of(&substituted));
    }
}
