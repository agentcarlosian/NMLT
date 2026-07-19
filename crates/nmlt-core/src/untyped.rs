//! Explicit boundary between the lossless surface tree and an untyped core.
//!
//! This projection removes trivia from declaration structure while retaining
//! exact source slices for types and expressions. It deliberately performs no
//! name resolution, type checking, effect checking, or behavioral reasoning.
//! A projection with issues is a partial inspection artifact and must not be
//! passed to semantic elaboration.

use std::collections::BTreeMap;

use crate::{Diagnostic, GreenElement, GreenNode, Span, SyntaxKind, SyntaxParse, TokenKind};

/// Exact UTF-8 source text paired with its half-open byte span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpannedText {
    pub text: String,
    pub span: Span,
}

/// An expression or type which has not been parsed into semantic operators.
///
/// The text is retained so a later, independently tested expression parser can
/// elaborate it. Nothing in this type implies that the term is well-scoped,
/// well-typed, total, or executable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawTerm {
    pub source: SpannedText,
}

/// Partial, syntax-directed projection of a source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedProjection {
    pub file: UntypedFile,
    pub issues: Vec<ProjectionIssue>,
}

impl UntypedProjection {
    /// True only when recovery, duplicate declarations, and structurally
    /// unsupported update targets were absent.
    ///
    /// This is *not* semantic acceptance. In particular, names in raw terms
    /// and update targets may still be undeclared.
    #[must_use]
    pub fn is_structurally_complete(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Untyped behavioral declarations plus surface-only declarations which the
/// narrow projection records but does not assign a core meaning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedFile {
    pub systems: Vec<UntypedSystem>,
    pub surface_only: Vec<UntypedSurfaceNode>,
}

/// A surface construct retained without a semantic interpretation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedSurfaceNode {
    pub kind: SyntaxKind,
    pub source: SpannedText,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedSystem {
    pub name: Option<SpannedText>,
    pub span: Span,
    pub members: Vec<UntypedMember>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingKind {
    Const,
    Input,
    State,
    Capability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedBinding {
    pub kind: BindingKind,
    pub name: Option<SpannedText>,
    pub declared_type: Option<RawTerm>,
    pub initializer: Option<RawTerm>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedPort {
    pub direction: Option<SpannedText>,
    pub name: Option<SpannedText>,
    pub declared_type: Option<RawTerm>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedParameter {
    pub name: Option<SpannedText>,
    pub declared_type: Option<RawTerm>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedAction {
    pub name: Option<SpannedText>,
    pub parameters: Vec<UntypedParameter>,
    pub grade: Option<SpannedText>,
    pub statements: Vec<UntypedStatement>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropertyKind {
    Safety,
    Temporal,
    Resource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedProperty {
    pub kind: PropertyKind,
    pub name: Option<SpannedText>,
    pub expression: Option<RawTerm>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationKind {
    Observe,
    Hide,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedObservation {
    pub kind: ObservationKind,
    pub expression: Option<RawTerm>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UntypedMember {
    Binding(UntypedBinding),
    Port(UntypedPort),
    Action(UntypedAction),
    Property(UntypedProperty),
    Observation(UntypedObservation),
    SurfaceOnly(UntypedSurfaceNode),
    Error(UntypedErrorNode),
}

impl UntypedMember {
    fn declared_name(&self) -> Option<&SpannedText> {
        match self {
            Self::Binding(binding) => binding.name.as_ref(),
            Self::Port(port) => port.name.as_ref(),
            Self::Action(action) => action.name.as_ref(),
            Self::Property(property) => property.name.as_ref(),
            Self::Observation(_) | Self::SurfaceOnly(_) | Self::Error(_) => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UntypedStatement {
    Require(RawTerm),
    Update {
        target: UntypedUpdateTarget,
        value: RawTerm,
        span: Span,
    },
    Emit(RawTerm),
    Consume(RawTerm),
    Error(UntypedErrorNode),
}

/// A syntactic location is an identifier followed by zero or more field or
/// index selectors. Whether its root denotes state is intentionally deferred.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UntypedUpdateTarget {
    Location {
        root: SpannedText,
        source: SpannedText,
    },
    Unsupported(SpannedText),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedErrorNode {
    pub source: SpannedText,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionIssue {
    pub kind: ProjectionIssueKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionIssueKind {
    SyntaxDiagnostic { code: &'static str },
    MissingDiagnosticSpan { code: &'static str },
    RecoveryNode,
    DuplicateDeclaration { name: String, first_span: Span },
    InvalidUpdateTarget,
}

/// Project a lossless surface parse into the explicit untyped boundary.
///
/// The returned artifact remains available when recovery occurred so tools can
/// inspect it. Callers must check [`UntypedProjection::is_structurally_complete`]
/// before attempting later elaboration; even a complete projection carries no
/// name-resolution or type-correctness claim.
#[must_use]
pub fn project_untyped(parsed: &SyntaxParse) -> UntypedProjection {
    let mut projection = Projector {
        issues: diagnostic_issues(parsed.diagnostics()),
        systems: Vec::new(),
        surface_only: Vec::new(),
    };
    projection.project_top_level(parsed.root(), 0);
    UntypedProjection {
        file: UntypedFile {
            systems: projection.systems,
            surface_only: projection.surface_only,
        },
        issues: projection.issues,
    }
}

fn diagnostic_issues(diagnostics: &[Diagnostic]) -> Vec<ProjectionIssue> {
    diagnostics
        .iter()
        .map(|diagnostic| match diagnostic.span {
            Some(span) => ProjectionIssue {
                kind: ProjectionIssueKind::SyntaxDiagnostic {
                    code: diagnostic.code,
                },
                span,
            },
            None => ProjectionIssue {
                kind: ProjectionIssueKind::MissingDiagnosticSpan {
                    code: diagnostic.code,
                },
                span: Span::new(0, 0),
            },
        })
        .collect()
}

struct Projector {
    issues: Vec<ProjectionIssue>,
    systems: Vec<UntypedSystem>,
    surface_only: Vec<UntypedSurfaceNode>,
}

impl Projector {
    fn project_top_level(&mut self, node: &GreenNode, base: usize) {
        for child in direct_nodes(node, base) {
            match child.node.kind() {
                SyntaxKind::SystemDecl => {
                    let system = self.project_system(child.node, child.span.start);
                    self.systems.push(system);
                }
                SyntaxKind::ModuleDecl => {
                    self.surface_only.push(surface_node(child));
                    self.project_top_level(child.node, child.span.start);
                }
                SyntaxKind::Error => {
                    self.record_recovery(child.span);
                    self.surface_only.push(surface_node(child));
                }
                SyntaxKind::ImportDecl
                | SyntaxKind::DataDecl
                | SyntaxKind::TypeDecl
                | SyntaxKind::RecordDecl
                | SyntaxKind::FunctionDecl
                | SyntaxKind::EnumDecl => self.surface_only.push(surface_node(child)),
                _ => {}
            }
        }
    }

    fn project_system(&mut self, node: &GreenNode, base: usize) -> UntypedSystem {
        let name = identifier(node, base, 1);
        let span = Span::new(base, base + node.text_len());
        let mut members = Vec::new();
        if let Some(body) = direct_nodes(node, base)
            .into_iter()
            .find(|child| child.node.kind() == SyntaxKind::SystemBody)
        {
            for child in direct_nodes(body.node, body.span.start) {
                let member = match child.node.kind() {
                    SyntaxKind::ConstDecl => self.project_binding(child, BindingKind::Const),
                    SyntaxKind::InputDecl => self.project_binding(child, BindingKind::Input),
                    SyntaxKind::StateDecl => self.project_binding(child, BindingKind::State),
                    SyntaxKind::CapabilityDecl => {
                        self.project_binding(child, BindingKind::Capability)
                    }
                    SyntaxKind::PortDecl => self.project_port(child),
                    SyntaxKind::ActionDecl => self.project_action(child),
                    SyntaxKind::SafetyDecl => self.project_property(child, PropertyKind::Safety),
                    SyntaxKind::TemporalDecl => {
                        self.project_property(child, PropertyKind::Temporal)
                    }
                    SyntaxKind::ResourceDecl => {
                        self.project_property(child, PropertyKind::Resource)
                    }
                    SyntaxKind::ObserveDecl => {
                        self.project_observation(child, ObservationKind::Observe)
                    }
                    SyntaxKind::HideDecl => self.project_observation(child, ObservationKind::Hide),
                    SyntaxKind::Error => {
                        self.record_recovery(child.span);
                        UntypedMember::Error(error_node(child))
                    }
                    _ => UntypedMember::SurfaceOnly(surface_node(child)),
                };
                members.push(member);
            }
        }
        self.record_duplicate_members(&members);
        UntypedSystem {
            name,
            span,
            members,
        }
    }

    fn project_binding(&mut self, node: NodeAt<'_>, kind: BindingKind) -> UntypedMember {
        let children = direct_nodes(node.node, node.span.start);
        UntypedMember::Binding(UntypedBinding {
            kind,
            name: identifier(node.node, node.span.start, 1),
            declared_type: raw_child(&children, SyntaxKind::TypeExpr),
            initializer: raw_child(&children, SyntaxKind::Expr),
            span: node.span,
        })
    }

    fn project_port(&mut self, node: NodeAt<'_>) -> UntypedMember {
        let children = direct_nodes(node.node, node.span.start);
        UntypedMember::Port(UntypedPort {
            direction: identifier(node.node, node.span.start, 1),
            name: identifier(node.node, node.span.start, 2),
            declared_type: raw_child(&children, SyntaxKind::TypeExpr),
            span: node.span,
        })
    }

    fn project_action(&mut self, node: NodeAt<'_>) -> UntypedMember {
        let children = direct_nodes(node.node, node.span.start);
        let parameters = children
            .iter()
            .find(|child| child.node.kind() == SyntaxKind::ParameterList)
            .map_or_else(Vec::new, |list| {
                direct_nodes(list.node, list.span.start)
                    .into_iter()
                    .filter(|parameter| parameter.node.kind() == SyntaxKind::Parameter)
                    .map(|parameter| {
                        let parameter_children = direct_nodes(parameter.node, parameter.span.start);
                        UntypedParameter {
                            name: identifier(parameter.node, parameter.span.start, 0),
                            declared_type: raw_child(&parameter_children, SyntaxKind::TypeExpr),
                            span: parameter.span,
                        }
                    })
                    .collect()
            });
        let grade = children
            .iter()
            .find(|child| child.node.kind() == SyntaxKind::GradeClause)
            .map(|child| exact_node_source(*child));
        let statements = children
            .iter()
            .find(|child| child.node.kind() == SyntaxKind::ActionBody)
            .map_or_else(Vec::new, |body| {
                direct_nodes(body.node, body.span.start)
                    .into_iter()
                    .filter_map(|statement| self.project_statement(statement))
                    .collect()
            });
        UntypedMember::Action(UntypedAction {
            name: identifier(node.node, node.span.start, 1),
            parameters,
            grade,
            statements,
            span: node.span,
        })
    }

    fn project_statement(&mut self, node: NodeAt<'_>) -> Option<UntypedStatement> {
        let expressions = direct_nodes(node.node, node.span.start)
            .into_iter()
            .filter(|child| child.node.kind() == SyntaxKind::Expr)
            .filter_map(raw_term)
            .collect::<Vec<_>>();
        match node.node.kind() {
            SyntaxKind::RequireStmt => expressions
                .into_iter()
                .next()
                .map(UntypedStatement::Require),
            SyntaxKind::EmitStmt => expressions.into_iter().next().map(UntypedStatement::Emit),
            SyntaxKind::ConsumeStmt => expressions
                .into_iter()
                .next()
                .map(UntypedStatement::Consume),
            SyntaxKind::UpdateStmt => {
                let mut expressions = expressions.into_iter();
                let target = expressions.next();
                let value = expressions.next();
                match (target, value) {
                    (Some(target), Some(value)) => {
                        let target = self.project_update_target(target);
                        Some(UntypedStatement::Update {
                            target,
                            value,
                            span: node.span,
                        })
                    }
                    _ => {
                        self.record_recovery(node.span);
                        Some(UntypedStatement::Error(error_node(node)))
                    }
                }
            }
            SyntaxKind::Error => {
                self.record_recovery(node.span);
                Some(UntypedStatement::Error(error_node(node)))
            }
            _ => None,
        }
    }

    fn project_update_target(&mut self, target: RawTerm) -> UntypedUpdateTarget {
        let source = &target.source;
        let lexed = crate::lex_source(&source.text);
        let significant = lexed
            .tokens
            .iter()
            .filter(|token| !token.kind.is_trivia())
            .copied()
            .collect::<Vec<_>>();
        if valid_location_tokens(&significant, &source.text) {
            let root_token = significant[0];
            UntypedUpdateTarget::Location {
                root: SpannedText {
                    text: root_token.text(&source.text).to_owned(),
                    span: Span::new(
                        source.span.start + root_token.span.start,
                        source.span.start + root_token.span.end,
                    ),
                },
                source: source.clone(),
            }
        } else {
            self.issues.push(ProjectionIssue {
                kind: ProjectionIssueKind::InvalidUpdateTarget,
                span: source.span,
            });
            UntypedUpdateTarget::Unsupported(source.clone())
        }
    }

    fn project_property(&mut self, node: NodeAt<'_>, kind: PropertyKind) -> UntypedMember {
        let children = direct_nodes(node.node, node.span.start);
        UntypedMember::Property(UntypedProperty {
            kind,
            name: identifier(node.node, node.span.start, 1),
            expression: raw_child(&children, SyntaxKind::Expr),
            span: node.span,
        })
    }

    fn project_observation(&mut self, node: NodeAt<'_>, kind: ObservationKind) -> UntypedMember {
        let children = direct_nodes(node.node, node.span.start);
        UntypedMember::Observation(UntypedObservation {
            kind,
            expression: raw_child(&children, SyntaxKind::Expr),
            span: node.span,
        })
    }

    fn record_duplicate_members(&mut self, members: &[UntypedMember]) {
        let mut first_by_name = BTreeMap::<String, Span>::new();
        for name in members.iter().filter_map(UntypedMember::declared_name) {
            if let Some(first_span) = first_by_name.get(&name.text).copied() {
                self.issues.push(ProjectionIssue {
                    kind: ProjectionIssueKind::DuplicateDeclaration {
                        name: name.text.clone(),
                        first_span,
                    },
                    span: name.span,
                });
            } else {
                first_by_name.insert(name.text.clone(), name.span);
            }
        }
    }

    fn record_recovery(&mut self, span: Span) {
        let issue = ProjectionIssue {
            kind: ProjectionIssueKind::RecoveryNode,
            span,
        };
        if !self.issues.contains(&issue) {
            self.issues.push(issue);
        }
    }
}

#[derive(Clone, Copy)]
struct NodeAt<'tree> {
    node: &'tree GreenNode,
    span: Span,
}

fn direct_nodes(node: &GreenNode, base: usize) -> Vec<NodeAt<'_>> {
    let mut offset = base;
    let mut nodes = Vec::new();
    for child in node.children() {
        if let GreenElement::Node(child_node) = child {
            nodes.push(NodeAt {
                node: child_node,
                span: Span::new(offset, offset + child_node.text_len()),
            });
        }
        offset += child.text_len();
    }
    nodes
}

fn identifier(node: &GreenNode, base: usize, index: usize) -> Option<SpannedText> {
    node.tokens_with_spans()
        .into_iter()
        .filter(|token| token.token.kind() == TokenKind::Identifier)
        .nth(index)
        .map(|token| SpannedText {
            text: token.token.text().to_owned(),
            span: Span::new(base + token.span.start, base + token.span.end),
        })
}

fn raw_child(children: &[NodeAt<'_>], kind: SyntaxKind) -> Option<RawTerm> {
    children
        .iter()
        .find(|child| child.node.kind() == kind)
        .and_then(|child| raw_term(*child))
}

fn raw_term(node: NodeAt<'_>) -> Option<RawTerm> {
    let tokens = node.node.tokens_with_spans();
    let first = tokens
        .iter()
        .find(|token| !token.token.kind().is_trivia())?;
    let last = tokens
        .iter()
        .rev()
        .find(|token| !token.token.kind().is_trivia())?;
    let reconstructed = node.node.reconstruct();
    Some(RawTerm {
        source: SpannedText {
            text: reconstructed[first.span.start..last.span.end].to_owned(),
            span: Span::new(
                node.span.start + first.span.start,
                node.span.start + last.span.end,
            ),
        },
    })
}

fn exact_node_source(node: NodeAt<'_>) -> SpannedText {
    SpannedText {
        text: node.node.reconstruct(),
        span: node.span,
    }
}

fn surface_node(node: NodeAt<'_>) -> UntypedSurfaceNode {
    UntypedSurfaceNode {
        kind: node.node.kind(),
        source: exact_node_source(node),
    }
}

fn error_node(node: NodeAt<'_>) -> UntypedErrorNode {
    UntypedErrorNode {
        source: exact_node_source(node),
    }
}

fn valid_location_tokens(tokens: &[crate::Token], source: &str) -> bool {
    if tokens
        .first()
        .is_none_or(|token| token.kind != TokenKind::Identifier)
    {
        return false;
    }
    let mut index = 1;
    while index < tokens.len() {
        let token = tokens[index];
        if token.kind == TokenKind::Punctuation && token.text(source) == "." {
            index += 1;
            if tokens
                .get(index)
                .is_none_or(|field| field.kind != TokenKind::Identifier)
            {
                return false;
            }
            index += 1;
            continue;
        }
        if token.kind == TokenKind::LeftBracket {
            let mut depth = 1_usize;
            let mut has_selector = false;
            index += 1;
            while index < tokens.len() && depth > 0 {
                match tokens[index].kind {
                    TokenKind::LeftBracket => depth += 1,
                    TokenKind::RightBracket => depth -= 1,
                    _ if depth == 1 => has_selector = true,
                    _ => {}
                }
                index += 1;
            }
            if depth != 0 || !has_selector {
                return false;
            }
            continue;
        }
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::parse_cst;

    use super::{
        ProjectionIssueKind, UntypedMember, UntypedStatement, UntypedUpdateTarget, project_untyped,
    };

    #[test]
    fn projects_explicit_behavior_structure_without_semantic_claims() {
        let source = concat!(
            "system Counter {\n",
            "  state count: Nat = 0\n",
            "  action increment(by: Nat) { require by > 0; set count = count + by }\n",
            "  safety Nonnegative = always(count >= 0)\n",
            "  observe count\n",
            "}\n",
        );
        let parsed = parse_cst(source);
        let projection = project_untyped(&parsed);
        assert!(projection.is_structurally_complete());
        assert_eq!(projection.file.systems.len(), 1);
        assert_eq!(
            projection.file.systems[0].name.as_ref().unwrap().text,
            "Counter"
        );
        let action = projection.file.systems[0]
            .members
            .iter()
            .find_map(|member| match member {
                UntypedMember::Action(action) => Some(action),
                _ => None,
            })
            .unwrap();
        assert_eq!(action.parameters[0].name.as_ref().unwrap().text, "by");
        let UntypedStatement::Update { target, value, .. } = &action.statements[1] else {
            panic!("second statement should be an explicit update")
        };
        assert!(
            matches!(target, UntypedUpdateTarget::Location { root, .. } if root.text == "count")
        );
        assert_eq!(value.source.text, "count + by");
        assert_eq!(
            &source[value.source.span.start..value.source.span.end],
            value.source.text
        );
    }

    #[test]
    fn marks_recovered_syntax_as_partial() {
        let parsed = parse_cst("system S { action go { mystery x; set x = 1 } }");
        let projection = project_untyped(&parsed);
        assert!(!projection.is_structurally_complete());
        assert!(projection.issues.iter().any(|issue| {
            matches!(
                issue.kind,
                ProjectionIssueKind::SyntaxDiagnostic { code: "NMLT2009" }
            )
        }));
        assert!(
            projection
                .issues
                .iter()
                .any(|issue| { matches!(issue.kind, ProjectionIssueKind::RecoveryNode) })
        );
    }

    #[test]
    fn rejects_expression_shaped_update_targets_at_projection_boundary() {
        let parsed = parse_cst("system S { state x: Nat = 0\n action go { set x + y = 1 } }");
        assert!(parsed.diagnostics().is_empty());
        let projection = project_untyped(&parsed);
        assert!(!projection.is_structurally_complete());
        assert!(
            projection
                .issues
                .iter()
                .any(|issue| { matches!(issue.kind, ProjectionIssueKind::InvalidUpdateTarget) })
        );
    }

    #[test]
    fn leaves_undeclared_location_roots_for_name_resolution() {
        let parsed = parse_cst("system S { action go { set missing = 1 } }");
        let projection = project_untyped(&parsed);
        assert!(projection.is_structurally_complete());
        let UntypedMember::Action(action) = &projection.file.systems[0].members[0] else {
            panic!("expected action")
        };
        let UntypedStatement::Update { target, .. } = &action.statements[0] else {
            panic!("expected update")
        };
        assert!(
            matches!(target, UntypedUpdateTarget::Location { root, .. } if root.text == "missing")
        );
    }
}
