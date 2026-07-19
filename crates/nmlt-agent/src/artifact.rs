use std::collections::BTreeMap;

use crate::digest::sha256_hex;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ArtifactRole {
    Intent,
    Property,
    Oracle,
    Candidate,
    Feedback,
    Proposal,
    Evaluation,
}

impl ArtifactRole {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Intent => "intent",
            Self::Property => "property",
            Self::Oracle => "oracle",
            Self::Candidate => "candidate",
            Self::Feedback => "feedback",
            Self::Proposal => "proposal",
            Self::Evaluation => "evaluation",
        }
    }

    #[must_use]
    pub const fn is_trusted(self) -> bool {
        matches!(self, Self::Intent | Self::Property | Self::Oracle)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedArtifact {
    pub id: String,
    pub role: ArtifactRole,
    pub path: String,
    pub bytes: Vec<u8>,
    pub digest: String,
}

impl TrustedArtifact {
    #[must_use]
    pub fn freeze(
        id: impl Into<String>,
        role: ArtifactRole,
        path: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> Self {
        assert!(role.is_trusted(), "only trusted roles can be frozen");
        let bytes = bytes.into();
        let digest = format!("sha256:{}", sha256_hex(&bytes));
        Self {
            id: id.into(),
            role,
            path: path.into(),
            bytes,
            digest,
        }
    }

    #[must_use]
    pub fn identity_is_valid(&self) -> bool {
        self.digest == format!("sha256:{}", sha256_hex(&self.bytes))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ArtifactSet {
    artifacts: BTreeMap<String, TrustedArtifact>,
}

impl ArtifactSet {
    pub fn insert(&mut self, artifact: TrustedArtifact) -> Result<(), String> {
        if !artifact.identity_is_valid() {
            return Err(format!("invalid identity for {}", artifact.id));
        }
        if self.artifacts.contains_key(&artifact.id) {
            return Err("duplicate artifact id".into());
        }
        self.artifacts.insert(artifact.id.clone(), artifact);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&TrustedArtifact> {
        self.artifacts.get(id)
    }

    pub fn verify_frozen(&self, expected: &Self) -> Result<(), String> {
        if self.artifacts.len() != expected.artifacts.len() {
            return Err("trusted artifact set changed".into());
        }
        for (id, artifact) in &expected.artifacts {
            let actual = self
                .artifacts
                .get(id)
                .ok_or_else(|| format!("trusted artifact {id} was dropped"))?;
            if actual.role != artifact.role
                || actual.path != artifact.path
                || actual.digest != artifact.digest
                || !actual.identity_is_valid()
            {
                return Err(format!("trusted artifact {id} changed"));
            }
        }
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &TrustedArtifact> {
        self.artifacts.values()
    }
}
