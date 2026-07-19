use crate::{
    DeclarationInput, DeclarationKey, DefPath, DefPathSegment, DefPathViolation, ImportInput,
    LookupError, ModuleInput, NameReference, Namespace, PathViolation, ProjectionIssue,
    ProjectionIssueKind, ResolveError, ResourceDimension, SemanticRole, SourceEncodingViolation,
    SourceSetEntry, SourceSetId, SourceSpan,
};

use crate::resolver::resolve_module_inputs as resolve_modules;

const ZERO: SourceSpan = SourceSpan::new(0, 0);

fn module(name: &str) -> ModuleInput {
    ModuleInput::new(
        name,
        format!("src/{name}.nmlt"),
        format!("module {name} {{}}\n"),
    )
}

fn imported(mut input: ModuleInput, names: &[&str]) -> ModuleInput {
    input.imports = names
        .iter()
        .map(|name| ImportInput::new(*name, ZERO))
        .collect();
    input
}

fn top_level(namespace: Namespace, name: &str) -> DeclarationInput {
    DeclarationInput::new(DefPath::top_level(namespace, name), ZERO)
}

fn nested(
    owner_namespace: Namespace,
    owner: &str,
    namespace: Namespace,
    name: &str,
) -> DeclarationInput {
    DeclarationInput::new(
        DefPath::new([
            DefPathSegment::new(owner_namespace, owner),
            DefPathSegment::new(namespace, name),
        ]),
        ZERO,
    )
}

fn top_key(namespace: Namespace, name: &str) -> DeclarationKey {
    DeclarationKey::top_level(namespace, name)
}

#[test]
fn resolution_is_deterministic_under_input_import_and_declaration_permutation() {
    let mut a = module("A");
    a.declarations = vec![
        top_level(Namespace::Value, "z"),
        top_level(Namespace::Type, "T"),
    ];
    let mut b = module("B");
    b.declarations = vec![top_level(Namespace::System, "S")];
    let c = imported(module("C"), &["B", "A"]);

    let left = resolve_modules(vec![c.clone(), b.clone(), a.clone()]).unwrap();
    a.declarations.reverse();
    let c_reordered = imported(module("C"), &["A", "B"]);
    let right = resolve_modules(vec![a, c_reordered, b]).unwrap();

    assert_eq!(left, right);
    assert_eq!(left.dependency_order, ["A", "B", "C"]);
}

#[test]
fn the_source_set_is_closed_and_duplicate_free() {
    let missing = resolve_modules(vec![imported(module("A"), &["Missing"])]).unwrap_err();
    assert!(matches!(
        missing,
        ResolveError::MissingImport {
            logical_module,
            imported_module,
            ..
        } if logical_module == "A" && imported_module == "Missing"
    ));

    let duplicate_import =
        resolve_modules(vec![imported(module("A"), &["B", "B"]), module("B")]).unwrap_err();
    assert!(matches!(
        duplicate_import,
        ResolveError::DuplicateImport {
            logical_module,
            imported_module,
            spans,
        } if logical_module == "A" && imported_module == "B" && spans == [ZERO, ZERO]
    ));

    let mut second_a = module("A");
    second_a.repository_path = "other/A.nmlt".to_owned();
    assert!(matches!(
        resolve_modules(vec![module("A"), second_a]),
        Err(ResolveError::DuplicateLogicalModule { logical_module, .. }) if logical_module == "A"
    ));

    let mut b = module("B");
    b.repository_path = "src/A.nmlt".to_owned();
    assert!(matches!(
        resolve_modules(vec![module("A"), b]),
        Err(ResolveError::DuplicateRepositoryPath { repository_path, .. })
            if repository_path == "src/A.nmlt"
    ));
}

