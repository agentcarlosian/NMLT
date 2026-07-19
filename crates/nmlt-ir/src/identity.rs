//! Canonical identities for the explicit typed-core boundary.

use std::fmt;

use nmlt_hir::{DefId, NodeId, sha256_bytes};

use crate::model::{
    CoreBinaryOp, CoreProgram, CorePropertyKind, CoreTermKind, CoreType, CoreUnaryOp,
};
use crate::validate::{CoreResourceDimension, CoreValidationError, MAX_CANONICAL_BYTES};

const CORE_NODE_DOMAIN: &[u8] = b"NMLT-CORE-NODE\0v1\0";
const CORE_PROGRAM_DOMAIN: &[u8] = b"NMLT-CORE-PROGRAM\0v1\0";
const MAX_INSERTION_DEPTH: usize = 32;

macro_rules! identity_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            pub const PREFIX: &'static str = $prefix;

            #[must_use]
            pub const fn digest(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, formatter)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(Self::PREFIX)?;
                for byte in self.0 {
                    write!(formatter, "{byte:02x}")?;
                }
                Ok(())
            }
        }
    };
}

identity_type!(CoreNodeId, "nmlt-core-node-v1:sha256:");
identity_type!(CoreProgramId, "nmlt-core-program-v1:sha256:");

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreIdentityError {
    InsertionPathTooDeep { actual: usize, maximum: usize },
}

impl fmt::Display for CoreIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsertionPathTooDeep { actual, maximum } => write!(
                formatter,
                "core insertion path depth {actual} exceeds maximum {maximum}"
            ),
        }
    }
}

impl std::error::Error for CoreIdentityError {}

impl CoreNodeId {
    /// Derive a core-node identity from its source HIR origin and a canonical
    /// elaborator-controlled insertion path. An empty path is the direct
    /// translation; nonempty paths identify type-directed wrapper nodes.
    pub fn from_origin(origin: NodeId, insertion_path: &[u32]) -> Result<Self, CoreIdentityError> {
        if insertion_path.len() > MAX_INSERTION_DEPTH {
            return Err(CoreIdentityError::InsertionPathTooDeep {
                actual: insertion_path.len(),
                maximum: MAX_INSERTION_DEPTH,
            });
        }
        let mut encoded = Encoder::with_domain(CORE_NODE_DOMAIN);
        encoded.raw(origin.digest());
        encoded.count(insertion_path.len());
        for segment in insertion_path {
            encoded.u32(*segment);
        }
        Ok(Self(sha256_bytes(&encoded.finish())))
    }
}

impl CoreProgramId {
    pub(crate) const fn zero() -> Self {
        Self([0; 32])
    }
}

