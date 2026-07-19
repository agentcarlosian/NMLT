//! Closed-set module and namespace resolution.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::identity::{
    ModuleMapEntry, SourceSetIdentityError, definition_id, module_id, module_map_id, resolution_id,
    surface_program_id,
};
use crate::model::{
    DeclarationInput, DeclarationKey, DefPath, ModuleInput, NameReference, ProjectedModule,
    ProjectionIssue, ResolvedDeclaration, ResolvedImport, ResolvedModule, ResolvedProgram,
    SourceSpan,
};
use crate::resolve_terms::{PendingModuleTerms, resolve_program_terms};
use crate::term::{RawTermInputKind, TermRootInput};
use crate::{SourceId, SourceSetEntry, SourceSetId};

const MAX_MODULES: u64 = 256;
const MAX_SOURCE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TOTAL_SOURCE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_HIR_BYTES: u64 = 32 * 1024 * 1024;
const MAX_HIR_NODES: u64 = 262_144;
const MAX_DEF_PATH_DEPTH: u64 = 256;
const MAX_TERM_DEPTH: u64 = 256;
const MAX_IDENTIFIER_BYTES: u64 = 255;
const MAX_LOGICAL_OR_PATH_BYTES: u64 = 4_096;
const MAX_INTEGER_MAGNITUDE_BYTES: u64 = 4_096;
const MAX_TOTAL_INTEGER_PAYLOAD: u64 = 16 * 1024 * 1024;
const MAX_CONTEXT_ENTRIES: u64 = 65_536;

/// Version-1 resolver resource dimensions inherited from RFC 0013.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceDimension {
    Modules,
    SourceBytes,
    TotalSourceBytes,
    HirBytes,
    HirNodes,
    DefinitionPathDepth,
    TermDepth,
    IdentifierBytes,
    LogicalModuleOrPathBytes,
    IntegerMagnitudeBytes,
    TotalIntegerPayload,
    ContextEntries,
}

impl ResourceDimension {
    #[must_use]
    pub const fn maximum(self) -> u64 {
        match self {
            Self::Modules => MAX_MODULES,
            Self::SourceBytes => MAX_SOURCE_BYTES,
            Self::TotalSourceBytes => MAX_TOTAL_SOURCE_BYTES,
            Self::HirBytes => MAX_HIR_BYTES,
            Self::HirNodes => MAX_HIR_NODES,
            Self::DefinitionPathDepth => MAX_DEF_PATH_DEPTH,
            Self::TermDepth => MAX_TERM_DEPTH,
            Self::IdentifierBytes => MAX_IDENTIFIER_BYTES,
            Self::LogicalModuleOrPathBytes => MAX_LOGICAL_OR_PATH_BYTES,
            Self::IntegerMagnitudeBytes => MAX_INTEGER_MAGNITUDE_BYTES,
            Self::TotalIntegerPayload => MAX_TOTAL_INTEGER_PAYLOAD,
            Self::ContextEntries => MAX_CONTEXT_ENTRIES,
        }
    }
}

/// Invalid typed nesting in a definition path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefPathViolation {
    Empty,
    InvalidTopLevel {
        namespace: crate::Namespace,
    },
    InvalidChild {
        parent: crate::Namespace,
        child: crate::Namespace,
    },
    TooManySegments,
}

impl fmt::Display for DefPathViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("definition path is empty"),
            Self::InvalidTopLevel { namespace } => write!(
                formatter,
                "{} cannot be a top-level definition",
                namespace.wire_name()
            ),
            Self::InvalidChild { parent, child } => write!(
                formatter,
                "{} cannot be nested directly under {}",
                child.wire_name(),
                parent.wire_name()
            ),
            Self::TooManySegments => {
                formatter.write_str("the first M9 profile permits at most two definition segments")
            }
        }
    }
}

/// A portable-path policy violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathViolation {
    Empty,
    Absolute,
    Backslash,
    EmptySegment,
    CurrentDirectorySegment,
    ParentDirectorySegment,
    Nul,
}

impl fmt::Display for PathViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "path is empty",
            Self::Absolute => "path is absolute",
            Self::Backslash => "path contains a backslash",
            Self::EmptySegment => "path contains an empty segment",
            Self::CurrentDirectorySegment => "path contains a `.` segment",
            Self::ParentDirectorySegment => "path contains a `..` segment",
            Self::Nul => "path contains a NUL byte",
        })
    }
}

/// Exact-source encoding violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEncodingViolation {
    InvalidUtf8,
    Utf8ByteOrderMark,
}

impl fmt::Display for SourceEncodingViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidUtf8 => "source is not valid UTF-8",
            Self::Utf8ByteOrderMark => "source begins with a forbidden UTF-8 byte-order mark",
        })
    }
}

