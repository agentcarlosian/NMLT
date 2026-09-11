//! Discover source imports with the pinned Lean/Lake header parser.
use super::{
    Manifest, Result, Run, Source, decode_json, err, module_path, name, read_bounded, write_new,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Configuration {
    schema: String,
    source_roots: Vec<String>,
    target_module: String,
    target: String,
    permitted_axioms: Vec<String>,
}

impl Configuration {
    pub(super) fn validate(&self) -> Result<()> {
        if self.schema != "nmlt-lean-project-v2"
            || self.source_roots.is_empty()
            || self.source_roots.len() > 16
        {
            return Err("invalid discovery schema or source root count".into());
        }
        let mut seen = BTreeSet::new();
        for root in &self.source_roots {
            if !root_name(root) || !seen.insert(root.to_lowercase()) {
                return Err(
                    "source roots must be unique, normalized project-relative paths".into(),
                );
            }
        }
        self.normalized(vec![self.target_module.clone()]).validate()
    }

    fn normalized(&self, modules: Vec<String>) -> Manifest {
        Manifest {
            schema: "nmlt-lean-project-v1".into(),
            modules,
            target_module: self.target_module.clone(),
            target: self.target.clone(),
            permitted_axioms: self.permitted_axioms.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Import {
    module: String,
    is_exported: bool,
    is_meta: bool,
    import_all: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Header {
    imports: Vec<Import>,
    is_module: bool,
}

impl Header {
    fn validate(&self) -> Result<()> {
        if self.imports.len() > 256 || self.imports.iter().any(|i| !name(&i.module)) {
            return Err("unsupported import name or more than 256 header imports".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Module {
    module: String,
    source_root: String,
    header: Header,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Closure {
    schema: String,
    configuration: Configuration,
    modules: Vec<Module>,
    toolchain_modules: Vec<String>,
}

impl Closure {
    pub fn validate(&self, manifest: &Manifest, sources: &[Source]) -> Result<()> {
        self.configuration.validate()?;
        if self.schema != "nmlt-lean-import-closure-v1"
            || self.modules.len() != sources.len()
            || self.configuration.normalized(manifest.modules.clone()) != *manifest
            || self.modules.len() != manifest.modules.len()
        {
            return Err("discovered project configuration or source sequence changed".into());
        }
        let positions = manifest
            .modules
            .iter()
            .enumerate()
            .map(|(i, m)| (m.as_str(), i))
            .collect::<BTreeMap<_, _>>();
        let mut paths = BTreeSet::new();
        let mut toolchain = BTreeSet::new();
        for (index, ((module, source), expected)) in self
            .modules
            .iter()
            .zip(sources)
            .zip(&manifest.modules)
            .enumerate()
        {
            module.header.validate()?;
            if module.module != *expected
                || source.module != *expected
                || !self
                    .configuration
                    .source_roots
                    .contains(&module.source_root)
            {
                return Err("discovered module name or source root changed".into());
            }
            let path = Path::new(&module.source_root).join(module_path(expected));
            // Omit `.` so overlapping roots cannot name the same saved file twice.
            let path = path
                .components()
                .filter_map(|c| match c {
                    std::path::Component::Normal(s) => Some(s.to_string_lossy().to_lowercase()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("/");
            if !paths.insert(path) {
                return Err("one source file has multiple module identities".into());
            }
            for import in &module.header.imports {
                if let Some(dependency) = positions.get(import.module.as_str()) {
                    if *dependency >= index {
                        return Err(
                            "discovered imports are cyclic or outside dependency order".into()
                        );
                    }
                } else {
                    toolchain.insert(import.module.clone());
                }
            }
        }
        if self.toolchain_modules != toolchain.into_iter().collect::<Vec<_>>() {
            return Err("discovered toolchain import set changed".into());
        }
        let mut reachable = BTreeSet::new();
        let mut pending = vec![manifest.target_module.as_str()];
        while let Some(module) = pending.pop() {
            if !reachable.insert(module) {
                continue;
            }
            let Some(index) = positions.get(module) else {
                return Err("discovery target is absent from the source closure".into());
            };
            for import in &self.modules[*index].header.imports {
                if positions.contains_key(import.module.as_str()) {
                    pending.push(import.module.as_str());
                }
            }
        }
        if reachable.len() != self.modules.len() {
            return Err("discovered source closure contains an unreachable module".into());
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ParserOutput {
    imports: Vec<ParserEntry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ParserEntry {
    errors: Vec<String>,
    result: Option<Header>,
}

fn parser_output(bytes: &[u8]) -> Result<Header> {
    let output: ParserOutput = decode_json(bytes)?;
    let [entry]: [ParserEntry; 1] = output
        .imports
        .try_into()
        .map_err(|_| "Lean import parser must report exactly one source")?;
    // --deps-json reports parser errors in JSON even when the process exits zero.
    if !entry.errors.is_empty() {
        return Err(format!(
            "Lean import parser rejected the header: {}",
            entry.errors.join("\n")
        ));
    }
    let header = entry
        .result
        .ok_or("Lean import parser omitted its result")?;
    header.validate()?;
    Ok(header)
}

fn parse(source: &Source, run: &mut Run) -> Result<Header> {
    let path = run
        .directory
        .join("import-sources")
        .join(module_path(&source.module));
    fs::create_dir_all(path.parent().ok_or("source lacks a parent")?).map_err(err)?;
    write_new(&path, source.text.as_bytes())?;
    let mut command = run.command()?;
    command
        .args(["--threads=1", "--memory=768", "--deps-json"])
        .arg(&path);
    let output = run.execute(&format!("imports:{}", source.module), command)?;
    if !output.stderr.is_empty() {
        return Err("Lean import parser emitted unexpected diagnostics".into());
    }
    parser_output(&output.stdout)
}

fn library(run: &Run) -> Result<PathBuf> {
    Ok(run
        .executable
        .parent()
        .and_then(Path::parent)
        .ok_or("Lean installation has no root")?
        .join("lib/lean"))
}

fn builtin(library: &Path, module: &str) -> Result<bool> {
    let path = library.join(module_path(module)).with_extension("olean");
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(err(error)),
    }
}

/// Resolve a regular local file/directory, rejecting links and casing aliases.
/// `None` means the requested relative path is absent in this root.
fn ordinary(root: &Path, relative: &Path, directory: bool) -> Result<Option<PathBuf>> {
    let mut path = root.to_path_buf();
    for component in relative.components() {
        let part = match component {
            std::path::Component::CurDir => continue,
            std::path::Component::Normal(part) => part,
            _ => return Err("source path escapes its selected root".into()),
        };
        let entries = fs::read_dir(&path)
            .map_err(err)?
            .take(4097)
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(err)?;
        if entries.len() > 4096 {
            return Err("source directory exceeds 4096 entries".into());
        }
        if !entries.iter().any(|e| e.file_name() == part) {
            if entries.iter().any(|e| {
                e.file_name().to_string_lossy().to_lowercase()
                    == part.to_string_lossy().to_lowercase()
            }) {
                return Err("source path casing differs from its module/root name".into());
            }
            return Ok(None);
        }
        path.push(part);
        if linked(&fs::symlink_metadata(&path).map_err(err)?) {
            return Err("source paths and roots must not contain links".into());
        }
    }
    let canonical = path.canonicalize().map_err(err)?;
    let metadata = fs::metadata(&canonical).map_err(err)?;
    if !canonical.starts_with(root)
        || (directory && !metadata.is_dir())
        || (!directory && !metadata.is_file())
    {
        return Err("source path escapes its root or has the wrong file type".into());
    }
    Ok(Some(canonical))
}

fn linked(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Include junctions and other reparse points, not just symbolic links.
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

fn root_name(root: &str) -> bool {
    root == "."
        || (!root.is_empty()
            && root.len() <= 1024
            && !root
                .chars()
                .any(|c| c.is_control() || c == '\\' || c == ':')
            && root.split('/').count() <= 16
            && root
                .split('/')
                .all(|p| !p.is_empty() && p != "." && p != ".."))
}

struct Root {
    name: String,
    path: PathBuf,
}
struct Capture<'a> {
    roots: Vec<Root>,
    library: PathBuf,
    run: &'a mut Run,
    started: BTreeSet<String>,
    active: BTreeSet<String>,
    finished: BTreeSet<String>,
    files: BTreeSet<PathBuf>,
    sources: Vec<Source>,
    modules: Vec<Module>,
    toolchain: BTreeSet<String>,
    bytes: usize,
}

impl Capture<'_> {
    fn source(&self, module: &str) -> Result<Option<(String, PathBuf)>> {
        let mut found = None;
        for root in &self.roots {
            if let Some(path) = ordinary(&root.path, &module_path(module), false)? {
                if found.is_some() {
                    return Err(format!(
                        "ambiguous source module {module}: found in multiple roots"
                    ));
                }
                found = Some((root.name.clone(), path));
            }
        }
        Ok(found)
    }

    fn visit(&mut self, module: &str, required_source: bool) -> Result<()> {
        if self.active.contains(module) {
            return Err(format!("cyclic source import involving {module}"));
        }
        if self.finished.contains(module) || self.toolchain.contains(module) {
            return Ok(());
        }
        let local = self.source(module)?;
        let builtin = builtin(&self.library, module)?;
        if builtin && local.is_some() {
            return Err(format!(
                "local source shadows pinned toolchain module {module}"
            ));
        }
        let Some((source_root, path)) = local else {
            if builtin && !required_source {
                self.toolchain.insert(module.into());
                return Ok(());
            }
            return Err(format!(
                "unresolved source import {module} in the selected roots/toolchain"
            ));
        };
        // Apply the original local module/name policy before writing any copied path.
        Manifest {
            schema: "nmlt-lean-project-v1".into(),
            modules: vec![module.into()],
            target_module: module.into(),
            target: "NMLTDiscovery.placeholder".into(),
            permitted_axioms: vec![],
        }
        .validate()?;
        if self.started.len() >= 64 || !self.started.insert(module.to_lowercase()) {
            return Err("discovery exceeds 64 modules or contains casing aliases".into());
        }
        if !self.files.insert(path.clone()) {
            return Err("one source file has multiple module identities".into());
        }
        let text = String::from_utf8(read_bounded(&path, super::MAX_SOURCE)?).map_err(err)?;
        self.bytes += text.len();
        if self.bytes > 16 * 1024 * 1024 {
            return Err("Lean source closure exceeds 16 MiB".into());
        }
        let source = Source {
            module: module.into(),
            text,
        };
        let header = parse(&source, self.run)?;
        self.active.insert(module.into());
        for import in &header.imports {
            self.visit(&import.module, false)?;
        }
        self.active.remove(module);
        self.finished.insert(module.into());
        self.sources.push(source);
        self.modules.push(Module {
            module: module.into(),
            source_root,
            header,
        });
        Ok(())
    }
}

pub(super) fn capture(
    project: &Path,
    configuration: Configuration,
    run: &mut Run,
) -> Result<(Manifest, Vec<Source>, Closure)> {
    configuration.validate()?;
    let mut roots = vec![];
    let mut seen = BTreeSet::new();
    for name in &configuration.source_roots {
        let path = ordinary(project, Path::new(name), true)?
            .ok_or_else(|| format!("source root does not exist: {name}"))?;
        if !seen.insert(path.clone()) {
            return Err("source roots resolve to the same directory".into());
        }
        roots.push(Root {
            name: name.clone(),
            path,
        });
    }
    let mut capture = Capture {
        roots,
        library: library(run)?,
        run,
        started: BTreeSet::new(),
        active: BTreeSet::new(),
        finished: BTreeSet::new(),
        files: BTreeSet::new(),
        sources: vec![],
        modules: vec![],
        toolchain: BTreeSet::new(),
        bytes: 0,
    };
    capture.visit(&configuration.target_module, true)?;
    let manifest =
        configuration.normalized(capture.sources.iter().map(|s| s.module.clone()).collect());
    manifest.validate()?;
    let closure = Closure {
        schema: "nmlt-lean-import-closure-v1".into(),
        configuration,
        modules: capture.modules,
        toolchain_modules: capture.toolchain.into_iter().collect(),
    };
    closure.validate(&manifest, &capture.sources)?;
    Ok((manifest, capture.sources, closure))
}

pub(super) fn verify(
    closure: &Closure,
    manifest: &Manifest,
    sources: &[Source],
    run: &mut Run,
) -> Result<()> {
    closure.validate(manifest, sources)?;
    let library = library(run)?;
    for module in &closure.toolchain_modules {
        if !builtin(&library, module)? {
            return Err(format!(
                "discovered toolchain import is unavailable: {module}"
            ));
        }
    }
    for (module, source) in closure.modules.iter().zip(sources) {
        if builtin(&library, &module.module)? {
            return Err("retained local module shadows the pinned toolchain".into());
        }
        if parse(source, run)? != module.header {
            return Err(format!(
                "Lean import header differs from the retained closure: {}",
                module.module
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roots_are_bounded_normalized_relative_paths() {
        for root in [".", "src", ".lake/packages/Support", "vendor/local library"] {
            assert!(root_name(root), "{root}");
        }
        for root in [
            "",
            "..",
            "../src",
            "src/../other",
            "/src",
            "C:/src",
            "src\\other",
            "./src",
            "src//other",
            "src/",
        ] {
            assert!(!root_name(root), "{root}");
        }
    }

    #[test]
    fn zero_exit_parser_errors_and_ambiguous_responses_are_rejected() {
        let header = parser_output(br#"{"imports":[{"errors":[],"result":{"imports":[{"module":"Init","isExported":true,"isMeta":true,"importAll":false}],"isModule":false}}]}"#).unwrap();
        assert_eq!(header.imports[0].module, "Init");
        for input in [
            br#"{"imports":[{"errors":["bad header"],"result":null}]}"#.as_slice(),
            br#"{"imports":[{"errors":[],"result":null}]}"#,
            br#"{"imports":[]}"#,
            br#"{"imports":[],"imports":[]}"#,
        ] {
            assert!(parser_output(input).is_err());
        }
    }
}