#[test]
fn cycle_witness_is_deterministic_under_permutation() {
    let a = imported(module("A"), &["B"]);
    let b = imported(module("B"), &["C"]);
    let c = imported(module("C"), &["A"]);

    let first = resolve_modules(vec![c.clone(), a.clone(), b.clone()]).unwrap_err();
    let second = resolve_modules(vec![b, c, a]).unwrap_err();
    assert_eq!(first, second);
    assert_eq!(
        first,
        ResolveError::ImportCycle {
            cycle: vec!["A".into(), "B".into(), "C".into(), "A".into()]
        }
    );

    let self_cycle = resolve_modules(vec![imported(module("Selfish"), &["Selfish"])]).unwrap_err();
    assert_eq!(
        self_cycle,
        ResolveError::ImportCycle {
            cycle: vec!["Selfish".into(), "Selfish".into()]
        }
    );
}

#[test]
fn namespaces_and_full_typed_parent_paths_are_distinct() {
    let mut input = module("Defs");
    input.declarations = vec![
        top_level(Namespace::Type, "X"),
        top_level(Namespace::Value, "X"),
        top_level(Namespace::System, "Left"),
        nested(Namespace::System, "Left", Namespace::State, "x"),
        top_level(Namespace::System, "Right"),
        nested(Namespace::System, "Right", Namespace::State, "x"),
    ];
    let resolved = resolve_modules(vec![input.clone()]).unwrap();
    let definitions = &resolved.modules["Defs"].declarations;
    assert_eq!(definitions.len(), 6);

    let ids = definitions
        .values()
        .map(|definition| definition.id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), definitions.len());

    input
        .declarations
        .push(nested(Namespace::System, "Left", Namespace::State, "x"));
    assert!(matches!(
        resolve_modules(vec![input]),
        Err(ResolveError::DuplicateDefinition { key, .. })
            if key.path.segments.last().is_some_and(|segment| segment.name == "x")
    ));
}

#[test]
fn projection_encoding_path_and_span_violations_fail_closed() {
    let mut incomplete = module("A");
    incomplete.projection_issues.push(ProjectionIssue::new(
        ProjectionIssueKind::CoverageGap,
        "unprojected declaration",
        Some(ZERO),
    ));
    assert!(matches!(
        resolve_modules(vec![incomplete]),
        Err(ResolveError::IncompleteProjection { logical_module, .. }) if logical_module == "A"
    ));

    let dotted = ModuleInput::new("A.B", "src/A.nmlt", b"module A {}\n".to_vec());
    assert!(matches!(
        resolve_modules(vec![dotted]),
        Err(ResolveError::InvalidLogicalModule { logical_module }) if logical_module == "A.B"
    ));

    let absolute = ModuleInput::new("A", "/src/A.nmlt", b"module A {}\n".to_vec());
    assert!(matches!(
        resolve_modules(vec![absolute]),
        Err(ResolveError::InvalidRepositoryPath {
            violation: PathViolation::Absolute,
            ..
        })
    ));

    let invalid_utf8 = ModuleInput::new("A", "src/A.nmlt", vec![0xff]);
    assert!(matches!(
        resolve_modules(vec![invalid_utf8]),
        Err(ResolveError::InvalidSourceEncoding {
            violation: SourceEncodingViolation::InvalidUtf8,
            ..
        })
    ));

    let bom = ModuleInput::new("A", "src/A.nmlt", b"\xef\xbb\xbfmodule A {}\n".to_vec());
    assert!(matches!(
        resolve_modules(vec![bom]),
        Err(ResolveError::InvalidSourceEncoding {
            violation: SourceEncodingViolation::Utf8ByteOrderMark,
            ..
        })
    ));

    let mut boundary = ModuleInput::new("A", "src/A.nmlt", "x\u{e9}".as_bytes().to_vec());
    boundary
        .imports
        .push(ImportInput::new("B", SourceSpan::new(2, 2)));
    assert!(matches!(
        resolve_modules(vec![boundary]),
        Err(ResolveError::InvalidSpan { span, .. }) if span == SourceSpan::new(2, 2)
    ));

    let mut empty_path = module("A");
    empty_path
        .declarations
        .push(DeclarationInput::new(DefPath::new([]), ZERO));
    assert!(matches!(
        resolve_modules(vec![empty_path]),
        Err(ResolveError::InvalidDefinitionPath {
            violation: DefPathViolation::Empty,
            ..
        })
    ));
}