/// Failure while building a closed, deterministic resolved program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolveError {
    EmptySourceSet,
    ResourceLimit {
        dimension: ResourceDimension,
        maximum: u64,
        actual: u64,
    },
    InvalidLogicalModule {
        logical_module: String,
    },
    InvalidRepositoryPath {
        logical_module: String,
        repository_path: String,
        violation: PathViolation,
    },
    InvalidSourceEncoding {
        logical_module: String,
        repository_path: String,
        violation: SourceEncodingViolation,
    },
    InvalidIdentifier {
        logical_module: String,
        identifier: String,
        context: String,
    },
    InvalidDefinitionPath {
        logical_module: String,
        path: DefPath,
        violation: DefPathViolation,
        span: SourceSpan,
    },
    InvalidSpan {
        logical_module: String,
        context: String,
        span: SourceSpan,
        source_len: usize,
    },
    IncompleteProjection {
        logical_module: String,
        issues: Vec<ProjectionIssue>,
    },
    DuplicateLogicalModule {
        logical_module: String,
        repository_paths: Vec<String>,
    },
    DuplicateRepositoryPath {
        repository_path: String,
        logical_modules: Vec<String>,
    },
    DuplicateImport {
        logical_module: String,
        imported_module: String,
        spans: Vec<SourceSpan>,
    },
    MissingImport {
        logical_module: String,
        imported_module: String,
        span: SourceSpan,
    },
    ImportCycle {
        /// Deterministic edge path with the first module repeated at the end.
        cycle: Vec<String>,
    },
    DuplicateDefinition {
        logical_module: String,
        key: DeclarationKey,
        spans: Vec<SourceSpan>,
    },
    MissingDefinitionParent {
        logical_module: String,
        path: DefPath,
        parent: DefPath,
        span: SourceSpan,
    },
    TermSyntax {
        logical_module: String,
        owner: DefPath,
        span: SourceSpan,
        message: String,
    },
    MissingTermOwner {
        logical_module: String,
        owner: DefPath,
        span: SourceSpan,
    },
    DuplicateLocalBinder {
        logical_module: String,
        owner: DefPath,
        name: String,
        spans: Vec<SourceSpan>,
    },
    InvalidLocalBinder {
        logical_module: String,
        owner: DefPath,
        name: String,
        span: SourceSpan,
    },
    UnresolvedReference {
        logical_module: String,
        owner: DefPath,
        spelling: String,
        span: SourceSpan,
    },
    AmbiguousReference {
        logical_module: String,
        owner: DefPath,
        spelling: String,
        candidates: Vec<crate::DefId>,
        span: SourceSpan,
    },
    LocalShadowing {
        logical_module: String,
        owner: DefPath,
        spelling: String,
        candidates: Vec<crate::DefId>,
        span: SourceSpan,
    },
    InvalidReferenceForm {
        logical_module: String,
        owner: DefPath,
        context: String,
        span: SourceSpan,
    },
    DuplicateHirOrigin {
        logical_module: String,
        origin: crate::NodeId,
    },
    ResolutionReadback {
        logical_module: String,
        message: String,
        span: Option<SourceSpan>,
    },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySourceSet => formatter.write_str("cannot resolve an empty source set"),
            Self::ResourceLimit {
                dimension,
                maximum,
                actual,
            } => write!(
                formatter,
                "NMLT_HIR_RESOURCE_LIMIT: {dimension:?} maximum is {maximum}, got {actual}"
            ),
            Self::InvalidLogicalModule { logical_module } => write!(
                formatter,
                "logical module `{logical_module}` is not an ASCII NMLT identifier"
            ),
            Self::InvalidRepositoryPath {
                logical_module,
                repository_path,
                violation,
            } => write!(
                formatter,
                "invalid repository path `{repository_path}` for module `{logical_module}`: {violation}"
            ),
            Self::InvalidSourceEncoding {
                logical_module,
                repository_path,
                violation,
            } => write!(
                formatter,
                "invalid exact source for module `{logical_module}` at `{repository_path}`: {violation}"
            ),
            Self::InvalidIdentifier {
                logical_module,
                identifier,
                context,
            } => write!(
                formatter,
                "invalid identifier `{identifier}` in {context} of module `{logical_module}`"
            ),
            Self::InvalidDefinitionPath {
                logical_module,
                path,
                violation,
                ..
            } => write!(
                formatter,
                "invalid definition path `{}` in module `{logical_module}`: {violation}",
                display_def_path(path)
            ),
            Self::InvalidSpan {
                logical_module,
                context,
                span,
                source_len,
            } => write!(
                formatter,
                "invalid span {}..{} for {context} in module `{logical_module}` (source length {source_len})",
                span.start, span.end
            ),
            Self::IncompleteProjection {
                logical_module,
                issues,
            } => write!(
                formatter,
                "module `{logical_module}` has {} explicit projection issue(s)",
                issues.len()
            ),
            Self::DuplicateLogicalModule {
                logical_module,
                repository_paths,
            } => write!(
                formatter,
                "logical module `{logical_module}` occurs at {} source paths",
                repository_paths.len()
            ),
            Self::DuplicateRepositoryPath {
                repository_path,
                logical_modules,
            } => write!(
                formatter,
                "repository path `{repository_path}` is assigned to {} logical modules",
                logical_modules.len()
            ),
            Self::DuplicateImport {
                logical_module,
                imported_module,
                ..
            } => write!(
                formatter,
                "module `{logical_module}` imports `{imported_module}` more than once"
            ),
            Self::MissingImport {
                logical_module,
                imported_module,
                ..
            } => write!(
                formatter,
                "module `{logical_module}` imports `{imported_module}`, which is outside the closed source set"
            ),
            Self::ImportCycle { cycle } => {
                write!(formatter, "import cycle: {}", cycle.join(" -> "))
            }
            Self::DuplicateDefinition {
                logical_module,
                key,
                ..
            } => write!(
                formatter,
                "duplicate {} definition `{}` in module `{logical_module}`",
                key_namespace_name(key),
                display_def_path(&key.path)
            ),
            Self::MissingDefinitionParent {
                logical_module,
                path,
                parent,
                ..
            } => write!(
                formatter,
                "definition `{}` in module `{logical_module}` has no declared parent `{}`",
                display_def_path(path),
                display_def_path(parent)
            ),
            Self::TermSyntax {
                logical_module,
                owner,
                message,
                ..
            } => write!(
                formatter,
                "invalid M9 term owned by `{}` in module `{logical_module}`: {message}",
                display_def_path(owner)
            ),
            Self::MissingTermOwner {
                logical_module,
                owner,
                ..
            } => write!(
                formatter,
                "term owner `{}` is not declared in module `{logical_module}`",
                display_def_path(owner)
            ),
            Self::DuplicateLocalBinder {
                logical_module,
                owner,
                name,
                ..
            } => write!(
                formatter,
                "duplicate local binder `{name}` under `{}` in module `{logical_module}`",
                display_def_path(owner)
            ),
            Self::InvalidLocalBinder {
                logical_module,
                owner,
                name,
                ..
            } => write!(
                formatter,
                "invalid local binder `{name}` under `{}` in module `{logical_module}`",
                display_def_path(owner)
            ),
            Self::UnresolvedReference {
                logical_module,
                owner,
                spelling,
                ..
            } => write!(
                formatter,
                "unresolved reference `{spelling}` under `{}` in module `{logical_module}`",
                display_def_path(owner)
            ),
            Self::AmbiguousReference {
                logical_module,
                owner,
                spelling,
                candidates,
                ..
            } => write!(
                formatter,
                "ambiguous reference `{spelling}` under `{}` in module `{logical_module}` ({} candidates)",
                display_def_path(owner),
                candidates.len()
            ),
            Self::LocalShadowing {
                logical_module,
                owner,
                spelling,
                ..
            } => write!(
                formatter,
                "local `{spelling}` shadows a visible definition under `{}` in module `{logical_module}`",
                display_def_path(owner)
            ),
            Self::InvalidReferenceForm {
                logical_module,
                owner,
                context,
                ..
            } => write!(
                formatter,
                "invalid reference form for {context} under `{}` in module `{logical_module}`",
                display_def_path(owner)
            ),
            Self::DuplicateHirOrigin {
                logical_module,
                origin,
            } => write!(
                formatter,
                "duplicate HIR origin `{origin}` in module `{logical_module}`"
            ),
            Self::ResolutionReadback {
                logical_module,
                message,
                ..
            } => write!(
                formatter,
                "resolution readback failed in module `{logical_module}`: {message}"
            ),
        }
    }
}

