use crate::*;
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct PackageError {
    pub path: String,
    pub diagnostic: Box<nmlt_core::Diagnostic>,
    pub expected: Option<Box<Type>>,
    pub actual: Option<Box<Type>>,
    pub related: Vec<(String, Span, String)>,
}

/// Compile the entry's import closure. The reader receives validated sibling
/// basenames, once per source, in deterministic order. It must bound its I/O;
/// the compiler separately bounds all returned source text before parsing.
pub fn compile_package(
    entry: &str,
    mut read: impl FnMut(&str) -> Result<String, String>,
) -> Result<Program, PackageError> {
    let at = Location {
        source: 0,
        start: 0,
        end: 0,
    };
    if !valid_file(entry) {
        return Err(located(
            entry,
            error(at, "entry must be a portable identifier followed by .nmlt"),
        ));
    }
    let mut loader = Loader {
        read: &mut read,
        files: vec![],
        identities: vec![],
        seen: BTreeMap::new(),
        active: BTreeSet::new(),
        heights: BTreeMap::new(),
        bytes: 0,
    };
    loader.visit(entry, entry, at, 0)?;
    let libraries: BTreeSet<String> = loader
        .identities
        .iter()
        .skip(1)
        .map(|s| s.path.trim_end_matches(".nmlt").to_owned())
        .collect();
    let mut combined = RawSource {
        functions: vec![],
        records: vec![],
        imports: vec![],
        modules: vec![],
    };
    let mut scopes = vec![];
    for (index, mut file) in loader.files.into_iter().enumerate() {
        let path = &loader.identities[index].path;
        let root = if index == 0 {
            String::new()
        } else {
            path.trim_end_matches(".nmlt").to_owned()
        };
        let imports: BTreeSet<_> = file.imports.iter().map(|(n, _)| n.clone()).collect();
        // Root declarations cannot occupy an implicit library namespace, even
        // when that library was loaded by a transitive import.
        let aliases = if index == 0 { &libraries } else { &imports };
        for (name, span) in file
            .modules
            .iter()
            .map(|(n, s)| (n, s))
            .chain(file.records.iter().map(|r| (&r.name, &r.span)))
            .chain(file.functions.iter().map(|f| (&f.name, &f.span)))
        {
            let first = name.split('.').next().unwrap();
            if aliases.contains(first) {
                return Err(located(
                    path,
                    error(
                        *span,
                        format!("declaration `{name}` conflicts with package module `{first}`"),
                    ),
                ));
            }
        }
        for f in &mut file.functions {
            f.name = parse::qualify(&root, &f.name);
            f.module = parse::qualify(&root, &f.module)
                .trim_end_matches('.')
                .to_owned();
            f.span.source = index;
            locate_expr(&mut f.body, index);
        }
        for r in &mut file.records {
            r.name = parse::qualify(&root, &r.name);
            r.module = parse::qualify(&root, &r.module)
                .trim_end_matches('.')
                .to_owned();
            r.span.source = index;
        }
        scopes.push(Scope {
            root,
            imports,
            libraries: libraries.clone(),
        });
        combined.functions.extend(file.functions);
        combined.records.extend(file.records);
    }
    check::compile_raw(combined, &scopes, loader.identities.clone()).map_err(|d| {
        let related = d
            .related
            .iter()
            .map(|(location, message)| {
                (
                    loader.identities[location.source].path.clone(),
                    (*location).into(),
                    message.clone(),
                )
            })
            .collect();
        let mut error = located(&loader.identities[d.source].path, d);
        error.related = related;
        error
    })
}

fn located(path: &str, diagnostic: Diagnostic) -> PackageError {
    PackageError {
        path: path.into(),
        diagnostic: Box::new(diagnostic.diagnostic),
        expected: diagnostic.expected,
        actual: diagnostic.actual,
        related: vec![],
    }
}