#[test]
fn typed_definition_paths_reject_impossible_m9_nesting() {
    let mut top_level_state = module("A");
    top_level_state
        .declarations
        .push(top_level(Namespace::State, "x"));
    assert!(matches!(
        resolve_modules(vec![top_level_state]),
        Err(ResolveError::InvalidDefinitionPath {
            violation: DefPathViolation::InvalidTopLevel {
                namespace: Namespace::State
            },
            ..
        })
    ));

    let mut wrong_parent = module("A");
    wrong_parent
        .declarations
        .push(nested(Namespace::System, "S", Namespace::Constructor, "C"));
    assert!(matches!(
        resolve_modules(vec![wrong_parent]),
        Err(ResolveError::InvalidDefinitionPath {
            violation: DefPathViolation::InvalidChild {
                parent: Namespace::System,
                child: Namespace::Constructor
            },
            ..
        })
    ));

    let mut too_deep = module("A");
    too_deep.declarations.push(DeclarationInput::new(
        DefPath::new([
            DefPathSegment::new(Namespace::System, "S"),
            DefPathSegment::new(Namespace::Action, "a"),
            DefPathSegment::new(Namespace::Value, "local"),
        ]),
        ZERO,
    ));
    assert!(matches!(
        resolve_modules(vec![too_deep]),
        Err(ResolveError::InvalidDefinitionPath {
            violation: DefPathViolation::TooManySegments,
            ..
        })
    ));

    let mut orphan = module("A");
    orphan.declarations.push(nested(
        Namespace::Type,
        "Missing",
        Namespace::Constructor,
        "One",
    ));
    assert!(matches!(
        resolve_modules(vec![orphan]),
        Err(ResolveError::MissingDefinitionParent { parent, .. })
            if parent == DefPath::top_level(Namespace::Type, "Missing")
    ));

    let mut valid = module("A");
    valid.declarations = vec![
        top_level(Namespace::Type, "E"),
        nested(Namespace::Type, "E", Namespace::Constructor, "One"),
        top_level(Namespace::System, "S"),
        nested(Namespace::System, "S", Namespace::Value, "limit"),
        nested(Namespace::System, "S", Namespace::Action, "step"),
        nested(Namespace::System, "S", Namespace::Observation, "Visible"),
    ];
    assert_eq!(
        resolve_modules(vec![valid]).unwrap().modules["A"]
            .declarations
            .len(),
        6
    );
}

#[test]
fn untrusted_input_limits_fail_before_resolution_allocation() {
    let too_many_modules = (0..257)
        .map(|index| module(&format!("M{index}")))
        .collect::<Vec<_>>();
    assert!(matches!(
        resolve_modules(too_many_modules),
        Err(ResolveError::ResourceLimit {
            dimension: ResourceDimension::Modules,
            maximum: 256,
            actual: 257,
        })
    ));

    let long_identifier = "A".repeat(256);
    assert!(matches!(
        resolve_modules(vec![ModuleInput::new(
            long_identifier,
            "a.nmlt",
            Vec::new()
        )]),
        Err(ResolveError::ResourceLimit {
            dimension: ResourceDimension::IdentifierBytes,
            maximum: 255,
            actual: 256,
        })
    ));

    let oversized_source = vec![b'x'; 4 * 1024 * 1024 + 1];
    assert!(matches!(
        resolve_modules(vec![ModuleInput::new("A", "a.nmlt", oversized_source)]),
        Err(ResolveError::ResourceLimit {
            dimension: ResourceDimension::SourceBytes,
            ..
        })
    ));

    let mut deep = module("A");
    deep.declarations.push(DeclarationInput::new(
        DefPath::new(
            (0..257).map(|index| DefPathSegment::new(Namespace::System, format!("S{index}"))),
        ),
        ZERO,
    ));
    assert!(matches!(
        resolve_modules(vec![deep]),
        Err(ResolveError::ResourceLimit {
            dimension: ResourceDimension::DefinitionPathDepth,
            maximum: 256,
            actual: 257,
        })
    ));
}