pub(crate) fn core_program_id(program: &CoreProgram) -> Result<CoreProgramId, CoreValidationError> {
    let mut encoded = Encoder::with_domain_limit(CORE_PROGRAM_DOMAIN, MAX_CANONICAL_BYTES);
    encoded.raw(program.resolved_hir_id.digest());
    encoded.count(program.modules.len());
    for (module_id, module) in &program.modules {
        encoded.raw(module_id.digest());
        encoded.count(module.imports.len());
        for import in &module.imports {
            encoded.raw(import.digest());
        }
        encoded.count(module.enumerations.len());
        for (enum_id, enumeration) in &module.enumerations {
            encoded.raw(enum_id.digest());
            encoded.count(enumeration.constructors.len());
            for constructor in &enumeration.constructors {
                encoded.raw(constructor.digest());
            }
        }
        encoded.count(module.systems.len());
        for (system_id, system) in &module.systems {
            encoded.raw(system_id.digest());
            encoded.count(system.state.len());
            for (state_id, state) in &system.state {
                encoded.raw(state_id.digest());
                encode_type(&mut encoded, &state.ty);
                encoded.raw(state.initializer.digest());
            }
            encoded.count(system.capabilities.len());
            for (capability_id, capability) in &system.capabilities {
                encoded.raw(capability_id.digest());
                encoded.raw(capability.protocol.digest());
            }
            encoded.count(system.actions.len());
            for (action_id, action) in &system.actions {
                encoded.raw(action_id.digest());
                encoded.raw(action.system.digest());
                encoded.count(action.parameters.len());
                for (local_id, parameter) in &action.parameters {
                    encoded.raw(local_id.digest());
                    encode_type(&mut encoded, &parameter.ty);
                }
                encode_core_ids(&mut encoded, &action.guards);
                encoded.count(action.updates.len());
                for (state_id, value) in &action.updates {
                    encoded.raw(state_id.digest());
                    encoded.raw(value.digest());
                }
                encode_def_ids(&mut encoded, &action.frames);
                encode_core_ids(&mut encoded, &action.outputs);
                encode_def_ids(&mut encoded, &action.consumes);
            }
            encoded.count(system.properties.len());
            for (property_id, property) in &system.properties {
                encoded.raw(property_id.digest());
                encoded.raw(property.system.digest());
                encoded.u8(match property.kind {
                    CorePropertyKind::Safety => 1,
                    CorePropertyKind::Temporal => 2,
                });
                encoded.raw(property.body.digest());
            }
            encoded.count(system.observations.len());
            for observation in &system.observations {
                encoded.raw(observation.owner.digest());
                encoded.raw(observation.origin.digest());
                encode_core_ids(&mut encoded, &observation.items);
            }
        }
    }
    encoded.count(program.terms.len());
    for (term_id, term) in &program.terms {
        encoded.raw(term_id.digest());
        encoded.raw(term.origin.digest());
        encoded.raw(term.owner.digest());
        encode_type(&mut encoded, &term.ty);
        encode_term(&mut encoded, &term.kind);
    }
    if encoded.overflowed() {
        return Err(CoreValidationError::ResourceExceeded {
            dimension: CoreResourceDimension::CanonicalBytes,
            actual: encoded.encoded_len(),
            maximum: MAX_CANONICAL_BYTES,
        });
    }
    let encoded = encoded.finish();
    Ok(CoreProgramId(sha256_bytes(&encoded)))
}

fn encode_type(encoded: &mut Encoder, ty: &CoreType) {
    match ty {
        CoreType::Bool => encoded.u8(1),
        CoreType::Nat => encoded.u8(2),
        CoreType::Int => encoded.u8(3),
        CoreType::Enum(id) => {
            encoded.u8(4);
            encoded.raw(id.digest());
        }
        CoreType::Once { protocol } => {
            encoded.u8(5);
            encoded.raw(protocol.digest());
        }
        CoreType::StateProp { system } => {
            encoded.u8(6);
            encoded.raw(system.digest());
        }
        CoreType::TemporalProp { system } => {
            encoded.u8(7);
            encoded.raw(system.digest());
        }
    }
}