impl std::error::Error for ResolveError {}

/// A canonical candidate included in lookup diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefinitionCandidate {
    pub logical_module: String,
    pub key: DeclarationKey,
    pub id: crate::DefId,
}

/// Failure to resolve a textual reference against an already resolved program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookupError {
    UnknownOriginModule {
        logical_module: String,
    },
    InvalidReferenceIdentifier {
        logical_module: String,
        identifier: String,
        context: String,
    },
    InvalidReferencePath {
        logical_module: String,
        path: DefPath,
        violation: DefPathViolation,
        span: SourceSpan,
    },
    InvalidReferenceSpan {
        logical_module: String,
        span: SourceSpan,
        source_len: usize,
    },
    ModuleNotVisible {
        from_module: String,
        requested_module: String,
        visible_modules: Vec<String>,
    },
    MissingDefinition {
        from_module: String,
        qualifier: Option<String>,
        key: DeclarationKey,
        span: SourceSpan,
    },
    AmbiguousDefinition {
        from_module: String,
        key: DeclarationKey,
        candidates: Vec<DefinitionCandidate>,
        span: SourceSpan,
    },
    StrictShadowing {
        from_module: String,
        key: DeclarationKey,
        local: Box<DefinitionCandidate>,
        imported: Vec<DefinitionCandidate>,
        span: SourceSpan,
    },
}

impl fmt::Display for LookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOriginModule { logical_module } => {
                write!(formatter, "unknown origin module `{logical_module}`")
            }
            Self::InvalidReferenceIdentifier {
                logical_module,
                identifier,
                context,
            } => write!(
                formatter,
                "invalid reference identifier `{identifier}` in {context} of module `{logical_module}`"
            ),
            Self::InvalidReferencePath {
                logical_module,
                path,
                violation,
                ..
            } => write!(
                formatter,
                "invalid reference path `{}` in module `{logical_module}`: {violation}",
                display_def_path(path)
            ),
            Self::InvalidReferenceSpan {
                logical_module,
                span,
                source_len,
            } => write!(
                formatter,
                "invalid reference span {}..{} in module `{logical_module}` (source length {source_len})",
                span.start, span.end
            ),
            Self::ModuleNotVisible {
                from_module,
                requested_module,
                ..
            } => write!(
                formatter,
                "module `{requested_module}` is neither `{from_module}` nor one of its direct imports"
            ),
            Self::MissingDefinition {
                from_module,
                qualifier,
                key,
                ..
            } => {
                let owner = qualifier.as_deref().unwrap_or(from_module);
                write!(
                    formatter,
                    "missing {} definition `{}::{}` visible from `{from_module}`",
                    key_namespace_name(key),
                    owner,
                    display_def_path(&key.path)
                )
            }
            Self::AmbiguousDefinition {
                from_module,
                key,
                candidates,
                ..
            } => write!(
                formatter,
                "ambiguous {} definition `{}` from `{from_module}`: {} candidates",
                key_namespace_name(key),
                display_def_path(&key.path),
                candidates.len()
            ),
            Self::StrictShadowing {
                from_module, key, ..
            } => write!(
                formatter,
                "local {} definition `{}` in `{from_module}` conflicts with a directly imported definition",
                key_namespace_name(key),
                display_def_path(&key.path)
            ),
        }
    }
}

impl std::error::Error for LookupError {}

/// Resolves a complete, closed set of projected modules.
///
/// Input and import order do not affect the result. Exact bytes and source
/// spans are retained, but spans are excluded from semantic identities.
pub fn resolve_modules(modules: Vec<ProjectedModule>) -> Result<ResolvedProgram, ResolveError> {
    resolve_module_inputs(modules.into_iter().map(|module| module.input).collect())
}