fn valid_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    name.len() <= 64
        && bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && ![
            "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
            "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
        ]
        .contains(&name.to_ascii_lowercase().as_str())
}
fn valid_file(path: &str) -> bool {
    path.strip_suffix(".nmlt").is_some_and(valid_name)
}
fn valid_import(name: &str) -> bool {
    valid_name(name)
        && ![
            "fn",
            "module",
            "import",
            "record",
            "new",
            "fold",
            "length",
            "get",
            "job_square",
            "let",
            "if",
            "else",
            "match",
            "true",
            "false",
            "and",
            "or",
            "not",
            "Ok",
            "Err",
            "Bool",
            "Int",
            "Text",
            "Outcome",
            "List",
        ]
        .contains(&name)
}

struct Loader<'a, F> {
    read: &'a mut F,
    files: Vec<RawSource>,
    identities: Vec<SourceIdentity>,
    seen: BTreeMap<String, String>,
    active: BTreeSet<String>,
    heights: BTreeMap<String, usize>,
    bytes: usize,
}
impl<F: FnMut(&str) -> Result<String, String>> Loader<'_, F> {
    fn visit(
        &mut self,
        path: &str,
        importer: &str,
        at: Location,
        depth: usize,
    ) -> Result<(), PackageError> {
        let fail = |message| located(importer, error(at, message));
        if self.active.contains(path) {
            return Err(fail(format!("import cycle reaches `{path}`")));
        }
        if depth > 8 {
            return Err(fail("import depth exceeds eight".into()));
        }
        let folded = path.to_ascii_lowercase();
        if let Some(existing) = self.seen.get(&folded) {
            return if existing != path {
                Err(fail(format!(
                    "case-conflicting source names `{existing}` and `{path}`"
                )))
            } else if depth + self.heights[path] > 8 {
                Err(fail("import depth exceeds eight".into()))
            } else {
                Ok(())
            };
        }
        if self.files.len() == MAX_PACKAGE_FILES {
            return Err(fail("package exceeds 32 source files".into()));
        }
        let source =
            (self.read)(path).map_err(|e| fail(format!("could not load `{path}`: {e}")))?;
        if source.len() > MAX_SOURCE_BYTES {
            return Err(fail(format!("`{path}` exceeds 128 KiB")));
        }
        self.bytes += source.len();
        if self.bytes > MAX_PACKAGE_BYTES {
            return Err(fail("package exceeds 1 MiB of source".into()));
        }
        let file = parse::source(&source).map_err(|d| located(path, d))?;
        let mut imports = file.imports.clone();
        imports.sort_by(|a, b| a.0.cmp(&b.0));
        self.identities.push(SourceIdentity {
            path: path.into(),
            bytes: source.len(),
            source_sha256: digest(source.as_bytes()),
        });
        self.files.push(file);
        self.seen.insert(folded, path.into());
        self.active.insert(path.into());
        let mut height = 0;
        for (module, span) in imports {
            if !valid_import(&module) {
                return Err(located(
                    path,
                    error(
                        span,
                        "import needs a portable, non-reserved module identifier",
                    ),
                ));
            }
            let child = format!("{module}.nmlt");
            self.visit(&child, path, span, depth + 1)?;
            height = height.max(1 + self.heights[&child]);
        }
        self.heights.insert(path.into(), height);
        self.active.remove(path);
        Ok(())
    }
}

fn locate_expr(expr: &mut Expr, source: usize) {
    expr.span.source = source;
    match &mut expr.kind {
        ExprKind::Literal(_) | ExprKind::Name(_) => {}
        ExprKind::Call(_, args) | ExprKind::List(args) => {
            for a in args {
                locate_expr(a, source);
            }
        }
        ExprKind::Record(_, fields) => {
            for (_, e) in fields {
                locate_expr(e, source);
            }
        }
        ExprKind::Let(_, _, a, b) | ExprKind::Binary(_, a, b) => {
            locate_expr(a, source);
            locate_expr(b, source);
        }
        ExprKind::If(a, b, c) | ExprKind::Match(a, _, b, _, c) | ExprKind::Fold(a, b, _, _, c) => {
            locate_expr(a, source);
            locate_expr(b, source);
            locate_expr(c, source);
        }
        ExprKind::Unary(_, a) | ExprKind::Field(a, _) => locate_expr(a, source),
    }
}
