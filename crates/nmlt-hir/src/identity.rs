//! Versioned content identities used by the resolver boundary.

use std::collections::BTreeSet;
use std::fmt;

const SOURCE_DOMAIN: &[u8] = b"NMLT-SOURCE\0v1\0";
const SOURCE_SET_DOMAIN: &[u8] = b"NMLT-SOURCE-SET\0v1\0";
const MODULE_MAP_DOMAIN: &[u8] = b"NMLT-MODULE-MAP\0v1\0";
const MODULE_DOMAIN: &[u8] = b"NMLT-MODULE\0v1\0";
const DEFINITION_DOMAIN: &[u8] = b"NMLT-DEF\0v1\0";
const NODE_DOMAIN: &[u8] = b"NMLT-NODE\0v1\0";
const RESOLUTION_DOMAIN: &[u8] = b"NMLT-HIR-RESOLUTION\0v1\0";

macro_rules! identity_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            /// The textual identity prefix, including the hash algorithm.
            pub const PREFIX: &'static str = $prefix;

            /// Returns the raw SHA-256 digest.
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
                write_hex(formatter, &self.0)
            }
        }
    };
}

identity_type!(SourceId, "nmlt-source-v1:sha256:");
identity_type!(SourceSetId, "nmlt-source-set-v1:sha256:");
identity_type!(ModuleMapId, "nmlt-module-map-v1:sha256:");
identity_type!(ModuleId, "nmlt-module-v1:sha256:");
identity_type!(DefId, "nmlt-def-v1:sha256:");
identity_type!(NodeId, "nmlt-node-v1:sha256:");
identity_type!(ResolutionId, "nmlt-hir-resolution-v1:sha256:");

/// A path-and-bytes pair used to compute an RFC 0004 source-set identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSetEntry<'a> {
    /// Portable repository-relative path. The caller must validate path policy.
    pub repository_path: &'a str,
    /// Exact, unnormalized source bytes.
    pub exact_bytes: &'a [u8],
}

/// Failure to construct a canonical source-set identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceSetIdentityError {
    /// Two entries used the same portable repository path.
    DuplicatePath(String),
}

impl fmt::Display for SourceSetIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePath(path) => {
                write!(formatter, "duplicate source-set path `{path}`")
            }
        }
    }
}

impl std::error::Error for SourceSetIdentityError {}

impl SourceId {
    /// Computes the RFC 0004 identity of exact source bytes without normalizing them.
    #[must_use]
    pub fn from_bytes(exact_bytes: &[u8]) -> Self {
        let mut preimage = Encoder::with_domain(SOURCE_DOMAIN);
        preimage.bytes(exact_bytes);
        Self(sha256(&preimage.finish()))
    }
}