pub(crate) fn resolve_module_inputs(
    mut inputs: Vec<ModuleInput>,
) -> Result<ResolvedProgram, ResolveError> {
    if inputs.is_empty() {
        return Err(ResolveError::EmptySourceSet);
    }
    enforce_resource_limits(&inputs)?;

    inputs.sort_by(|left, right| {
        left.logical_module
            .as_bytes()
            .cmp(right.logical_module.as_bytes())
            .then_with(|| {
                left.repository_path
                    .as_bytes()
                    .cmp(right.repository_path.as_bytes())
            })
            .then_with(|| left.exact_bytes.cmp(&right.exact_bytes))
    });

    reject_duplicate_modules(&inputs)?;
    reject_duplicate_paths(&inputs)?;
    for input in &inputs {
        validate_module(input)?;
    }

    let module_names = inputs
        .iter()
        .map(|input| input.logical_module.as_str())
        .collect::<BTreeSet<_>>();
    let graph = build_import_graph(&inputs, &module_names)?;
    let dependency_order = topological_order(&graph)?;

    let source_entries = inputs
        .iter()
        .map(|input| SourceSetEntry {
            repository_path: &input.repository_path,
            exact_bytes: &input.exact_bytes,
        })
        .collect::<Vec<_>>();
    let source_set_id =
        SourceSetId::from_entries(&source_entries).map_err(|error| match error {
            SourceSetIdentityError::DuplicatePath(repository_path) => {
                let mut logical_modules = inputs
                    .iter()
                    .filter(|input| input.repository_path == repository_path)
                    .map(|input| input.logical_module.clone())
                    .collect::<Vec<_>>();
                logical_modules.sort();
                ResolveError::DuplicateRepositoryPath {
                    repository_path,
                    logical_modules,
                }
            }
        })?;

    let module_map_entries = inputs
        .iter()
        .map(|input| ModuleMapEntry {
            logical_module: &input.logical_module,
            repository_path: &input.repository_path,
        })
        .collect::<Vec<_>>();
    let module_map_id = module_map_id(source_set_id, &module_map_entries);
    let canonical_surface = canonical_surface_bytes(&inputs);
    let surface_program_id = surface_program_id(source_set_id, module_map_id, &canonical_surface);

    let module_ids = inputs
        .iter()
        .map(|input| {
            (
                input.logical_module.clone(),
                module_id(module_map_id, &input.logical_module),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut modules = BTreeMap::new();
    let mut pending_terms = BTreeMap::new();
    for input in inputs {
        pending_terms.insert(
            input.logical_module.clone(),
            PendingModuleTerms {
                local_binders: input.local_binders,
                raw_terms: input.raw_terms,
            },
        );
        let id = module_ids[&input.logical_module];
        let mut imports = input.imports;
        imports.sort_by(|left, right| left.logical_module.cmp(&right.logical_module));
        let imports = imports
            .into_iter()
            .map(|import| ResolvedImport {
                module_id: module_ids[&import.logical_module],
                logical_module: import.logical_module,
                span: import.span,
            })
            .collect::<Vec<_>>();

        let declarations = resolve_declarations(&input.logical_module, id, input.declarations)?;
        let source_id = SourceId::from_bytes(&input.exact_bytes);
        let logical_module = input.logical_module;
        modules.insert(
            logical_module.clone(),
            ResolvedModule {
                logical_module,
                repository_path: input.repository_path,
                source_id,
                id,
                imports,
                declarations,
                local_binders: BTreeMap::new(),
                hir_roots: Vec::new(),
                hir_nodes: BTreeMap::new(),
                exact_bytes: input.exact_bytes,
            },
        );
    }

    let resolution_map = resolve_program_terms(&pending_terms, &mut modules)?;
    let canonical_hir = crate::resolve_terms::canonical_hir_bytes(&modules, &resolution_map);
    let resolution_id = resolution_id(source_set_id, module_map_id, &canonical_hir);

    let program = ResolvedProgram {
        source_set_id,
        module_map_id,
        surface_program_id,
        resolution_id,
        dependency_order,
        modules,
        resolution_map,
    };
    crate::verify_resolution_readback(&program)?;
    Ok(program)
}

impl ResolvedProgram {
    /// Resolves a qualified or unqualified reference from one module.
    ///
    /// Only the origin module and its direct imports are visible. Local/imported
    /// collisions fail under strict shadowing instead of silently changing
    /// meaning when an import is added.
    pub fn resolve_name(
        &self,
        from_module: &str,
        reference: &NameReference,
    ) -> Result<&ResolvedDeclaration, LookupError> {
        let origin =
            self.modules
                .get(from_module)
                .ok_or_else(|| LookupError::UnknownOriginModule {
                    logical_module: from_module.to_owned(),
                })?;
        validate_reference(origin, reference)?;

        if let Some(qualifier) = &reference.qualifier {
            if qualifier != from_module
                && !origin
                    .imports
                    .iter()
                    .any(|import| import.logical_module == *qualifier)
            {
                let mut visible_modules = origin
                    .imports
                    .iter()
                    .map(|import| import.logical_module.clone())
                    .collect::<Vec<_>>();
                visible_modules.push(from_module.to_owned());
                visible_modules.sort();
                return Err(LookupError::ModuleNotVisible {
                    from_module: from_module.to_owned(),
                    requested_module: qualifier.clone(),
                    visible_modules,
                });
            }
            let target = &self.modules[qualifier];
            return target.declarations.get(&reference.key).ok_or_else(|| {
                LookupError::MissingDefinition {
                    from_module: from_module.to_owned(),
                    qualifier: Some(qualifier.clone()),
                    key: reference.key.clone(),
                    span: reference.span,
                }
            });
        }

        let local = origin.declarations.get(&reference.key);
        let mut imported = origin
            .imports
            .iter()
            .filter_map(|import| {
                let module = &self.modules[&import.logical_module];
                module
                    .declarations
                    .get(&reference.key)
                    .map(|definition| (module, definition))
            })
            .collect::<Vec<_>>();
        imported.sort_by(|(left, _), (right, _)| left.logical_module.cmp(&right.logical_module));

        match (local, imported.as_slice()) {
            (Some(definition), []) => Ok(definition),
            (Some(definition), imported) => Err(LookupError::StrictShadowing {
                from_module: from_module.to_owned(),
                key: reference.key.clone(),
                local: Box::new(candidate(origin, definition)),
                imported: imported
                    .iter()
                    .map(|(module, definition)| candidate(module, definition))
                    .collect(),
                span: reference.span,
            }),
            (None, [(_, definition)]) => Ok(definition),
            (None, []) => Err(LookupError::MissingDefinition {
                from_module: from_module.to_owned(),
                qualifier: None,
                key: reference.key.clone(),
                span: reference.span,
            }),
            (None, imported) => Err(LookupError::AmbiguousDefinition {
                from_module: from_module.to_owned(),
                key: reference.key.clone(),
                candidates: imported
                    .iter()
                    .map(|(module, definition)| candidate(module, definition))
                    .collect(),
                span: reference.span,
            }),
        }
    }
}

fn validate_module(input: &ModuleInput) -> Result<(), ResolveError> {
    if !is_identifier(&input.logical_module) {
        return Err(ResolveError::InvalidLogicalModule {
            logical_module: input.logical_module.clone(),
        });
    }
    if let Some(violation) = path_violation(&input.repository_path) {
        return Err(ResolveError::InvalidRepositoryPath {
            logical_module: input.logical_module.clone(),
            repository_path: input.repository_path.clone(),
            violation,
        });
    }
    let source = std::str::from_utf8(&input.exact_bytes).map_err(|_| {
        ResolveError::InvalidSourceEncoding {
            logical_module: input.logical_module.clone(),
            repository_path: input.repository_path.clone(),
            violation: SourceEncodingViolation::InvalidUtf8,
        }
    })?;
    if input.exact_bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(ResolveError::InvalidSourceEncoding {
            logical_module: input.logical_module.clone(),
            repository_path: input.repository_path.clone(),
            violation: SourceEncodingViolation::Utf8ByteOrderMark,
        });
    }

    let mut imports = input.imports.iter().collect::<Vec<_>>();
    imports.sort_by(|left, right| {
        left.logical_module
            .cmp(&right.logical_module)
            .then_with(|| left.span.cmp(&right.span))
    });
    for import in imports {
        if !is_identifier(&import.logical_module) {
            return Err(ResolveError::InvalidIdentifier {
                logical_module: input.logical_module.clone(),
                identifier: import.logical_module.clone(),
                context: "import".to_owned(),
            });
        }
        validate_span(
            &input.logical_module,
            source,
            import.span,
            format!("import `{}`", import.logical_module),
        )?;
    }

    let mut declarations = input.declarations.iter().collect::<Vec<_>>();
    declarations.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.span.cmp(&right.span))
    });
    for declaration in declarations {
        validate_declaration_identifiers(&input.logical_module, declaration)?;
        validate_span(
            &input.logical_module,
            source,
            declaration.span,
            format!(
                "{} declaration `{}`",
                declaration
                    .path
                    .terminal_namespace()
                    .map_or("empty-path", |namespace| namespace.wire_name()),
                display_def_path(&declaration.path)
            ),
        )?;
    }

    for binder in &input.local_binders {
        validate_span(
            &input.logical_module,
            source,
            binder.name_span,
            format!("local binder `{}`", binder.name),
        )?;
        validate_raw_term_input(&input.logical_module, source, &binder.declared_type)?;
    }
    for term in &input.raw_terms {
        validate_raw_term_input(&input.logical_module, source, term)?;
    }

    let mut projection_issues = input.projection_issues.iter().collect::<Vec<_>>();
    projection_issues.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.span.cmp(&right.span))
            .then_with(|| left.message.cmp(&right.message))
    });
    for issue in projection_issues {
        if let Some(span) = issue.span {
            validate_span(
                &input.logical_module,
                source,
                span,
                "projection issue".to_owned(),
            )?;
        }
    }
    if !input.projection_issues.is_empty() {
        let mut issues = input.projection_issues.clone();
        issues.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.span.cmp(&right.span))
                .then_with(|| left.message.cmp(&right.message))
        });
        return Err(ResolveError::IncompleteProjection {
            logical_module: input.logical_module.clone(),
            issues,
        });
    }

    Ok(())
}

