//! Integrated exact-source to kernel-checked-program orchestration.

#![forbid(unsafe_code)]

use std::fmt;

use nmlt_elaborate::elaborate;
use nmlt_hir::{ProjectedModule, project_source_module, resolve_modules};
use nmlt_kernel::{CheckedProgram, RawCertificate, check};

/// One exact module in the closed compilation source set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceModule {
    logical_module: String,
    repository_path: String,
    exact_bytes: Vec<u8>,
}

impl SourceModule {
    #[must_use]
    pub fn new(
        logical_module: impl Into<String>,
        repository_path: impl Into<String>,
        exact_bytes: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            logical_module: logical_module.into(),
            repository_path: repository_path.into(),
            exact_bytes: exact_bytes.into(),
        }
    }
}

/// Stable integrated-pipeline failure stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompileStage {
    Projection,
    Resolution,
    Elaboration,
    Kernel,
}

impl CompileStage {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Projection => "PROJECTION",
            Self::Resolution => "RESOLUTION",
            Self::Elaboration => "ELABORATION",
            Self::Kernel => "KERNEL",
        }
    }
}

/// A fail-closed error from one named integrated-pipeline stage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileDiagnostic {
    stage: CompileStage,
    detail: String,
}

impl CompileDiagnostic {
    #[must_use]
    pub const fn stage(&self) -> CompileStage {
        self.stage
    }

    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    fn new(stage: CompileStage, detail: impl Into<String>) -> Self {
        Self {
            stage,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for CompileDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "NMLT_COMPILE_{}: {}",
            self.stage.as_str(),
            self.detail
        )
    }
}

impl std::error::Error for CompileDiagnostic {}

/// Compile a complete, closed exact source set into a kernel-issued program.
pub fn compile_source_set(
    sources: impl IntoIterator<Item = SourceModule>,
) -> Result<CheckedProgram, CompileDiagnostic> {
    let projected = sources
        .into_iter()
        .map(|source| {
            project_source_module(
                source.logical_module,
                source.repository_path,
                source.exact_bytes,
            )
        })
        .collect::<Vec<ProjectedModule>>();
    let projection_errors = projected
        .iter()
        .flat_map(|module| {
            module
                .projection_issues()
                .iter()
                .map(move |issue| format!("{}: {}", module.repository_path(), issue.message))
        })
        .collect::<Vec<_>>();
    if !projection_errors.is_empty() {
        return Err(CompileDiagnostic::new(
            CompileStage::Projection,
            projection_errors.join("; "),
        ));
    }
    let hir = resolve_modules(projected)
        .map_err(|error| CompileDiagnostic::new(CompileStage::Resolution, error.to_string()))?;
    let artifact = elaborate(&hir)
        .map_err(|error| CompileDiagnostic::new(CompileStage::Elaboration, error.to_string()))?;
    let raw = RawCertificate::from_artifact(&artifact);
    check(&hir, artifact.core_program(), &raw)
        .map_err(|error| CompileDiagnostic::new(CompileStage::Kernel, error.to_string()))
}

/// Compile one standalone source module.
pub fn compile_single(
    logical_module: impl Into<String>,
    repository_path: impl Into<String>,
    exact_bytes: impl Into<Vec<u8>>,
) -> Result<CheckedProgram, CompileDiagnostic> {
    compile_source_set([SourceModule::new(
        logical_module,
        repository_path,
        exact_bytes,
    )])
}