#[test]
fn rfc0004_and_module_map_identities_have_separate_responsibilities() {
    let bytes = b"module Placeholder {}\n".to_vec();
    let a = ModuleInput::new("A", "src/shared.nmlt", bytes.clone());
    let b = ModuleInput::new("B", "src/shared.nmlt", bytes.clone());
    let resolved_a = resolve_modules(vec![a]).unwrap();
    let resolved_b = resolve_modules(vec![b]).unwrap();

    assert_eq!(resolved_a.source_set_id, resolved_b.source_set_id);
    assert_ne!(resolved_a.module_map_id, resolved_b.module_map_id);
    assert_ne!(resolved_a.modules["A"].id, resolved_b.modules["B"].id);

    let mut moved = ModuleInput::new("A", "moved/shared.nmlt", bytes);
    moved.declarations.push(top_level(Namespace::Type, "T"));
    let mut original =
        ModuleInput::new("A", "src/shared.nmlt", b"module Placeholder {}\n".to_vec());
    original.declarations.push(top_level(Namespace::Type, "T"));
    let original = resolve_modules(vec![original]).unwrap();
    let moved = resolve_modules(vec![moved]).unwrap();

    assert_eq!(
        original.modules["A"].source_id,
        moved.modules["A"].source_id
    );
    assert_ne!(original.source_set_id, moved.source_set_id);
    assert_ne!(original.module_map_id, moved.module_map_id);
    assert_ne!(original.modules["A"].id, moved.modules["A"].id);
    assert_ne!(
        original.modules["A"].declarations[&top_key(Namespace::Type, "T")].id,
        moved.modules["A"].declarations[&top_key(Namespace::Type, "T")].id
    );
    let original_definition = &original.modules["A"].declarations[&top_key(Namespace::Type, "T")];
    let moved_definition = &moved.modules["A"].declarations[&top_key(Namespace::Type, "T")];
    assert_ne!(
        original_definition.node_id(&[SemanticRole::Initializer]),
        moved_definition.node_id(&[SemanticRole::Initializer])
    );

    let path_a = ModuleInput::new("A", "a.nmlt", b"first".to_vec());
    let path_b = ModuleInput::new("B", "b.nmlt", b"second".to_vec());
    let swapped_a = ModuleInput::new("A", "b.nmlt", b"second".to_vec());
    let swapped_b = ModuleInput::new("B", "a.nmlt", b"first".to_vec());
    let direct = resolve_modules(vec![path_a, path_b]).unwrap();
    let swapped = resolve_modules(vec![swapped_a, swapped_b]).unwrap();
    assert_eq!(direct.source_set_id, swapped.source_set_id);
    assert_ne!(direct.module_map_id, swapped.module_map_id);
}

#[test]
fn resolution_identity_binds_projected_metadata_but_not_spans() {
    let a = module("A");
    let b = module("B");
    let no_import = resolve_modules(vec![a.clone(), b.clone()]).unwrap();
    let with_import = resolve_modules(vec![imported(a, &["B"]), b]).unwrap();

    assert_eq!(no_import.source_set_id, with_import.source_set_id);
    assert_eq!(no_import.module_map_id, with_import.module_map_id);
    assert_eq!(no_import.modules["A"].id, with_import.modules["A"].id);
    assert_ne!(no_import.resolution_id, with_import.resolution_id);

    let mut first = module("Spans");
    first.declarations.push(DeclarationInput::new(
        DefPath::top_level(Namespace::Value, "x"),
        SourceSpan::new(0, 1),
    ));
    let mut second = first.clone();
    second.declarations[0].span = SourceSpan::new(1, 2);
    let first = resolve_modules(vec![first]).unwrap();
    let second = resolve_modules(vec![second]).unwrap();
    assert_eq!(first.resolution_id, second.resolution_id);
    assert_eq!(
        first.modules["Spans"].declarations[&top_key(Namespace::Value, "x")].id,
        second.modules["Spans"].declarations[&top_key(Namespace::Value, "x")].id
    );
    let first_definition = &first.modules["Spans"].declarations[&top_key(Namespace::Value, "x")];
    let second_definition = &second.modules["Spans"].declarations[&top_key(Namespace::Value, "x")];
    assert_eq!(
        first_definition.node_id(&[SemanticRole::Initializer]),
        second_definition.node_id(&[SemanticRole::Initializer])
    );
}