fn validate_raw_term_input(
    logical_module: &str,
    source: &str,
    term: &crate::term::RawTermInput,
) -> Result<(), ResolveError> {
    validate_span(
        logical_module,
        source,
        term.span,
        "raw type/expression".to_owned(),
    )?;
    if source.get(term.span.start..term.span.end) != Some(term.source.as_str()) {
        return Err(ResolveError::ResolutionReadback {
            logical_module: logical_module.to_owned(),
            message: "raw term bytes do not match their exact-source span".to_owned(),
            span: Some(term.span),
        });
    }
    Ok(())
}

fn validate_declaration_identifiers(
    logical_module: &str,
    declaration: &DeclarationInput,
) -> Result<(), ResolveError> {
    if let Some(violation) = def_path_violation(&declaration.path) {
        return Err(ResolveError::InvalidDefinitionPath {
            logical_module: logical_module.to_owned(),
            path: declaration.path.clone(),
            violation,
            span: declaration.span,
        });
    }
    for segment in &declaration.path.segments {
        if !is_identifier(&segment.name) {
            return Err(ResolveError::InvalidIdentifier {
                logical_module: logical_module.to_owned(),
                identifier: segment.name.clone(),
                context: format!("{} definition-path segment", segment.namespace.wire_name()),
            });
        }
    }
    Ok(())
}

fn validate_reference(
    origin: &ResolvedModule,
    reference: &NameReference,
) -> Result<(), LookupError> {
    if !valid_span(
        std::str::from_utf8(&origin.exact_bytes)
            .expect("resolved modules have already passed UTF-8 validation"),
        reference.span,
    ) {
        return Err(LookupError::InvalidReferenceSpan {
            logical_module: origin.logical_module.clone(),
            span: reference.span,
            source_len: origin.exact_bytes.len(),
        });
    }
    if let Some(qualifier) = &reference.qualifier {
        if !is_identifier(qualifier) {
            return Err(LookupError::InvalidReferenceIdentifier {
                logical_module: origin.logical_module.clone(),
                identifier: qualifier.clone(),
                context: "module qualifier".to_owned(),
            });
        }
    }
    if let Some(violation) = def_path_violation(&reference.key.path) {
        return Err(LookupError::InvalidReferencePath {
            logical_module: origin.logical_module.clone(),
            path: reference.key.path.clone(),
            violation,
            span: reference.span,
        });
    }
    for segment in &reference.key.path.segments {
        if !is_identifier(&segment.name) {
            return Err(LookupError::InvalidReferenceIdentifier {
                logical_module: origin.logical_module.clone(),
                identifier: segment.name.clone(),
                context: format!("{} reference-path segment", segment.namespace.wire_name()),
            });
        }
    }
    Ok(())
}

fn validate_span(
    logical_module: &str,
    source: &str,
    span: SourceSpan,
    context: String,
) -> Result<(), ResolveError> {
    if valid_span(source, span) {
        Ok(())
    } else {
        Err(ResolveError::InvalidSpan {
            logical_module: logical_module.to_owned(),
            context,
            span,
            source_len: source.len(),
        })
    }
}

fn valid_span(source: &str, span: SourceSpan) -> bool {
    span.start <= span.end
        && span.end <= source.len()
        && source.is_char_boundary(span.start)
        && source.is_char_boundary(span.end)
}

fn path_violation(path: &str) -> Option<PathViolation> {
    if path.is_empty() {
        return Some(PathViolation::Empty);
    }
    if path.starts_with('/') {
        return Some(PathViolation::Absolute);
    }
    if path.contains('\\') {
        return Some(PathViolation::Backslash);
    }
    if path.contains('\0') {
        return Some(PathViolation::Nul);
    }
    for segment in path.split('/') {
        match segment {
            "" => return Some(PathViolation::EmptySegment),
            "." => return Some(PathViolation::CurrentDirectorySegment),
            ".." => return Some(PathViolation::ParentDirectorySegment),
            _ => {}
        }
    }
    None
}

