use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::artifact::ArtifactRole;
use crate::digest::sha256_hex;
use crate::feedback::ResultClass;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ByteSpan {
    pub start: usize,
    pub end: usize,
}

impl ByteSpan {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    #[must_use]
    const fn valid_for(self, source_len: usize) -> bool {
        self.start <= self.end && self.end <= source_len
    }

    #[must_use]
    const fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    #[must_use]
    const fn contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateFile {
    pub path: String,
    pub source: String,
    pub digest: String,
}

impl CandidateFile {
    #[must_use]
    pub fn new(path: impl Into<String>, source: impl Into<String>) -> Self {
        let path = path.into();
        let source = source.into();
        let digest = format!("sha256:{}", sha256_hex(source.as_bytes()));
        Self {
            path,
            source,
            digest,
        }
    }

    #[must_use]
    pub fn identity_is_valid(&self) -> bool {
        self.digest == format!("sha256:{}", sha256_hex(self.source.as_bytes()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtectedSpan {
    pub path: String,
    pub span: ByteSpan,
    pub digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditPolicy {
    pub editable_spans: BTreeMap<String, Vec<ByteSpan>>,
    pub protected_paths: BTreeMap<String, String>,
    pub protected_spans: Vec<ProtectedSpan>,
    pub insertion_boundaries: BTreeSet<(String, usize)>,
    pub max_edited_bytes: usize,
    pub max_edits: usize,
}

impl EditPolicy {
    #[must_use]
    pub fn localized(max_edits: usize, max_edited_bytes: usize) -> Self {
        Self {
            editable_spans: BTreeMap::new(),
            protected_paths: BTreeMap::new(),
            protected_spans: Vec::new(),
            insertion_boundaries: BTreeSet::new(),
            max_edited_bytes,
            max_edits,
        }
    }

    pub fn allow_span(&mut self, path: impl Into<String>, span: ByteSpan) {
        self.editable_spans
            .entry(path.into())
            .or_default()
            .push(span);
    }

    pub fn protect_path(&mut self, path: impl Into<String>, bytes: &[u8]) {
        self.protected_paths
            .insert(path.into(), format!("sha256:{}", sha256_hex(bytes)));
    }

    pub fn protect_span(
        &mut self,
        path: impl Into<String>,
        span: ByteSpan,
        source: &str,
    ) -> Result<(), AuthorityError> {
        if !span.valid_for(source.len())
            || !source.is_char_boundary(span.start)
            || !source.is_char_boundary(span.end)
        {
            return Err(AuthorityError::InvalidSpan);
        }
        let path = path.into();
        self.protected_spans.push(ProtectedSpan {
            path,
            span,
            digest: format!(
                "sha256:{}",
                sha256_hex(&source.as_bytes()[span.start..span.end])
            ),
        });
        Ok(())
    }

    pub fn allow_insertion_boundary(&mut self, path: impl Into<String>, offset: usize) {
        self.insertion_boundaries.insert((path.into(), offset));
    }

    pub fn verify_protected_paths<'a>(
        &self,
        actual: impl IntoIterator<Item = (&'a str, &'a [u8])>,
    ) -> Result<(), AuthorityError> {
        let actual = actual
            .into_iter()
            .map(|(path, bytes)| (path, format!("sha256:{}", sha256_hex(bytes))))
            .collect::<BTreeMap<_, _>>();
        for (path, digest) in &self.protected_paths {
            if actual.get(path.as_str()) != Some(digest) {
                return Err(AuthorityError::ProtectedDigestChanged(path.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Edit {
    pub role: ArtifactRole,
    pub path: String,
    pub span: ByteSpan,
    pub replacement: String,
}

impl Edit {
    #[must_use]
    pub fn candidate(
        path: impl Into<String>,
        span: ByteSpan,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            role: ArtifactRole::Candidate,
            path: path.into(),
            span,
            replacement: replacement.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub proposal_id: String,
    pub edits: Vec<Edit>,
    pub rationale: String,
    /// The wire format forbids this field. It exists so the gate can reject an
    /// untrusted assistant that attempts to forge checker authority.
    pub claimed_result: Option<ResultClass>,
}

impl Proposal {
    #[must_use]
    pub fn localized(
        proposal_id: impl Into<String>,
        edits: Vec<Edit>,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            edits,
            rationale: rationale.into(),
            claimed_result: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthorityError {
    EmptyProposal,
    ForgedResult,
    InvalidPath(String),
    NonCandidateRole(ArtifactRole),
    PathNotAllowlisted(String),
    CandidateMissing(String),
    CandidateDigestChanged(String),
    InvalidSpan,
    EditOutsideAllowlist(String),
    ProtectedSpan(String),
    ProtectedDigestChanged(String),
    ProtectedBoundary(String),
    WholeFileReplacement(String),
    TooManyEdits,
    EditBudgetExceeded,
    OverlappingEdits(String),
    NonUtf8Boundary(String),
}

impl fmt::Display for AuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AuthorityError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedCandidates {
    pub files: BTreeMap<String, CandidateFile>,
    pub edited_bytes: usize,
}

fn safe_relative_path(path: &str) -> bool {
    if path.is_empty()
        || path.starts_with('/')
        || path.starts_with('~')
        || path.contains('\\')
        || path.contains('\0')
        || path.contains("//")
        || path.contains(':')
    {
        return false;
    }
    path.split('/')
        .all(|component| !component.is_empty() && component != "." && component != "..")
}

pub fn validate_proposal(
    candidates: &BTreeMap<String, CandidateFile>,
    policy: &EditPolicy,
    proposal: &Proposal,
) -> Result<usize, AuthorityError> {
    if proposal.edits.is_empty() {
        return Err(AuthorityError::EmptyProposal);
    }
    if proposal.claimed_result.is_some() {
        return Err(AuthorityError::ForgedResult);
    }
    if proposal.edits.len() > policy.max_edits {
        return Err(AuthorityError::TooManyEdits);
    }

    for candidate in candidates.values() {
        if !candidate.identity_is_valid() {
            return Err(AuthorityError::CandidateDigestChanged(
                candidate.path.clone(),
            ));
        }
    }
    for protected in &policy.protected_spans {
        let candidate = candidates
            .get(&protected.path)
            .ok_or_else(|| AuthorityError::CandidateMissing(protected.path.clone()))?;
        if !protected.span.valid_for(candidate.source.len()) {
            return Err(AuthorityError::InvalidSpan);
        }
        let digest = format!(
            "sha256:{}",
            sha256_hex(&candidate.source.as_bytes()[protected.span.start..protected.span.end])
        );
        if digest != protected.digest {
            return Err(AuthorityError::ProtectedDigestChanged(
                protected.path.clone(),
            ));
        }
    }

    let mut extent = 0_usize;
    let mut by_path: BTreeMap<&str, Vec<ByteSpan>> = BTreeMap::new();
    for edit in &proposal.edits {
        if edit.role != ArtifactRole::Candidate {
            return Err(AuthorityError::NonCandidateRole(edit.role));
        }
        if !safe_relative_path(&edit.path) {
            return Err(AuthorityError::InvalidPath(edit.path.clone()));
        }
        let allowed = policy
            .editable_spans
            .get(&edit.path)
            .ok_or_else(|| AuthorityError::PathNotAllowlisted(edit.path.clone()))?;
        let candidate = candidates
            .get(&edit.path)
            .ok_or_else(|| AuthorityError::CandidateMissing(edit.path.clone()))?;
        if !edit.span.valid_for(candidate.source.len()) {
            return Err(AuthorityError::InvalidSpan);
        }
        if !candidate.source.is_char_boundary(edit.span.start)
            || !candidate.source.is_char_boundary(edit.span.end)
        {
            return Err(AuthorityError::NonUtf8Boundary(edit.path.clone()));
        }
        if !allowed.iter().any(|span| span.contains(edit.span)) {
            return Err(AuthorityError::EditOutsideAllowlist(edit.path.clone()));
        }
        if edit.span.start == 0 && edit.span.end == candidate.source.len() {
            return Err(AuthorityError::WholeFileReplacement(edit.path.clone()));
        }
        for protected in policy
            .protected_spans
            .iter()
            .filter(|protected| protected.path == edit.path)
        {
            if edit.span.overlaps(protected.span) {
                return Err(AuthorityError::ProtectedSpan(edit.path.clone()));
            }
            if edit.span.is_empty()
                && (edit.span.start == protected.span.start
                    || edit.span.start == protected.span.end)
                && !policy
                    .insertion_boundaries
                    .contains(&(edit.path.clone(), edit.span.start))
            {
                return Err(AuthorityError::ProtectedBoundary(edit.path.clone()));
            }
        }
        extent = extent
            .saturating_add(edit.span.len())
            .saturating_add(edit.replacement.len());
        by_path.entry(&edit.path).or_default().push(edit.span);
    }
    if extent > policy.max_edited_bytes {
        return Err(AuthorityError::EditBudgetExceeded);
    }
    for (path, spans) in &mut by_path {
        spans.sort();
        for pair in spans.windows(2) {
            let insertion_conflict = (pair[0].is_empty()
                && pair[1].start <= pair[0].start
                && pair[0].start <= pair[1].end)
                || (pair[1].is_empty()
                    && pair[0].start <= pair[1].start
                    && pair[1].start <= pair[0].end);
            if pair[0].overlaps(pair[1]) || insertion_conflict {
                return Err(AuthorityError::OverlappingEdits((*path).to_owned()));
            }
        }
    }
    Ok(extent)
}

pub fn apply_proposal(
    candidates: &BTreeMap<String, CandidateFile>,
    policy: &EditPolicy,
    proposal: &Proposal,
) -> Result<AppliedCandidates, AuthorityError> {
    let edited_bytes = validate_proposal(candidates, policy, proposal)?;
    let mut files = candidates.clone();
    let mut edits = proposal.edits.iter().collect::<Vec<_>>();
    edits.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| right.span.start.cmp(&left.span.start))
            .then_with(|| right.span.end.cmp(&left.span.end))
    });
    for edit in edits {
        let candidate = files
            .get_mut(&edit.path)
            .ok_or_else(|| AuthorityError::CandidateMissing(edit.path.clone()))?;
        candidate
            .source
            .replace_range(edit.span.start..edit.span.end, &edit.replacement);
        candidate.digest = format!("sha256:{}", sha256_hex(candidate.source.as_bytes()));
    }
    Ok(AppliedCandidates {
        files,
        edited_bytes,
    })
}