#[test]
fn unqualified_lookup_rejects_ambiguity_and_strict_shadowing() {
    let key = top_key(Namespace::Value, "x");
    let mut b = module("B");
    b.declarations.push(top_level(Namespace::Value, "x"));
    let mut c = module("C");
    c.declarations.push(top_level(Namespace::Value, "x"));
    let a = imported(module("A"), &["B", "C"]);
    let program = resolve_modules(vec![a, b.clone(), c]).unwrap();
    let reference = NameReference::unqualified(key.clone(), ZERO);
    assert!(matches!(
        program.resolve_name("A", &reference),
        Err(LookupError::AmbiguousDefinition { candidates, .. })
            if candidates.iter().map(|candidate| candidate.logical_module.as_str()).collect::<Vec<_>>() == ["B", "C"]
    ));

    let mut local = imported(module("Local"), &["B"]);
    local.declarations.push(top_level(Namespace::Value, "x"));
    let program = resolve_modules(vec![local, b]).unwrap();
    assert!(matches!(
        program.resolve_name("Local", &reference),
        Err(LookupError::StrictShadowing { imported, .. }) if imported.len() == 1
    ));
}

#[test]
fn lookup_does_not_leak_transitive_imports() {
    let mut c = module("C");
    c.declarations.push(top_level(Namespace::Value, "x"));
    let b = imported(module("B"), &["C"]);
    let a = imported(module("A"), &["B"]);
    let program = resolve_modules(vec![a, b, c]).unwrap();
    let key = top_key(Namespace::Value, "x");

    assert!(matches!(
        program.resolve_name("A", &NameReference::unqualified(key.clone(), ZERO)),
        Err(LookupError::MissingDefinition { .. })
    ));
    assert!(matches!(
        program.resolve_name("A", &NameReference::qualified("C", key.clone(), ZERO)),
        Err(LookupError::ModuleNotVisible { requested_module, .. }) if requested_module == "C"
    ));

    let resolved = program
        .resolve_name("B", &NameReference::qualified("C", key, ZERO))
        .unwrap();
    assert_eq!(resolved.key, top_key(Namespace::Value, "x"));
}

#[test]
fn semantic_role_ids_use_only_frozen_registry_tags() {
    let mut input = module("A");
    input.declarations.push(top_level(Namespace::Value, "x"));
    let program = resolve_modules(vec![input]).unwrap();
    let definition = &program.modules["A"].declarations[&top_key(Namespace::Value, "x")];

    assert_ne!(
        definition.node_id(&[]),
        definition.node_id(&[SemanticRole::Initializer])
    );
    assert_eq!(
        definition.node_id(&[SemanticRole::Initializer]),
        definition.node_id(&[SemanticRole::Initializer])
    );
}

#[test]
fn source_set_identity_is_path_sorted_and_exact_byte_sensitive() {
    let lf = b"module A {}\n";
    let crlf = b"module A {}\r\n";
    let first = SourceSetId::from_entries(&[
        SourceSetEntry {
            repository_path: "b.nmlt",
            exact_bytes: lf,
        },
        SourceSetEntry {
            repository_path: "a.nmlt",
            exact_bytes: lf,
        },
    ])
    .unwrap();
    let permuted = SourceSetId::from_entries(&[
        SourceSetEntry {
            repository_path: "a.nmlt",
            exact_bytes: lf,
        },
        SourceSetEntry {
            repository_path: "b.nmlt",
            exact_bytes: lf,
        },
    ])
    .unwrap();
    let changed = SourceSetId::from_entries(&[
        SourceSetEntry {
            repository_path: "a.nmlt",
            exact_bytes: lf,
        },
        SourceSetEntry {
            repository_path: "b.nmlt",
            exact_bytes: crlf,
        },
    ])
    .unwrap();

    assert_eq!(first, permuted);
    assert_ne!(first, changed);
}