fn is_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    matches!(bytes.next(), Some(first) if first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn def_path_violation(path: &DefPath) -> Option<DefPathViolation> {
    use crate::Namespace::{
        Action, Capability, Constructor, Observation, Property, State, System, SystemInput, Type,
        Value,
    };

    match path.segments.as_slice() {
        [] => Some(DefPathViolation::Empty),
        [segment] if matches!(segment.namespace, Type | Value | System) => None,
        [segment] => Some(DefPathViolation::InvalidTopLevel {
            namespace: segment.namespace,
        }),
        [parent, child]
            if matches!(
                (parent.namespace, child.namespace),
                (Type, Constructor)
                    | (
                        System,
                        Value | State | Action | SystemInput | Capability | Property | Observation
                    )
            ) =>
        {
            None
        }
        [parent, child] => Some(DefPathViolation::InvalidChild {
            parent: parent.namespace,
            child: child.namespace,
        }),
        _ => Some(DefPathViolation::TooManySegments),
    }
}

fn enforce_resource_limits(inputs: &[ModuleInput]) -> Result<(), ResolveError> {
    enforce_limit(ResourceDimension::Modules, usize_as_u64(inputs.len()))?;

    let mut total_source_bytes = 0_u64;
    let mut hir_bytes = 8_u64;
    let mut hir_nodes = 0_u64;
    let mut context_entries = 0_u64;
    for input in inputs {
        let logical_len = usize_as_u64(input.logical_module.len());
        enforce_limit(ResourceDimension::IdentifierBytes, logical_len)?;
        enforce_limit(ResourceDimension::LogicalModuleOrPathBytes, logical_len)?;
        let path_len = usize_as_u64(input.repository_path.len());
        enforce_limit(ResourceDimension::LogicalModuleOrPathBytes, path_len)?;
        let source_len = usize_as_u64(input.exact_bytes.len());
        enforce_limit(ResourceDimension::SourceBytes, source_len)?;
        checked_add_limited(
            &mut total_source_bytes,
            source_len,
            ResourceDimension::TotalSourceBytes,
        )?;

        checked_add_limited(&mut hir_nodes, 1, ResourceDimension::HirNodes)?;
        checked_add_limited(
            &mut hir_bytes,
            64_u64.saturating_add(logical_len).saturating_add(path_len),
            ResourceDimension::HirBytes,
        )?;

        for import in &input.imports {
            let import_len = usize_as_u64(import.logical_module.len());
            enforce_limit(ResourceDimension::IdentifierBytes, import_len)?;
            checked_add_limited(&mut hir_nodes, 1, ResourceDimension::HirNodes)?;
            checked_add_limited(
                &mut hir_bytes,
                8_u64.saturating_add(import_len),
                ResourceDimension::HirBytes,
            )?;
        }

        checked_add_limited(
            &mut context_entries,
            usize_as_u64(input.declarations.len()),
            ResourceDimension::ContextEntries,
        )?;
        for declaration in &input.declarations {
            let depth = usize_as_u64(declaration.path.segments.len());
            enforce_limit(ResourceDimension::DefinitionPathDepth, depth)?;
            checked_add_limited(&mut hir_nodes, 1, ResourceDimension::HirNodes)?;
            checked_add_limited(&mut hir_bytes, 8, ResourceDimension::HirBytes)?;
            for segment in &declaration.path.segments {
                let name_len = usize_as_u64(segment.name.len());
                enforce_limit(ResourceDimension::IdentifierBytes, name_len)?;
                checked_add_limited(&mut hir_nodes, 1, ResourceDimension::HirNodes)?;
                checked_add_limited(
                    &mut hir_bytes,
                    9_u64.saturating_add(name_len),
                    ResourceDimension::HirBytes,
                )?;
            }
        }

        checked_add_limited(
            &mut context_entries,
            usize_as_u64(input.local_binders.len()),
            ResourceDimension::ContextEntries,
        )?;
        for binder in &input.local_binders {
            let name_len = usize_as_u64(binder.name.len());
            enforce_limit(ResourceDimension::IdentifierBytes, name_len)?;
            checked_add_limited(&mut hir_nodes, 2, ResourceDimension::HirNodes)?;
            checked_add_limited(
                &mut hir_bytes,
                64_u64
                    .saturating_add(name_len)
                    .saturating_add(usize_as_u64(binder.declared_type.source.len())),
                ResourceDimension::HirBytes,
            )?;
        }
        for term in &input.raw_terms {
            checked_add_limited(&mut hir_nodes, 1, ResourceDimension::HirNodes)?;
            checked_add_limited(
                &mut hir_bytes,
                48_u64.saturating_add(usize_as_u64(term.source.len())),
                ResourceDimension::HirBytes,
            )?;
        }

        for issue in &input.projection_issues {
            checked_add_limited(&mut hir_nodes, 1, ResourceDimension::HirNodes)?;
            checked_add_limited(
                &mut hir_bytes,
                9_u64.saturating_add(usize_as_u64(issue.message.len())),
                ResourceDimension::HirBytes,
            )?;
        }
    }
    Ok(())
}

fn usize_as_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn enforce_limit(dimension: ResourceDimension, actual: u64) -> Result<(), ResolveError> {
    let maximum = dimension.maximum();
    if actual > maximum {
        Err(ResolveError::ResourceLimit {
            dimension,
            maximum,
            actual,
        })
    } else {
        Ok(())
    }
}

fn checked_add_limited(
    total: &mut u64,
    amount: u64,
    dimension: ResourceDimension,
) -> Result<(), ResolveError> {
    let actual = total.checked_add(amount).unwrap_or(u64::MAX);
    enforce_limit(dimension, actual)?;
    *total = actual;
    Ok(())
}

fn reject_duplicate_modules(inputs: &[ModuleInput]) -> Result<(), ResolveError> {
    let mut by_name: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for input in inputs {
        by_name
            .entry(&input.logical_module)
            .or_default()
            .push(&input.repository_path);
    }
    if let Some((logical_module, paths)) = by_name.into_iter().find(|(_, paths)| paths.len() > 1) {
        let mut repository_paths = paths.into_iter().map(str::to_owned).collect::<Vec<_>>();
        repository_paths.sort();
        return Err(ResolveError::DuplicateLogicalModule {
            logical_module: logical_module.to_owned(),
            repository_paths,
        });
    }
    Ok(())
}

fn reject_duplicate_paths(inputs: &[ModuleInput]) -> Result<(), ResolveError> {
    let mut by_path: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for input in inputs {
        by_path
            .entry(&input.repository_path)
            .or_default()
            .push(&input.logical_module);
    }
    if let Some((repository_path, modules)) =
        by_path.into_iter().find(|(_, modules)| modules.len() > 1)
    {
        let mut logical_modules = modules.into_iter().map(str::to_owned).collect::<Vec<_>>();
        logical_modules.sort();
        return Err(ResolveError::DuplicateRepositoryPath {
            repository_path: repository_path.to_owned(),
            logical_modules,
        });
    }
    Ok(())
}

fn build_import_graph(
    inputs: &[ModuleInput],
    module_names: &BTreeSet<&str>,
) -> Result<BTreeMap<String, Vec<String>>, ResolveError> {
    let mut graph = BTreeMap::new();
    for input in inputs {
        let mut imports = input.imports.iter().collect::<Vec<_>>();
        imports.sort_by(|left, right| {
            left.logical_module
                .cmp(&right.logical_module)
                .then_with(|| left.span.cmp(&right.span))
        });
        let mut cursor = 0;
        while cursor < imports.len() {
            let imported_module = imports[cursor].logical_module.as_str();
            let end = imports[cursor..]
                .iter()
                .position(|import| import.logical_module != imported_module)
                .map_or(imports.len(), |offset| cursor + offset);
            if end - cursor > 1 {
                return Err(ResolveError::DuplicateImport {
                    logical_module: input.logical_module.clone(),
                    imported_module: imported_module.to_owned(),
                    spans: imports[cursor..end]
                        .iter()
                        .map(|import| import.span)
                        .collect(),
                });
            }
            cursor = end;
        }

        for import in &imports {
            if !module_names.contains(import.logical_module.as_str()) {
                return Err(ResolveError::MissingImport {
                    logical_module: input.logical_module.clone(),
                    imported_module: import.logical_module.clone(),
                    span: import.span,
                });
            }
        }
        graph.insert(
            input.logical_module.clone(),
            imports
                .into_iter()
                .map(|import| import.logical_module.clone())
                .collect(),
        );
    }
    Ok(graph)
}

fn topological_order(graph: &BTreeMap<String, Vec<String>>) -> Result<Vec<String>, ResolveError> {
    let mut remaining_dependencies = graph
        .iter()
        .map(|(module, imports)| (module.clone(), imports.len()))
        .collect::<BTreeMap<_, _>>();
    let mut dependents = graph
        .keys()
        .map(|module| (module.clone(), Vec::new()))
        .collect::<BTreeMap<_, Vec<String>>>();
    for (module, imports) in graph {
        for import in imports {
            dependents
                .get_mut(import)
                .expect("closed import graph")
                .push(module.clone());
        }
    }
    for modules in dependents.values_mut() {
        modules.sort();
    }

    let mut ready = remaining_dependencies
        .iter()
        .filter_map(|(module, count)| (*count == 0).then_some(module.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(graph.len());
    while let Some(module) = ready.pop_first() {
        order.push(module.clone());
        for dependent in &dependents[&module] {
            let count = remaining_dependencies
                .get_mut(dependent)
                .expect("dependent is a graph node");
            *count -= 1;
            if *count == 0 {
                ready.insert(dependent.clone());
            }
        }
    }

    if order.len() != graph.len() {
        return Err(ResolveError::ImportCycle {
            cycle: deterministic_cycle(graph),
        });
    }
    Ok(order)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisitState {
    Active,
    Complete,
}

fn deterministic_cycle(graph: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut states = BTreeMap::new();
    for start in graph.keys() {
        if states.contains_key(start) {
            continue;
        }
        states.insert(start.clone(), VisitState::Active);
        let mut stack = vec![(start.clone(), 0_usize)];
        while let Some((module, next_import)) = stack.last_mut() {
            if *next_import == graph[module].len() {
                let completed = module.clone();
                stack.pop();
                states.insert(completed, VisitState::Complete);
                continue;
            }

            let import = graph[module][*next_import].clone();
            *next_import += 1;
            match states.get(&import) {
                Some(VisitState::Active) => {
                    let cycle_start = stack
                        .iter()
                        .position(|(entry, _)| entry == &import)
                        .expect("active module is on the DFS stack");
                    let mut cycle = stack[cycle_start..]
                        .iter()
                        .map(|(entry, _)| entry.clone())
                        .collect::<Vec<_>>();
                    cycle.push(import);
                    return cycle;
                }
                Some(VisitState::Complete) => {}
                None => {
                    states.insert(import.clone(), VisitState::Active);
                    stack.push((import, 0));
                }
            }
        }
    }
    unreachable!("topological sort reported a cycle, but DFS did not find one")
}

fn resolve_declarations(
    logical_module: &str,
    module_id: crate::ModuleId,
    declarations: Vec<DeclarationInput>,
) -> Result<BTreeMap<DeclarationKey, ResolvedDeclaration>, ResolveError> {
    let mut declarations = declarations
        .into_iter()
        .map(|declaration| (DeclarationKey::from(&declaration), declaration))
        .collect::<Vec<_>>();
    declarations.sort_by(|(left_key, left), (right_key, right)| {
        left_key
            .cmp(right_key)
            .then_with(|| left.span.cmp(&right.span))
    });

    let mut resolved = BTreeMap::new();
    let mut cursor = 0;
    while cursor < declarations.len() {
        let key = &declarations[cursor].0;
        let end = declarations[cursor..]
            .iter()
            .position(|(candidate, _)| candidate != key)
            .map_or(declarations.len(), |offset| cursor + offset);
        if end - cursor > 1 {
            return Err(ResolveError::DuplicateDefinition {
                logical_module: logical_module.to_owned(),
                key: key.clone(),
                spans: declarations[cursor..end]
                    .iter()
                    .map(|(_, declaration)| declaration.span)
                    .collect(),
            });
        }
        let (key, declaration) = &declarations[cursor];
        resolved.insert(
            key.clone(),
            ResolvedDeclaration {
                id: definition_id(
                    module_id,
                    key.path
                        .segments
                        .iter()
                        .map(|segment| (segment.namespace.wire_tag(), segment.name.as_str())),
                ),
                key: key.clone(),
                span: declaration.span,
                flavor: declaration.flavor,
            },
        );
        cursor = end;
    }

    for declaration in resolved.values() {
        if declaration.key.path.segments.len() == 2 {
            let parent = DefPath::new([declaration.key.path.segments[0].clone()]);
            if !resolved.contains_key(&DeclarationKey::new(parent.clone())) {
                return Err(ResolveError::MissingDefinitionParent {
                    logical_module: logical_module.to_owned(),
                    path: declaration.key.path.clone(),
                    parent,
                    span: declaration.span,
                });
            }
        }
    }
    Ok(resolved)
}

fn canonical_surface_bytes(inputs: &[ModuleInput]) -> Vec<u8> {
    let mut output = Vec::new();
    push_count(&mut output, inputs.len());
    for input in inputs {
        push_text(&mut output, &input.logical_module);
        push_text(&mut output, &input.repository_path);
        push_count(&mut output, input.imports.len());
        let mut imports = input.imports.iter().collect::<Vec<_>>();
        imports.sort_by(|left, right| left.logical_module.cmp(&right.logical_module));
        for import in imports {
            push_text(&mut output, &import.logical_module);
        }
        push_count(&mut output, input.declarations.len());
        let mut declarations = input.declarations.iter().collect::<Vec<_>>();
        declarations.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.flavor.cmp(&right.flavor))
        });
        for declaration in declarations {
            encode_path(&mut output, &declaration.path);
            output.push(declaration.flavor.wire_tag());
        }
        push_count(&mut output, input.local_binders.len());
        let mut binders = input.local_binders.iter().collect::<Vec<_>>();
        binders.sort_by(|left, right| {
            left.owner
                .cmp(&right.owner)
                .then_with(|| left.index.cmp(&right.index))
                .then_with(|| left.name.cmp(&right.name))
        });
        for binder in binders {
            encode_path(&mut output, &binder.owner);
            output.extend_from_slice(&binder.index.to_be_bytes());
            push_text(&mut output, &binder.name);
            encode_raw_term(&mut output, &binder.declared_type);
        }
        push_count(&mut output, input.raw_terms.len());
        let mut terms = input.raw_terms.iter().collect::<Vec<_>>();
        terms.sort_by(|left, right| {
            left.owner
                .cmp(&right.owner)
                .then_with(|| left.root.cmp(&right.root))
                .then_with(|| raw_term_kind_tag(left.kind).cmp(&raw_term_kind_tag(right.kind)))
                .then_with(|| left.source.cmp(&right.source))
        });
        for term in terms {
            encode_raw_term(&mut output, term);
        }
    }
    output
}

fn encode_path(output: &mut Vec<u8>, path: &DefPath) {
    push_count(output, path.segments.len());
    for segment in &path.segments {
        output.push(segment.namespace.wire_tag());
        push_text(output, &segment.name);
    }
}

fn encode_raw_term(output: &mut Vec<u8>, term: &crate::term::RawTermInput) {
    encode_path(output, &term.owner);
    match term.root {
        TermRootInput::DeclaredType => output.push(1),
        TermRootInput::Initializer => output.push(2),
        TermRootInput::ActionParameterType(index) => {
            output.push(3);
            output.extend_from_slice(&index.to_be_bytes());
        }
        TermRootInput::Guard(index) => {
            output.push(4);
            output.extend_from_slice(&index.to_be_bytes());
        }
        TermRootInput::UpdateTarget(index) => {
            output.push(5);
            output.extend_from_slice(&index.to_be_bytes());
        }
        TermRootInput::UpdateValue(index) => {
            output.push(6);
            output.extend_from_slice(&index.to_be_bytes());
        }
        TermRootInput::Output(index) => {
            output.push(7);
            output.extend_from_slice(&index.to_be_bytes());
        }
        TermRootInput::Consume(index) => {
            output.push(8);
            output.extend_from_slice(&index.to_be_bytes());
        }
        TermRootInput::PropertyBody => output.push(9),
        TermRootInput::ObservationItems => output.push(10),
    }
    output.push(raw_term_kind_tag(term.kind));
    push_text(output, &term.source);
}

const fn raw_term_kind_tag(kind: RawTermInputKind) -> u8 {
    match kind {
        RawTermInputKind::Type => 1,
        RawTermInputKind::Expression => 2,
        RawTermInputKind::ExpressionList => 3,
        RawTermInputKind::UpdateTarget => 4,
        RawTermInputKind::Consume => 5,
    }
}

fn push_count(output: &mut Vec<u8>, count: usize) {
    output.extend_from_slice(&(count as u64).to_be_bytes());
}

fn push_text(output: &mut Vec<u8>, text: &str) {
    push_count(output, text.len());
    output.extend_from_slice(text.as_bytes());
}

fn candidate(module: &ResolvedModule, definition: &ResolvedDeclaration) -> DefinitionCandidate {
    DefinitionCandidate {
        logical_module: module.logical_module.clone(),
        key: definition.key.clone(),
        id: definition.id,
    }
}

fn display_def_path(path: &DefPath) -> String {
    path.segments
        .iter()
        .map(|segment| format!("{}:{}", segment.namespace.wire_name(), segment.name))
        .collect::<Vec<_>>()
        .join("::")
}

fn key_namespace_name(key: &DeclarationKey) -> &'static str {
    key.namespace()
        .map_or("empty-path", |namespace| namespace.wire_name())
}

#[cfg(test)]
mod tests {
    use super::{PathViolation, is_identifier, path_violation};

    #[test]
    fn identifier_policy_matches_the_current_lexer() {
        assert!(is_identifier("_x9"));
        assert!(is_identifier("System"));
        assert!(!is_identifier(""));
        assert!(!is_identifier("9x"));
        assert!(!is_identifier("a.b"));
        assert!(!is_identifier("λ"));
    }

    #[test]
    fn portable_path_policy_is_lexical_and_fail_closed() {
        assert_eq!(path_violation("src/a.nmlt"), None);
        assert_eq!(path_violation(""), Some(PathViolation::Empty));
        assert_eq!(path_violation("/a"), Some(PathViolation::Absolute));
        assert_eq!(path_violation("a\\b"), Some(PathViolation::Backslash));
        assert_eq!(path_violation("a//b"), Some(PathViolation::EmptySegment));
        assert_eq!(
            path_violation("a/./b"),
            Some(PathViolation::CurrentDirectorySegment)
        );
        assert_eq!(
            path_violation("a/../b"),
            Some(PathViolation::ParentDirectorySegment)
        );
    }
}