fn encode_term(encoded: &mut Encoder, term: &CoreTermKind) {
    match term {
        CoreTermKind::Bool(value) => {
            encoded.u8(1);
            encoded.u8(u8::from(*value));
        }
        CoreTermKind::Nat { magnitude } => {
            encoded.u8(2);
            encoded.bytes(magnitude);
        }
        CoreTermKind::Int {
            negative,
            magnitude,
        } => {
            encoded.u8(3);
            encoded.u8(u8::from(*negative));
            encoded.bytes(magnitude);
        }
        CoreTermKind::Local(id) => {
            encoded.u8(4);
            encoded.raw(id.digest());
        }
        CoreTermKind::State { system, state } => {
            encoded.u8(5);
            encoded.raw(system.digest());
            encoded.raw(state.digest());
        }
        CoreTermKind::Constructor {
            enumeration,
            constructor,
        } => {
            encoded.u8(6);
            encoded.raw(enumeration.digest());
            encoded.raw(constructor.digest());
        }
        CoreTermKind::Unary { operator, operand } => {
            encoded.u8(7);
            encoded.u8(match operator {
                CoreUnaryOp::Not => 1,
                CoreUnaryOp::Negate => 2,
            });
            encoded.raw(operand.digest());
        }
        CoreTermKind::Binary {
            operator,
            left,
            right,
        } => {
            encoded.u8(8);
            encoded.u8(match operator {
                CoreBinaryOp::Or => 1,
                CoreBinaryOp::And => 2,
                CoreBinaryOp::Implies => 3,
                CoreBinaryOp::Equal => 4,
                CoreBinaryOp::NotEqual => 5,
                CoreBinaryOp::Less => 6,
                CoreBinaryOp::LessEqual => 7,
                CoreBinaryOp::Greater => 8,
                CoreBinaryOp::GreaterEqual => 9,
                CoreBinaryOp::Add => 10,
                CoreBinaryOp::Subtract => 11,
                CoreBinaryOp::Multiply => 12,
            });
            encoded.raw(left.digest());
            encoded.raw(right.digest());
        }
        CoreTermKind::IntFromNat { operand } => {
            encoded.u8(9);
            encoded.raw(operand.digest());
        }
        CoreTermKind::StatePredicate { system, condition } => {
            encoded.u8(10);
            encoded.raw(system.digest());
            encoded.raw(condition.digest());
        }
        CoreTermKind::Always { system, property } => {
            encoded.u8(11);
            encoded.raw(system.digest());
            encoded.raw(property.digest());
        }
        CoreTermKind::Eventually { system, property } => {
            encoded.u8(12);
            encoded.raw(system.digest());
            encoded.raw(property.digest());
        }
        CoreTermKind::Next { system, property } => {
            encoded.u8(13);
            encoded.raw(system.digest());
            encoded.raw(property.digest());
        }
        CoreTermKind::Until {
            system,
            left,
            right,
        } => {
            encoded.u8(14);
            encoded.raw(system.digest());
            encoded.raw(left.digest());
            encoded.raw(right.digest());
        }
        CoreTermKind::Enabled { system, action } => {
            encoded.u8(15);
            encoded.raw(system.digest());
            encoded.raw(action.digest());
        }
        CoreTermKind::ActionOccurred { system, action } => {
            encoded.u8(16);
            encoded.raw(system.digest());
            encoded.raw(action.digest());
        }
    }
}

fn encode_core_ids(encoded: &mut Encoder, ids: &[CoreNodeId]) {
    encoded.count(ids.len());
    for id in ids {
        encoded.raw(id.digest());
    }
}

fn encode_def_ids<'a>(encoded: &mut Encoder, ids: impl IntoIterator<Item = &'a DefId>) {
    let ids: Vec<_> = ids.into_iter().collect();
    encoded.count(ids.len());
    for id in ids {
        encoded.raw(id.digest());
    }
}

struct Encoder {
    bytes: Vec<u8>,
    encoded_len: usize,
    limit: Option<usize>,
    overflowed: bool,
}

impl Encoder {
    fn with_domain(domain: &[u8]) -> Self {
        Self {
            bytes: domain.to_vec(),
            encoded_len: domain.len(),
            limit: None,
            overflowed: false,
        }
    }
    fn with_domain_limit(domain: &[u8], limit: usize) -> Self {
        Self {
            bytes: domain.to_vec(),
            encoded_len: domain.len(),
            limit: Some(limit),
            overflowed: domain.len() > limit,
        }
    }
    fn raw(&mut self, bytes: &[u8]) {
        self.encoded_len = self.encoded_len.saturating_add(bytes.len());
        if self.limit.is_some_and(|limit| self.encoded_len > limit) {
            self.overflowed = true;
            return;
        }
        self.bytes.extend_from_slice(bytes);
    }
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }
    fn u32(&mut self, value: u32) {
        self.raw(&value.to_be_bytes());
    }
    fn count(&mut self, count: usize) {
        self.raw(&(count as u64).to_be_bytes());
    }
    fn bytes(&mut self, bytes: &[u8]) {
        self.count(bytes.len());
        self.raw(bytes);
    }
    fn finish(self) -> Vec<u8> {
        self.bytes
    }
    const fn encoded_len(&self) -> usize {
        self.encoded_len
    }
    const fn overflowed(&self) -> bool {
        self.overflowed
    }
}