impl SourceSetId {
    /// Computes the RFC 0004 identity of a canonical source set.
    ///
    /// Entries are sorted by UTF-8 path bytes. Duplicate paths are rejected;
    /// portable-path and symlink policy remain the resolver adapter's duty.
    pub fn from_entries(entries: &[SourceSetEntry<'_>]) -> Result<Self, SourceSetIdentityError> {
        let mut canonical = entries.to_vec();
        canonical.sort_by(|left, right| {
            left.repository_path
                .as_bytes()
                .cmp(right.repository_path.as_bytes())
        });

        let mut seen = BTreeSet::new();
        for entry in &canonical {
            if !seen.insert(entry.repository_path) {
                return Err(SourceSetIdentityError::DuplicatePath(
                    entry.repository_path.to_owned(),
                ));
            }
        }

        let mut preimage = Encoder::with_domain(SOURCE_SET_DOMAIN);
        preimage.count(canonical.len());
        for entry in canonical {
            preimage.text(entry.repository_path);
            preimage.raw(SourceId::from_bytes(entry.exact_bytes).digest());
        }
        Ok(Self(sha256(&preimage.finish())))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ModuleMapEntry<'a> {
    pub logical_module: &'a str,
    pub repository_path: &'a str,
}

pub(crate) fn module_map_id(
    source_set_id: SourceSetId,
    entries: &[ModuleMapEntry<'_>],
) -> ModuleMapId {
    let mut canonical = entries.to_vec();
    canonical.sort_by(|left, right| {
        left.logical_module
            .as_bytes()
            .cmp(right.logical_module.as_bytes())
    });
    let mut preimage = Encoder::with_domain(MODULE_MAP_DOMAIN);
    preimage.raw(source_set_id.digest());
    preimage.count(canonical.len());
    for entry in canonical {
        preimage.text(entry.logical_module);
        preimage.text(entry.repository_path);
    }
    ModuleMapId(sha256(&preimage.finish()))
}

pub(crate) fn module_id(module_map_id: ModuleMapId, logical_module: &str) -> ModuleId {
    let mut preimage = Encoder::with_domain(MODULE_DOMAIN);
    preimage.raw(module_map_id.digest());
    preimage.text(logical_module);
    ModuleId(sha256(&preimage.finish()))
}

pub(crate) fn definition_id<I, S>(module_id: ModuleId, path: I) -> DefId
where
    I: ExactSizeIterator<Item = (u8, S)>,
    S: AsRef<str>,
{
    let mut encoded_path = Encoder::empty();
    encoded_path.count(path.len());
    for (kind, name) in path {
        encoded_path.raw(&[kind]);
        encoded_path.text(name.as_ref());
    }
    let mut preimage = Encoder::with_domain(DEFINITION_DOMAIN);
    preimage.raw(module_id.digest());
    preimage.bytes(&encoded_path.finish());
    DefId(sha256(&preimage.finish()))
}

pub(crate) fn node_id(definition_id: DefId, semantic_roles: &[u8]) -> NodeId {
    let mut encoded_path = Encoder::empty();
    encoded_path.count(semantic_roles.len());
    for role in semantic_roles {
        encoded_path.raw(&[*role]);
    }
    let mut preimage = Encoder::with_domain(NODE_DOMAIN);
    preimage.raw(definition_id.digest());
    preimage.bytes(&encoded_path.finish());
    NodeId(sha256(&preimage.finish()))
}

pub(crate) struct ResolutionIdentityModule<'a> {
    pub logical_module: &'a str,
    pub repository_path: &'a str,
    pub source_id: SourceId,
    pub imports: Vec<&'a str>,
    pub declarations: Vec<ResolutionIdentityDeclaration<'a>>,
}

pub(crate) struct ResolutionIdentityDeclaration<'a> {
    pub path: Vec<(u8, &'a str)>,
}

pub(crate) fn resolution_id(
    source_set_id: SourceSetId,
    module_map_id: ModuleMapId,
    modules: &[ResolutionIdentityModule<'_>],
) -> ResolutionId {
    let mut preimage = Encoder::with_domain(RESOLUTION_DOMAIN);
    preimage.raw(source_set_id.digest());
    preimage.raw(module_map_id.digest());
    preimage.count(modules.len());
    for module in modules {
        preimage.text(module.logical_module);
        preimage.text(module.repository_path);
        preimage.raw(module.source_id.digest());
        preimage.count(module.imports.len());
        for import in &module.imports {
            preimage.text(import);
        }
        preimage.count(module.declarations.len());
        for declaration in &module.declarations {
            preimage.count(declaration.path.len());
            for (kind, name) in &declaration.path {
                preimage.raw(&[*kind]);
                preimage.text(name);
            }
        }
    }
    ResolutionId(sha256(&preimage.finish()))
}

struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    fn empty() -> Self {
        Self { bytes: Vec::new() }
    }

    fn with_domain(domain: &[u8]) -> Self {
        Self {
            bytes: domain.to_vec(),
        }
    }

    fn raw(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn u64(&mut self, value: u64) {
        self.raw(&value.to_be_bytes());
    }

    fn count(&mut self, count: usize) {
        self.u64(count as u64);
    }

    fn bytes(&mut self, bytes: &[u8]) {
        self.count(bytes.len());
        self.raw(bytes);
    }

    fn text(&mut self, text: &str) {
        self.bytes(text.as_bytes());
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

fn write_hex(formatter: &mut fmt::Formatter<'_>, digest: &[u8; 32]) -> fmt::Result {
    for byte in digest {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
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

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::{
        ModuleMapEntry, SourceId, SourceSetId, definition_id, module_id, module_map_id, node_id,
        sha256,
    };

    fn hex(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            let _ = write!(output, "{byte:02x}");
        }
        output
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn source_id_uses_the_normative_domain_and_length_prefix() {
        assert_eq!(
            SourceId::from_bytes(b"").to_string(),
            "nmlt-source-v1:sha256:\
             2219d5183cf9c81e12b457263a376f865a55cdd3184b341f6f7d4054a2236cf4"
        );
    }

    #[test]
    fn single_identifier_m9_identity_golden_vector() {
        let source_set_id = SourceSetId(std::array::from_fn(|index| index as u8));
        let module_map_id = module_map_id(
            source_set_id,
            &[ModuleMapEntry {
                logical_module: "Demo",
                repository_path: "examples/toggle.nmlt",
            }],
        );
        assert_eq!(
            module_map_id.to_string(),
            "nmlt-module-map-v1:sha256:\
             4e98b7b4f0fa022e4630a6c9541ac7fd7e9c3cd4fd992d01eb62b125e83b7839"
        );

        let module_id = module_id(module_map_id, "Demo");
        assert_eq!(
            module_id.to_string(),
            "nmlt-module-v1:sha256:\
             c4ba45c8114c82d9daf57884c9a1c3c8fa75d3dee97689bb7cea52a1d258fc6c"
        );

        let system_id = definition_id(module_id, [(0x04, "Toggle")].into_iter());
        assert_eq!(
            system_id.to_string(),
            "nmlt-def-v1:sha256:\
             687edb3577543b9836cbf68bdf582dd25b566e73671b604e9f249c9a44715b55"
        );

        let state_id = definition_id(module_id, [(0x04, "Toggle"), (0x05, "on")].into_iter());
        assert_eq!(
            state_id.to_string(),
            "nmlt-def-v1:sha256:\
             bbaa1fdb91efc6617229d4efa1d0451fa4d06802a7e6a33943dea931d1a52aa4"
        );

        assert_eq!(
            node_id(state_id, &[0x03]).to_string(),
            "nmlt-node-v1:sha256:\
             217a2e50a04141495c25eb47000e3ad8fc3ea00b8c75672bd10083226e3863ba"
        );
    }

    #[test]
    fn module_map_identity_is_input_order_independent() {
        let source_set_id = SourceSetId([7; 32]);
        let left = ModuleMapEntry {
            logical_module: "A",
            repository_path: "a.nmlt",
        };
        let right = ModuleMapEntry {
            logical_module: "B",
            repository_path: "b.nmlt",
        };
        assert_eq!(
            module_map_id(source_set_id, &[left, right]),
            module_map_id(source_set_id, &[right, left])
        );
    }
}
