//! Explicit boundary between the lossless surface tree and an untyped core.
//!
//! This projection removes trivia from declaration structure while retaining
//! exact source slices for types and expressions. It deliberately performs no
//! name resolution, type checking, effect checking, or behavioral reasoning.
//! A projection with issues is a partial inspection artifact and must not be
//! passed to semantic elaboration.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{Diagnostic, GreenElement, GreenNode, Span, SyntaxKind, SyntaxParse, TokenKind};

/// Exact UTF-8 source text paired with its half-open byte span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpannedText {
    pub text: String,
    pub span: Span,
}

/// Exact CST origin of one projected semantic node.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SurfaceOrigin {
    pub kind: SyntaxKind,
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
    pub origin: SurfaceOrigin,
}

/// Partial, syntax-directed projection of a source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedProjection {
    pub file: UntypedFile,
    pub issues: Vec<ProjectionIssue>,
    pub coverage: ProjectionCoverage,
}

impl UntypedProjection {
    /// True only when recovery, duplicate declarations, and structurally
    /// unsupported update targets were absent.
    ///
    /// This is *not* semantic acceptance. In particular, names in raw terms
    /// and update targets may still be undeclared.
    #[must_use]
    pub fn is_structurally_complete(&self) -> bool {
        self.issues.is_empty() && self.coverage.is_exact()
    }

    /// Report syntax-level feature-boundary failures for the frozen first M9
    /// surface slice.
    ///
    /// An empty result means only that projection was exact and that no
    /// explicitly excluded surface construct was present. Raw types and
    /// expressions still require parsing, resolution, and elaboration.
    /// Fully projected `connect` endpoints as
    /// `(left_system, left_action, right_system, right_action)`.
    /// Incomplete names are skipped. This is not composition elaboration.
    #[must_use]
    pub fn surface_endpoint_wires(&self) -> Vec<(&str, &str, &str, &str)> {
        surface_endpoint_wires(&self.file)
    }

    /// `(left_action, right_action)` for wires whose left system is `left_system`.
    #[must_use]
    pub fn wired_action_pairs_for_left(&self, left_system: &str) -> Vec<(&str, &str)> {
        surface_wired_action_pairs_for_left(&self.file, left_system)
    }

    #[must_use]
    pub fn m9_surface_issues(&self) -> Vec<M9SurfaceIssue> {
        let mut issues = Vec::new();
        if !self.is_structurally_complete() {
            issues.push(M9SurfaceIssue {
                code: "NMLT-M9-SURFACE-INCOMPLETE",
                feature: "incomplete surface projection",
                span: self
                    .issues
                    .first()
                    .map_or(Span::new(0, 0), |issue| issue.span),
            });
            return issues;
        }
        collect_m9_file_issues(&self.file.declarations, &mut issues);
        issues
    }
}

/// Stable syntax-level rejection emitted before M9 name resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct M9SurfaceIssue {
    pub code: &'static str,
    pub feature: &'static str,
    pub span: Span,
}

/// Independent census of semantic CST nodes versus projected surface nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionCoverage {
    pub expected: Vec<SurfaceOrigin>,
    pub projected: Vec<SurfaceOrigin>,
    pub missing: Vec<SurfaceOrigin>,
    pub duplicated: Vec<SurfaceOrigin>,
}

impl ProjectionCoverage {
    #[must_use]
    pub fn is_exact(&self) -> bool {
        self.missing.is_empty() && self.duplicated.is_empty() && self.expected == self.projected
    }
}

/// Ordered, complete top-level surface declarations.
///
/// Every declaration node in the lossless tree has exactly one entry here.
/// Unsupported declarations remain explicit instead of disappearing before
/// resolution and elaboration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedFile {
    pub declarations: Vec<UntypedDeclaration>,
}

impl UntypedFile {
    /// Return every system in source order, including systems nested in a
    /// module declaration.
    #[must_use]
    pub fn systems(&self) -> Vec<&UntypedSystem> {
        let mut systems = Vec::new();
        collect_systems(&self.declarations, &mut systems);
        systems
    }

    /// First system whose projected name is `name`.
    #[must_use]
    pub fn system_named(&self, name: &str) -> Option<&UntypedSystem> {
        self.systems()
            .into_iter()
            .find(|system| system.name.as_ref().map(|n| n.text.as_str()) == Some(name))
    }

    /// First `compose` whose projected name is `name`, including nested modules.
    #[must_use]
    pub fn compose_named(&self, name: &str) -> Option<&UntypedCompose> {
        let mut composes = Vec::new();
        collect_composes(&self.declarations, &mut composes);
        composes
            .into_iter()
            .find(|compose| compose.name.as_ref().map(|n| n.text.as_str()) == Some(name))
    }

    /// Every projected refinement, including refinements nested in a module.
    #[must_use]
    pub fn refinements(&self) -> Vec<&UntypedRefinement> {
        let mut refinements = Vec::new();
        collect_refinements(&self.declarations, &mut refinements);
        refinements
    }
}

/// One top-level declaration in the complete surface projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UntypedDeclaration {
    Module(UntypedModule),
    Import(UntypedImport),
    Enum(UntypedEnum),
    System(UntypedSystem),
    Compose(UntypedCompose),
    Connect(UntypedConnect),
    Refinement(UntypedRefinement),
    Unsupported(UntypedSurfaceNode),
    Error(UntypedErrorNode),
}

impl UntypedDeclaration {
    /// Exact span occupied by this declaration.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Module(module) => module.span,
            Self::Import(import) => import.span,
            Self::Enum(enumeration) => enumeration.span,
            Self::System(system) => system.span,
            Self::Compose(compose) => compose.span,
            Self::Connect(connect) => connect.span,
            Self::Refinement(refinement) => refinement.span,
            Self::Unsupported(node) => node.source.span,
            Self::Error(node) => node.source.span,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedModule {
    pub name: Option<SpannedText>,
    pub span: Span,
    pub declarations: Vec<UntypedDeclaration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedImport {
    pub module: Option<SpannedText>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedEnum {
    pub name: Option<SpannedText>,
    /// Every direct enum-body node in source order.
    ///
    /// Call [`Self::supported_variants`] when only successfully shaped enum
    /// variants are relevant. Recovery and future surface-only nodes remain
    /// in this list so the projection never changes source order by omission.
    pub variants: Vec<UntypedEnumItem>,
    pub span: Span,
}

impl UntypedEnum {
    /// Iterate over successfully shaped variants without discarding the other
    /// entries from the underlying complete projection.
    pub fn supported_variants(&self) -> impl Iterator<Item = &UntypedEnumVariant> {
        self.variants.iter().filter_map(UntypedEnumItem::as_variant)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedEnumVariant {
    pub name: Option<SpannedText>,
    pub span: Span,
}

/// One direct child of an enum body in the complete surface projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UntypedEnumItem {
    Variant(UntypedEnumVariant),
    SurfaceOnly(UntypedSurfaceNode),
    Error(UntypedErrorNode),
}

impl UntypedEnumItem {
    /// Exact span occupied by this enum-body item.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Variant(variant) => variant.span,
            Self::SurfaceOnly(node) => node.source.span,
            Self::Error(node) => node.source.span,
        }
    }

    /// Return the supported variant represented by this item, if any.
    #[must_use]
    pub const fn as_variant(&self) -> Option<&UntypedEnumVariant> {
        match self {
            Self::Variant(variant) => Some(variant),
            Self::SurfaceOnly(_) | Self::Error(_) => None,
        }
    }
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
    /// Every direct parameter-list node in source order.
    pub parameters: Vec<UntypedParameterItem>,
    pub members: Vec<UntypedMember>,
}

impl UntypedSystem {
    /// Iterate over successfully shaped parameters while retaining all list
    /// items in [`Self::parameters`].
    pub fn supported_parameters(&self) -> impl Iterator<Item = &UntypedParameter> {
        self.parameters
            .iter()
            .filter_map(UntypedParameterItem::as_parameter)
    }
}

/// Top-level `compose Name { connect ... }` surface form.
///
/// This lossless frontend records the endpoint names. Semantic validation and
/// core construction occur in downstream compiler layers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedCompose {
    pub name: Option<SpannedText>,
    pub span: Span,
    pub connections: Vec<UntypedComposeItem>,
}

impl UntypedCompose {
    pub fn supported_connections(&self) -> impl Iterator<Item = &UntypedConnect> {
        self.connections
            .iter()
            .filter_map(UntypedComposeItem::as_connect)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UntypedComposeItem {
    Connect(UntypedConnect),
    SurfaceOnly(UntypedSurfaceNode),
    Error(UntypedErrorNode),
}

impl UntypedComposeItem {
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Connect(connect) => connect.span,
            Self::SurfaceOnly(node) => node.source.span,
            Self::Error(node) => node.source.span,
        }
    }

    #[must_use]
    pub const fn as_connect(&self) -> Option<&UntypedConnect> {
        match self {
            Self::Connect(connect) => Some(connect),
            Self::SurfaceOnly(_) | Self::Error(_) => None,
        }
    }
}

/// `connect Left.action -> Right.action` (file-level or inside `compose`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedConnect {
    pub left_system: Option<SpannedText>,
    pub left_action: Option<SpannedText>,
    pub right_system: Option<SpannedText>,
    pub right_action: Option<SpannedText>,
    pub span: Span,
}

/// A first-slice refinement declaration with explicit state and hidden-action maps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedRefinement {
    pub concrete_system: Option<SpannedText>,
    pub abstract_system: Option<SpannedText>,
    pub items: Vec<UntypedRefinementItem>,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UntypedRefinementItem {
    StateMap(UntypedStateMap),
    HiddenAction(UntypedObservation),
    SurfaceOnly(UntypedSurfaceNode),
    Error(UntypedErrorNode),
}

impl UntypedRefinementItem {
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::StateMap(mapping) => mapping.span,
            Self::HiddenAction(observation) => observation.span,
            Self::SurfaceOnly(node) => node.source.span,
            Self::Error(node) => node.source.span,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedStateMap {
    pub concrete_field: Option<SpannedText>,
    pub abstract_field: Option<SpannedText>,
    pub span: Span,
}

impl UntypedConnect {
    /// `(left_action, right_action)` when both action names projected.
    #[must_use]
    pub fn action_pair(&self) -> Option<(&str, &str)> {
        Some((
            self.left_action.as_ref()?.text.as_str(),
            self.right_action.as_ref()?.text.as_str(),
        ))
    }
}

/// One projected `connect`, with optional enclosing `compose` name.
///
/// Endpoints are `None` when a name failed to project. This is a name-level
/// view of surface wiring, not an executable composition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceWire<'a> {
    pub compose_name: Option<&'a str>,
    pub left_system: Option<&'a str>,
    pub left_action: Option<&'a str>,
    pub right_system: Option<&'a str>,
    pub right_action: Option<&'a str>,
}

impl<'a> SurfaceWire<'a> {
    fn from_connect(compose_name: Option<&'a str>, connect: &'a UntypedConnect) -> Self {
        Self {
            compose_name,
            left_system: connect.left_system.as_ref().map(|n| n.text.as_str()),
            left_action: connect.left_action.as_ref().map(|n| n.text.as_str()),
            right_system: connect.right_system.as_ref().map(|n| n.text.as_str()),
            right_action: connect.right_action.as_ref().map(|n| n.text.as_str()),
        }
    }

    /// Fully projected `Left.action -> Right.action` endpoints.
    #[must_use]
    pub fn endpoints(self) -> Option<(&'a str, &'a str, &'a str, &'a str)> {
        Some((
            self.left_system?,
            self.left_action?,
            self.right_system?,
            self.right_action?,
        ))
    }

    /// `(left_action, right_action)` when both action names projected.
    #[must_use]
    pub fn action_pair(self) -> Option<(&'a str, &'a str)> {
        Some((self.left_action?, self.right_action?))
    }

    /// Port-declared polarity of the left endpoint, if a same-named `port` exists.
    ///
    /// Port-only: action `input`/`output` keywords are ignored here. Use
    /// [`Self::left_declared_polarity`] to fall back to [`UntypedAction::polarity`].
    #[must_use]
    pub fn left_port_polarity(self, file: &UntypedFile) -> Option<SurfacePolarity> {
        port_polarity(file.system_named(self.left_system?)?, self.left_action?)
    }

    /// Port-declared polarity of the right endpoint, if a same-named `port` exists.
    #[must_use]
    pub fn right_port_polarity(self, file: &UntypedFile) -> Option<SurfacePolarity> {
        port_polarity(file.system_named(self.right_system?)?, self.right_action?)
    }

    /// Action-declared polarity of the left endpoint, if the named action has one.
    #[must_use]
    pub fn left_action_polarity(self, file: &UntypedFile) -> Option<SurfacePolarity> {
        action_polarity(file.system_named(self.left_system?)?, self.left_action?)
    }

    /// Action-declared polarity of the right endpoint, if the named action has one.
    #[must_use]
    pub fn right_action_polarity(self, file: &UntypedFile) -> Option<SurfacePolarity> {
        action_polarity(file.system_named(self.right_system?)?, self.right_action?)
    }

    /// Port polarity if present, else action polarity, else unknown. Never invented.
    #[must_use]
    pub fn left_declared_polarity(self, file: &UntypedFile) -> Option<SurfacePolarity> {
        declared_polarity(file.system_named(self.left_system?)?, self.left_action?)
    }

    /// Port polarity if present, else action polarity, else unknown. Never invented.
    #[must_use]
    pub fn right_declared_polarity(self, file: &UntypedFile) -> Option<SurfacePolarity> {
        declared_polarity(file.system_named(self.right_system?)?, self.right_action?)
    }

    /// `(left, right)` port polarities. `None` on either side is unknown, not invented.
    #[must_use]
    pub fn port_polarities(
        self,
        file: &UntypedFile,
    ) -> (Option<SurfacePolarity>, Option<SurfacePolarity>) {
        (
            self.left_port_polarity(file),
            self.right_port_polarity(file),
        )
    }

    /// `(left, right)` declared polarities: port wins, else action, else unknown.
    #[must_use]
    pub fn declared_polarities(
        self,
        file: &UntypedFile,
    ) -> (Option<SurfacePolarity>, Option<SurfacePolarity>) {
        (
            self.left_declared_polarity(file),
            self.right_declared_polarity(file),
        )
    }

    /// True only when both endpoints have a known port polarity and they are not complementary.
    ///
    /// Unknown polarities (no same-named `port`) are not mismatches. Action polarities
    /// are ignored here; see [`Self::has_non_complementary_declared_polarities`].
    /// This is a surface name check, not elaboration.
    #[must_use]
    pub fn has_non_complementary_port_polarities(self, file: &UntypedFile) -> bool {
        match self.port_polarities(file) {
            (Some(left), Some(right)) => !left.is_complementary_to(right),
            _ => false,
        }
    }

    /// True only when both endpoints have a known declared polarity and they are not complementary.
    ///
    /// Rule: port polarity if present, else action polarity, else skip. Missing
    /// polarities are not mismatches. Surface name check, not elaboration.
    #[must_use]
    pub fn has_non_complementary_declared_polarities(self, file: &UntypedFile) -> bool {
        match self.declared_polarities(file) {
            (Some(left), Some(right)) => !left.is_complementary_to(right),
            _ => false,
        }
    }
}

/// Declared surface input/output polarity. Not inferred from action bodies.
///
/// Sources, in lookup order used by [`declared_polarity`]:
/// - a same-named `port input` / `port output` member ([`UntypedPort::direction`]);
/// - optional `action input name` / `action output name` ([`UntypedAction::polarity`]);
/// - caller-supplied polarities keyed by `(system, name)`.
///
/// Bare `action ping { ... }` stays [`None`] — polarity is never invented. This
/// type is not an executable composition interface.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SurfacePolarity {
    Input,
    Output,
}

impl SurfacePolarity {
    /// Parse a port direction token. Only `input` and `output` are recognized.
    #[must_use]
    pub fn from_direction_text(text: &str) -> Option<Self> {
        match text {
            "input" => Some(Self::Input),
            "output" => Some(Self::Output),
            _ => None,
        }
    }

    /// One input and one output. Input-input and output-output are not complementary.
    #[must_use]
    pub const fn is_complementary_to(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Input, Self::Output) | (Self::Output, Self::Input)
        )
    }
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

/// `port input name: T` / `port output name: T`.
///
/// [`Self::direction`] is the identifier after `port` (`input` or `output` when well-formed).
/// Port polarity wins over [`UntypedAction::polarity`] when both name the same endpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedPort {
    pub direction: Option<SpannedText>,
    pub name: Option<SpannedText>,
    pub declared_type: Option<RawTerm>,
    pub span: Span,
}

impl UntypedPort {
    /// `input`/`output` polarity from [`Self::direction`], if that token is one of those words.
    #[must_use]
    pub fn polarity(&self) -> Option<SurfacePolarity> {
        self.direction
            .as_ref()
            .and_then(|direction| SurfacePolarity::from_direction_text(direction.text.as_str()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedParameter {
    pub name: Option<SpannedText>,
    pub declared_type: Option<RawTerm>,
    pub span: Span,
}

/// One direct child of a system or action parameter list in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UntypedParameterItem {
    Parameter(UntypedParameter),
    SurfaceOnly(UntypedSurfaceNode),
    Error(UntypedErrorNode),
}

impl UntypedParameterItem {
    /// Exact span occupied by this parameter-list item.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Parameter(parameter) => parameter.span,
            Self::SurfaceOnly(node) => node.source.span,
            Self::Error(node) => node.source.span,
        }
    }

    /// Return the supported parameter represented by this item, if any.
    #[must_use]
    pub const fn as_parameter(&self) -> Option<&UntypedParameter> {
        match self {
            Self::Parameter(parameter) => Some(parameter),
            Self::SurfaceOnly(_) | Self::Error(_) => None,
        }
    }
}

/// Surface `action name ... { ... }` or optional `action input|output name ... { ... }`.
///
/// [`Self::polarity`] is [`Some`] only when the parser saw `input`/`output` *and*
/// another identifier (the name). `action ping { ... }` and `action input { ... }`
/// (name `input`, no following name) stay [`None`]. `in`/`out` are not recognized.
/// Complementary-polarity checks for `connect` use this field, same-named
/// [`UntypedPort`] members (port wins), or caller-supplied maps — they do not
/// invent polarity from action bodies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedAction {
    pub name: Option<SpannedText>,
    /// `Some` only for `action input name` / `action output name`. Not inferred.
    pub polarity: Option<SurfacePolarity>,
    /// Every direct parameter-list node in source order.
    pub parameters: Vec<UntypedParameterItem>,
    pub grade: Option<SpannedText>,
    pub statements: Vec<UntypedStatement>,
    pub span: Span,
}

impl UntypedAction {
    /// Iterate over successfully shaped parameters while retaining all list
    /// items in [`Self::parameters`].
    pub fn supported_parameters(&self) -> impl Iterator<Item = &UntypedParameter> {
        self.parameters
            .iter()
            .filter_map(UntypedParameterItem::as_parameter)
    }
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

/// What a `hide` declaration conceals. `observe` does not use this.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HideSort {
    /// `hide input, channel` — omit state fields from the observation.
    StateFields,
    /// `hide action ping` — mark labels as refinement-hidden (RFC 0007).
    Actions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntypedObservation {
    pub kind: ObservationKind,
    pub hide_sort: Option<HideSort>,
    pub names: Vec<SpannedText>,
    pub expression: Option<RawTerm>,
    pub span: Span,
}

impl UntypedObservation {
    #[must_use]
    pub fn hides_actions(&self) -> bool {
        self.kind == ObservationKind::Hide && self.hide_sort == Some(HideSort::Actions)
    }
}

/// Collect action labels named by every `hide action ...` in a system.
#[must_use]
pub fn hidden_action_names(system: &UntypedSystem) -> Vec<&str> {
    system
        .members
        .iter()
        .filter_map(|member| match member {
            UntypedMember::Observation(observation) if observation.hides_actions() => {
                Some(observation)
            }
            _ => None,
        })
        .flat_map(|observation| observation.names.iter().map(|name| name.text.as_str()))
        .collect()
}

/// Left-action names that violate I-NO-HIDDEN-BOUNDARY given explicit wires.
/// `connections` are `(left_action, right_action)` pairs; only left is checked.
#[must_use]
pub fn hidden_wired_actions<'a>(
    system: &'a UntypedSystem,
    connections: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<&'a str> {
    let hidden: HashSet<&str> = hidden_action_names(system).into_iter().collect();
    let mut out = Vec::new();
    for (left, _) in connections {
        if hidden.contains(left) && !out.contains(&left) {
            out.push(left);
        }
    }
    out
}

/// Every projected `connect` in source order (file-level and inside `compose`).
#[must_use]
pub fn surface_connections(file: &UntypedFile) -> Vec<&UntypedConnect> {
    let mut out = Vec::new();
    collect_surface_connections(&file.declarations, &mut out);
    out
}

fn collect_surface_connections<'a>(
    declarations: &'a [UntypedDeclaration],
    out: &mut Vec<&'a UntypedConnect>,
) {
    for declaration in declarations {
        match declaration {
            UntypedDeclaration::Module(module) => {
                collect_surface_connections(&module.declarations, out);
            }
            UntypedDeclaration::Compose(compose) => {
                for item in &compose.connections {
                    if let UntypedComposeItem::Connect(connect) = item {
                        out.push(connect);
                    }
                }
            }
            UntypedDeclaration::Connect(connect) => out.push(connect),
            UntypedDeclaration::Import(_)
            | UntypedDeclaration::Enum(_)
            | UntypedDeclaration::System(_)
            | UntypedDeclaration::Refinement(_)
            | UntypedDeclaration::Unsupported(_)
            | UntypedDeclaration::Error(_) => {}
        }
    }
}

/// `(left_action, right_action)` pairs from surface `connect` declarations.
#[must_use]
pub fn surface_wired_action_pairs(file: &UntypedFile) -> Vec<(&str, &str)> {
    surface_connections(file)
        .into_iter()
        .filter_map(UntypedConnect::action_pair)
        .collect()
}

/// Every projected `connect` with optional enclosing `compose` name, source order.
#[must_use]
pub fn surface_wires(file: &UntypedFile) -> Vec<SurfaceWire<'_>> {
    let mut out = Vec::new();
    collect_surface_wires(&file.declarations, &mut out);
    out
}

fn collect_surface_wires<'a>(
    declarations: &'a [UntypedDeclaration],
    out: &mut Vec<SurfaceWire<'a>>,
) {
    for declaration in declarations {
        match declaration {
            UntypedDeclaration::Module(module) => {
                collect_surface_wires(&module.declarations, out);
            }
            UntypedDeclaration::Compose(compose) => {
                let name = compose.name.as_ref().map(|n| n.text.as_str());
                for item in &compose.connections {
                    if let UntypedComposeItem::Connect(connect) = item {
                        out.push(SurfaceWire::from_connect(name, connect));
                    }
                }
            }
            UntypedDeclaration::Connect(connect) => {
                out.push(SurfaceWire::from_connect(None, connect));
            }
            UntypedDeclaration::Import(_)
            | UntypedDeclaration::Enum(_)
            | UntypedDeclaration::System(_)
            | UntypedDeclaration::Refinement(_)
            | UntypedDeclaration::Unsupported(_)
            | UntypedDeclaration::Error(_) => {}
        }
    }
}

/// Fully projected `(left_system, left_action, right_system, right_action)` wires.
#[must_use]
pub fn surface_endpoint_wires(file: &UntypedFile) -> Vec<(&str, &str, &str, &str)> {
    surface_wires(file)
        .into_iter()
        .filter_map(SurfaceWire::endpoints)
        .collect()
}

/// Endpoint 4-tuples whose left system is `left_system`.
#[must_use]
pub fn surface_endpoint_wires_for_left<'a>(
    file: &'a UntypedFile,
    left_system: &str,
) -> Vec<(&'a str, &'a str, &'a str, &'a str)> {
    surface_wires(file)
        .into_iter()
        .filter(|wire| wire.left_system == Some(left_system))
        .filter_map(SurfaceWire::endpoints)
        .collect()
}

/// `(left_action, right_action)` for wires whose left system is `left_system`.
#[must_use]
pub fn surface_wired_action_pairs_for_left<'a>(
    file: &'a UntypedFile,
    left_system: &str,
) -> Vec<(&'a str, &'a str)> {
    surface_wires(file)
        .into_iter()
        .filter(|wire| wire.left_system == Some(left_system))
        .filter_map(SurfaceWire::action_pair)
        .collect()
}

/// Wires inside a named `compose` declaration.
#[must_use]
pub fn surface_wires_in_compose<'a>(
    file: &'a UntypedFile,
    compose_name: &str,
) -> Vec<SurfaceWire<'a>> {
    surface_wires(file)
        .into_iter()
        .filter(|wire| wire.compose_name == Some(compose_name))
        .collect()
}

/// Port-declared polarity of `name` on `system`, if a matching `port` exists.
///
/// Looks at [`UntypedPort::direction`] only. Action polarities are ignored here;
/// see [`action_polarity`] and [`declared_polarity`].
#[must_use]
pub fn port_polarity(system: &UntypedSystem, name: &str) -> Option<SurfacePolarity> {
    system.members.iter().find_map(|member| match member {
        UntypedMember::Port(port) if port.name.as_ref().map(|n| n.text.as_str()) == Some(name) => {
            port.polarity()
        }
        _ => None,
    })
}

/// Action-declared polarity of `name` on `system`, if that action has one.
///
/// Reads [`UntypedAction::polarity`] only. `action ping { ... }` yields [`None`].
#[must_use]
pub fn action_polarity(system: &UntypedSystem, name: &str) -> Option<SurfacePolarity> {
    system.members.iter().find_map(|member| match member {
        UntypedMember::Action(action)
            if action.name.as_ref().map(|n| n.text.as_str()) == Some(name) =>
        {
            action.polarity
        }
        _ => None,
    })
}

/// Port polarity if present, else action polarity, else unknown.
///
/// Never inferred from action bodies. A same-named port wins when both exist.
#[must_use]
pub fn declared_polarity(system: &UntypedSystem, name: &str) -> Option<SurfacePolarity> {
    port_polarity(system, name).or_else(|| action_polarity(system, name))
}

/// `(system, port_name)` polarities from every well-formed `port input` / `port output`.
///
/// Action polarities are not included; see [`surface_action_polarities`].
#[must_use]
pub fn surface_port_polarities(file: &UntypedFile) -> BTreeMap<(String, String), SurfacePolarity> {
    let mut out = BTreeMap::new();
    for system in file.systems() {
        let Some(system_name) = system.name.as_ref() else {
            continue;
        };
        for member in &system.members {
            if let UntypedMember::Port(port) = member {
                let Some(port_name) = port.name.as_ref() else {
                    continue;
                };
                if let Some(polarity) = port.polarity() {
                    out.insert((system_name.text.clone(), port_name.text.clone()), polarity);
                }
            }
        }
    }
    out
}

/// `(system, action_name)` polarities from every `action input|output name`.
///
/// Bare `action ping` contributes nothing. Ports are not included.
#[must_use]
pub fn surface_action_polarities(
    file: &UntypedFile,
) -> BTreeMap<(String, String), SurfacePolarity> {
    let mut out = BTreeMap::new();
    for system in file.systems() {
        let Some(system_name) = system.name.as_ref() else {
            continue;
        };
        for member in &system.members {
            if let UntypedMember::Action(action) = member {
                let Some(action_name) = action.name.as_ref() else {
                    continue;
                };
                if let Some(polarity) = action.polarity {
                    out.insert(
                        (system_name.text.clone(), action_name.text.clone()),
                        polarity,
                    );
                }
            }
        }
    }
    out
}

/// Declared polarities: action map, then port map overwrites (port wins).
#[must_use]
pub fn surface_declared_polarities(
    file: &UntypedFile,
) -> BTreeMap<(String, String), SurfacePolarity> {
    let mut out = surface_action_polarities(file);
    out.extend(surface_port_polarities(file));
    out
}

/// Wires whose caller-supplied polarities are both known and not complementary.
///
/// `polarity(system, name)` should return [`SurfacePolarity`] when the caller
/// knows one (from a test, a later elaborator, [`port_polarity`], or
/// [`action_polarity`]). Missing polarities are skipped — this helper does
/// **not** invent in/out from action bodies. Input-input and output-output
/// wires are returned. Surface name check only; M9 still fail-closes compose
/// (`NMLT-M9-COMPOSE` / `NMLT-M9-CONNECT`) and polarized actions
/// (`NMLT-M9-ACTION-POLARITY`).
#[must_use]
pub fn non_complementary_surface_wires<'a, F>(
    wires: impl IntoIterator<Item = SurfaceWire<'a>>,
    mut polarity: F,
) -> Vec<SurfaceWire<'a>>
where
    F: FnMut(&str, &str) -> Option<SurfacePolarity>,
{
    let mut out = Vec::new();
    for wire in wires {
        let Some((left_system, left_action, right_system, right_action)) = wire.endpoints() else {
            continue;
        };
        match (
            polarity(left_system, left_action),
            polarity(right_system, right_action),
        ) {
            (Some(left), Some(right)) if !left.is_complementary_to(right) => out.push(wire),
            _ => {}
        }
    }
    out
}

/// Complementary-polarity mismatches using `port input` / `port output` as the source.
///
/// Endpoints without a same-named port are skipped, even if the action has
/// polarity. Use [`non_complementary_declared_wires`] to fall back to actions.
/// Not composition elaboration.
#[must_use]
pub fn non_complementary_port_wires(file: &UntypedFile) -> Vec<SurfaceWire<'_>> {
    non_complementary_surface_wires(surface_wires(file), |system, name| {
        file.system_named(system)
            .and_then(|system| port_polarity(system, name))
    })
}

/// Complementary-polarity mismatches using declared polarities.
///
/// Lookup: port polarity if present, else action polarity, else skip. Bare
/// `action ping` is unknown. Not composition elaboration.
#[must_use]
pub fn non_complementary_declared_wires(file: &UntypedFile) -> Vec<SurfaceWire<'_>> {
    non_complementary_surface_wires(surface_wires(file), |system, name| {
        file.system_named(system)
            .and_then(|system| declared_polarity(system, name))
    })
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
    fn duplicate_key(&self) -> Option<(u8, &SpannedText)> {
        match self {
            Self::Binding(binding) => binding.name.as_ref().map(|name| {
                let namespace = match binding.kind {
                    BindingKind::Const => 0x03,
                    BindingKind::Input => 0x07,
                    BindingKind::State => 0x05,
                    BindingKind::Capability => 0x08,
                };
                (namespace, name)
            }),
            Self::Port(port) => port.name.as_ref().map(|name| (0x0b, name)),
            Self::Action(action) => action.name.as_ref().map(|name| (0x06, name)),
            Self::Property(property) => property.name.as_ref().map(|name| (0x09, name)),
            Self::Observation(_) | Self::SurfaceOnly(_) | Self::Error(_) => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UntypedStatement {
    Require {
        condition: RawTerm,
        span: Span,
    },
    Update {
        target: UntypedUpdateTarget,
        value: RawTerm,
        span: Span,
    },
    Emit {
        value: RawTerm,
        span: Span,
    },
    Consume {
        capability: RawTerm,
        span: Span,
    },
    Rely {
        fact: RawTerm,
        span: Span,
    },
    Guarantee {
        fact: RawTerm,
        span: Span,
    },
    SurfaceOnly(UntypedSurfaceNode),
    Error(UntypedErrorNode),
}

/// A syntactic location is an identifier followed by zero or more field or
/// index selectors. Whether its root denotes state is intentionally deferred.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UntypedUpdateTarget {
    Location {
        root: SpannedText,
        source: SpannedText,
        origin: SurfaceOrigin,
    },
    Unsupported {
        source: SpannedText,
        origin: SurfaceOrigin,
    },
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
    SyntaxDiagnostic {
        code: &'static str,
    },
    MissingDiagnosticSpan {
        code: &'static str,
    },
    RecoveryNode,
    DuplicateDeclaration {
        name: String,
        first_span: Span,
    },
    InvalidUpdateTarget,
    MissingProjectedChild {
        parent: SyntaxKind,
        expected: SyntaxKind,
    },
    UnexpectedProjectedNode {
        kind: SyntaxKind,
    },
    MissingCoverage {
        kind: SyntaxKind,
    },
    DuplicateCoverage {
        kind: SyntaxKind,
    },
    CoverageOrderMismatch,
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
    };
    let declarations = projection.project_declarations(parsed.root(), 0);
    let file = UntypedFile { declarations };
    let coverage = projection_coverage(parsed.root(), &file);
    for missing in &coverage.missing {
        projection.issues.push(ProjectionIssue {
            kind: ProjectionIssueKind::MissingCoverage { kind: missing.kind },
            span: missing.span,
        });
    }
    for duplicated in &coverage.duplicated {
        projection.issues.push(ProjectionIssue {
            kind: ProjectionIssueKind::DuplicateCoverage {
                kind: duplicated.kind,
            },
            span: duplicated.span,
        });
    }
    if coverage.missing.is_empty()
        && coverage.duplicated.is_empty()
        && coverage.expected != coverage.projected
    {
        let span = coverage
            .expected
            .iter()
            .zip(&coverage.projected)
            .find_map(|(expected, projected)| (expected != projected).then_some(expected.span))
            .unwrap_or(Span::new(0, 0));
        projection.issues.push(ProjectionIssue {
            kind: ProjectionIssueKind::CoverageOrderMismatch,
            span,
        });
    }
    UntypedProjection {
        file,
        issues: projection.issues,
        coverage,
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
}

impl Projector {
    fn project_declarations(&mut self, node: &GreenNode, base: usize) -> Vec<UntypedDeclaration> {
        direct_nodes(node, base)
            .into_iter()
            .map(|child| self.project_declaration(child))
            .collect()
    }

    fn project_declaration(&mut self, node: NodeAt<'_>) -> UntypedDeclaration {
        match node.node.kind() {
            SyntaxKind::ModuleDecl => UntypedDeclaration::Module(self.project_module(node)),
            SyntaxKind::ImportDecl => UntypedDeclaration::Import(self.project_import(node)),
            SyntaxKind::EnumDecl => UntypedDeclaration::Enum(self.project_enum(node)),
            SyntaxKind::SystemDecl => {
                UntypedDeclaration::System(self.project_system(node.node, node.span.start))
            }
            SyntaxKind::ComposeDecl => UntypedDeclaration::Compose(self.project_compose(node)),
            SyntaxKind::ConnectDecl => UntypedDeclaration::Connect(self.project_connect(node)),
            SyntaxKind::RefineDecl => UntypedDeclaration::Refinement(self.project_refinement(node)),
            SyntaxKind::Error => {
                self.record_recovery(node.span);
                UntypedDeclaration::Error(error_node(node))
            }
            SyntaxKind::DataDecl
            | SyntaxKind::TypeDecl
            | SyntaxKind::RecordDecl
            | SyntaxKind::FunctionDecl => UntypedDeclaration::Unsupported(surface_node(node)),
            kind => {
                self.issues.push(ProjectionIssue {
                    kind: ProjectionIssueKind::UnexpectedProjectedNode { kind },
                    span: node.span,
                });
                UntypedDeclaration::Unsupported(surface_node(node))
            }
        }
    }

    fn project_module(&mut self, node: NodeAt<'_>) -> UntypedModule {
        UntypedModule {
            name: identifier(node.node, node.span.start, 1),
            span: node.span,
            declarations: self.project_declarations(node.node, node.span.start),
        }
    }

    fn project_import(&mut self, node: NodeAt<'_>) -> UntypedImport {
        UntypedImport {
            module: identifier(node.node, node.span.start, 1),
            span: node.span,
        }
    }

    fn project_compose(&mut self, node: NodeAt<'_>) -> UntypedCompose {
        let children = direct_nodes(node.node, node.span.start);
        let mut connections = Vec::new();
        if let Some(body) = children
            .into_iter()
            .find(|child| child.node.kind() == SyntaxKind::ComposeBody)
        {
            for child in direct_nodes(body.node, body.span.start) {
                let item = match child.node.kind() {
                    SyntaxKind::ConnectDecl => {
                        UntypedComposeItem::Connect(self.project_connect(child))
                    }
                    SyntaxKind::Error => {
                        self.record_recovery(child.span);
                        UntypedComposeItem::Error(error_node(child))
                    }
                    kind => {
                        self.issues.push(ProjectionIssue {
                            kind: ProjectionIssueKind::UnexpectedProjectedNode { kind },
                            span: child.span,
                        });
                        UntypedComposeItem::SurfaceOnly(surface_node(child))
                    }
                };
                connections.push(item);
            }
        }
        UntypedCompose {
            name: identifier(node.node, node.span.start, 1),
            span: node.span,
            connections,
        }
    }

    fn project_connect(&mut self, node: NodeAt<'_>) -> UntypedConnect {
        // Identifier tokens: connect, left_system, left_action, right_system, right_action
        UntypedConnect {
            left_system: identifier(node.node, node.span.start, 1),
            left_action: identifier(node.node, node.span.start, 2),
            right_system: identifier(node.node, node.span.start, 3),
            right_action: identifier(node.node, node.span.start, 4),
            span: node.span,
        }
    }

    fn project_refinement(&mut self, node: NodeAt<'_>) -> UntypedRefinement {
        let children = direct_nodes(node.node, node.span.start);
        let mut items = Vec::new();
        if let Some(body) = children
            .into_iter()
            .find(|child| child.node.kind() == SyntaxKind::RefineBody)
        {
            for child in direct_nodes(body.node, body.span.start) {
                let item = match child.node.kind() {
                    SyntaxKind::MapStateDecl => UntypedRefinementItem::StateMap(UntypedStateMap {
                        concrete_field: identifier(child.node, child.span.start, 2),
                        abstract_field: identifier(child.node, child.span.start, 3),
                        span: child.span,
                    }),
                    SyntaxKind::HideDecl => {
                        match self.project_observation(child, ObservationKind::Hide) {
                            UntypedMember::Observation(observation) => {
                                UntypedRefinementItem::HiddenAction(observation)
                            }
                            _ => unreachable!("observation projection returns an observation"),
                        }
                    }
                    SyntaxKind::Error => {
                        self.record_recovery(child.span);
                        UntypedRefinementItem::Error(error_node(child))
                    }
                    kind => {
                        self.issues.push(ProjectionIssue {
                            kind: ProjectionIssueKind::UnexpectedProjectedNode { kind },
                            span: child.span,
                        });
                        UntypedRefinementItem::SurfaceOnly(surface_node(child))
                    }
                };
                items.push(item);
            }
        }
        UntypedRefinement {
            concrete_system: identifier(node.node, node.span.start, 1),
            abstract_system: identifier(node.node, node.span.start, 3),
            items,
            span: node.span,
        }
    }

    fn project_enum(&mut self, node: NodeAt<'_>) -> UntypedEnum {
        let variants = direct_nodes(node.node, node.span.start)
            .into_iter()
            .map(|variant| match variant.node.kind() {
                SyntaxKind::EnumVariant => UntypedEnumItem::Variant(UntypedEnumVariant {
                    name: identifier(variant.node, variant.span.start, 0),
                    span: variant.span,
                }),
                SyntaxKind::Error => {
                    self.record_recovery(variant.span);
                    UntypedEnumItem::Error(error_node(variant))
                }
                kind => {
                    self.issues.push(ProjectionIssue {
                        kind: ProjectionIssueKind::UnexpectedProjectedNode { kind },
                        span: variant.span,
                    });
                    UntypedEnumItem::SurfaceOnly(surface_node(variant))
                }
            })
            .collect();
        UntypedEnum {
            name: identifier(node.node, node.span.start, 1),
            variants,
            span: node.span,
        }
    }

    fn project_system(&mut self, node: &GreenNode, base: usize) -> UntypedSystem {
        let name = identifier(node, base, 1);
        let span = Span::new(base, base + node.text_len());
        let children = direct_nodes(node, base);
        let parameters = children
            .iter()
            .find(|child| child.node.kind() == SyntaxKind::ParameterList)
            .map_or_else(Vec::new, |list| self.project_parameters(*list));
        let mut members = Vec::new();
        if let Some(body) = children
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
            parameters,
            members,
        }
    }

    fn project_parameters(&mut self, list: NodeAt<'_>) -> Vec<UntypedParameterItem> {
        direct_nodes(list.node, list.span.start)
            .into_iter()
            .map(|parameter| match parameter.node.kind() {
                SyntaxKind::Parameter => {
                    let children = direct_nodes(parameter.node, parameter.span.start);
                    UntypedParameterItem::Parameter(UntypedParameter {
                        name: identifier(parameter.node, parameter.span.start, 0),
                        declared_type: raw_child(&children, SyntaxKind::TypeExpr),
                        span: parameter.span,
                    })
                }
                SyntaxKind::Error => {
                    self.record_recovery(parameter.span);
                    UntypedParameterItem::Error(error_node(parameter))
                }
                kind => {
                    self.issues.push(ProjectionIssue {
                        kind: ProjectionIssueKind::UnexpectedProjectedNode { kind },
                        span: parameter.span,
                    });
                    UntypedParameterItem::SurfaceOnly(surface_node(parameter))
                }
            })
            .collect()
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
            .map_or_else(Vec::new, |list| self.project_parameters(*list));
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
                    .map(|statement| self.project_statement(statement))
                    .collect()
            });
        let (name, polarity) = action_name_and_polarity(node.node, node.span.start);
        UntypedMember::Action(UntypedAction {
            name,
            polarity,
            parameters,
            grade,
            statements,
            span: node.span,
        })
    }

    fn project_statement(&mut self, node: NodeAt<'_>) -> UntypedStatement {
        let expressions = direct_nodes(node.node, node.span.start)
            .into_iter()
            .filter(|child| child.node.kind() == SyntaxKind::Expr)
            .filter_map(raw_term)
            .collect::<Vec<_>>();
        match node.node.kind() {
            SyntaxKind::RequireStmt => self.only_expression(expressions, node).map_or_else(
                UntypedStatement::Error,
                |condition| UntypedStatement::Require {
                    condition,
                    span: node.span,
                },
            ),
            SyntaxKind::EmitStmt => self.only_expression(expressions, node).map_or_else(
                UntypedStatement::Error,
                |value| UntypedStatement::Emit {
                    value,
                    span: node.span,
                },
            ),
            SyntaxKind::ConsumeStmt => self.only_expression(expressions, node).map_or_else(
                UntypedStatement::Error,
                |capability| UntypedStatement::Consume {
                    capability,
                    span: node.span,
                },
            ),
            SyntaxKind::RelyStmt => self.only_expression(expressions, node).map_or_else(
                UntypedStatement::Error,
                |fact| UntypedStatement::Rely {
                    fact,
                    span: node.span,
                },
            ),
            SyntaxKind::GuaranteeStmt => self.only_expression(expressions, node).map_or_else(
                UntypedStatement::Error,
                |fact| UntypedStatement::Guarantee {
                    fact,
                    span: node.span,
                },
            ),
            SyntaxKind::UpdateStmt => {
                let mut expressions = expressions.into_iter();
                let target = expressions.next();
                let value = expressions.next();
                match (target, value, expressions.next()) {
                    (Some(target), Some(value), None) => {
                        let target = self.project_update_target(target);
                        UntypedStatement::Update {
                            target,
                            value,
                            span: node.span,
                        }
                    }
                    _ => {
                        self.record_missing_child(node, SyntaxKind::Expr);
                        UntypedStatement::Error(error_node(node))
                    }
                }
            }
            SyntaxKind::Error => {
                self.record_recovery(node.span);
                UntypedStatement::Error(error_node(node))
            }
            kind => {
                self.issues.push(ProjectionIssue {
                    kind: ProjectionIssueKind::UnexpectedProjectedNode { kind },
                    span: node.span,
                });
                UntypedStatement::SurfaceOnly(surface_node(node))
            }
        }
    }

    fn only_expression(
        &mut self,
        expressions: Vec<RawTerm>,
        node: NodeAt<'_>,
    ) -> Result<RawTerm, UntypedErrorNode> {
        let mut expressions = expressions.into_iter();
        match (expressions.next(), expressions.next()) {
            (Some(expression), None) => Ok(expression),
            _ => {
                self.record_missing_child(node, SyntaxKind::Expr);
                Err(error_node(node))
            }
        }
    }

    fn record_missing_child(&mut self, node: NodeAt<'_>, expected: SyntaxKind) {
        self.issues.push(ProjectionIssue {
            kind: ProjectionIssueKind::MissingProjectedChild {
                parent: node.node.kind(),
                expected,
            },
            span: node.span,
        });
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
                origin: target.origin,
            }
        } else {
            self.issues.push(ProjectionIssue {
                kind: ProjectionIssueKind::InvalidUpdateTarget,
                span: source.span,
            });
            UntypedUpdateTarget::Unsupported {
                source: source.clone(),
                origin: target.origin,
            }
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
        let expression = raw_child(&children, SyntaxKind::Expr);
        let (hide_sort, names) = match kind {
            ObservationKind::Hide => classify_hide_expression(expression.as_ref()),
            ObservationKind::Observe => (None, observation_names(expression.as_ref())),
        };
        UntypedMember::Observation(UntypedObservation {
            kind,
            hide_sort,
            names,
            expression,
            span: node.span,
        })
    }

    fn record_duplicate_members(&mut self, members: &[UntypedMember]) {
        let mut first_by_name = BTreeMap::<(u8, String), Span>::new();
        for (namespace, name) in members.iter().filter_map(UntypedMember::duplicate_key) {
            let key = (namespace, name.text.clone());
            if let Some(first_span) = first_by_name.get(&key).copied() {
                self.issues.push(ProjectionIssue {
                    kind: ProjectionIssueKind::DuplicateDeclaration {
                        name: name.text.clone(),
                        first_span,
                    },
                    span: name.span,
                });
            } else {
                first_by_name.insert(key, name.span);
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

fn collect_systems<'file>(
    declarations: &'file [UntypedDeclaration],
    systems: &mut Vec<&'file UntypedSystem>,
) {
    for declaration in declarations {
        match declaration {
            UntypedDeclaration::Module(module) => {
                collect_systems(&module.declarations, systems);
            }
            UntypedDeclaration::System(system) => systems.push(system),
            UntypedDeclaration::Import(_)
            | UntypedDeclaration::Enum(_)
            | UntypedDeclaration::Compose(_)
            | UntypedDeclaration::Connect(_)
            | UntypedDeclaration::Refinement(_)
            | UntypedDeclaration::Unsupported(_)
            | UntypedDeclaration::Error(_) => {}
        }
    }
}

fn collect_composes<'file>(
    declarations: &'file [UntypedDeclaration],
    composes: &mut Vec<&'file UntypedCompose>,
) {
    for declaration in declarations {
        match declaration {
            UntypedDeclaration::Module(module) => {
                collect_composes(&module.declarations, composes);
            }
            UntypedDeclaration::Compose(compose) => composes.push(compose),
            UntypedDeclaration::Import(_)
            | UntypedDeclaration::Enum(_)
            | UntypedDeclaration::System(_)
            | UntypedDeclaration::Connect(_)
            | UntypedDeclaration::Refinement(_)
            | UntypedDeclaration::Unsupported(_)
            | UntypedDeclaration::Error(_) => {}
        }
    }
}

fn collect_refinements<'file>(
    declarations: &'file [UntypedDeclaration],
    refinements: &mut Vec<&'file UntypedRefinement>,
) {
    for declaration in declarations {
        match declaration {
            UntypedDeclaration::Module(module) => {
                collect_refinements(&module.declarations, refinements);
            }
            UntypedDeclaration::Refinement(refinement) => refinements.push(refinement),
            UntypedDeclaration::Import(_)
            | UntypedDeclaration::Enum(_)
            | UntypedDeclaration::System(_)
            | UntypedDeclaration::Compose(_)
            | UntypedDeclaration::Connect(_)
            | UntypedDeclaration::Unsupported(_)
            | UntypedDeclaration::Error(_) => {}
        }
    }
}

fn collect_m9_file_issues(declarations: &[UntypedDeclaration], issues: &mut Vec<M9SurfaceIssue>) {
    let module_count = declarations
        .iter()
        .filter(|declaration| matches!(declaration, UntypedDeclaration::Module(_)))
        .count();
    if module_count > 0 && (module_count != 1 || declarations.len() != 1) {
        for declaration in declarations {
            issues.push(M9SurfaceIssue {
                code: "NMLT-M9-MODULE-LAYOUT",
                feature: "module wrapper mixed with other top-level declarations",
                span: declaration.span(),
            });
        }
        return;
    }
    collect_m9_declaration_issues(declarations, 0, issues);
}

fn collect_m9_declaration_issues(
    declarations: &[UntypedDeclaration],
    module_depth: usize,
    issues: &mut Vec<M9SurfaceIssue>,
) {
    for declaration in declarations {
        match declaration {
            UntypedDeclaration::Module(module) => {
                if module_depth > 0 {
                    issues.push(M9SurfaceIssue {
                        code: "NMLT-M9-NESTED-MODULE",
                        feature: "nested module declaration",
                        span: module.span,
                    });
                } else {
                    collect_m9_declaration_issues(&module.declarations, module_depth + 1, issues);
                }
            }
            UntypedDeclaration::Import(import) => {
                if import.module.is_none() {
                    issues.push(M9SurfaceIssue {
                        code: "NMLT-M9-IMPORT-TARGET",
                        feature: "missing import target",
                        span: import.span,
                    });
                }
            }
            UntypedDeclaration::Enum(_) => {}
            UntypedDeclaration::System(system) => collect_m9_system_issues(system, issues),
            UntypedDeclaration::Compose(compose) => {
                issues.push(M9SurfaceIssue {
                    code: "NMLT-M9-COMPOSE",
                    feature: "compose declaration (surface wiring only; no elaborator)",
                    span: compose.span,
                });
                for item in &compose.connections {
                    if let UntypedComposeItem::Connect(connect) = item {
                        issues.push(M9SurfaceIssue {
                            code: "NMLT-M9-CONNECT",
                            feature: "connect declaration (surface wiring only; no elaborator)",
                            span: connect.span,
                        });
                    } else {
                        issues.push(M9SurfaceIssue {
                            code: "NMLT-M9-SURFACE-INCOMPLETE",
                            feature: "recovered or unsupported compose member",
                            span: item.span(),
                        });
                    }
                }
            }
            UntypedDeclaration::Connect(connect) => issues.push(M9SurfaceIssue {
                code: "NMLT-M9-CONNECT",
                feature: "connect declaration (surface wiring only; no elaborator)",
                span: connect.span,
            }),
            UntypedDeclaration::Refinement(refinement) => issues.push(M9SurfaceIssue {
                code: "NMLT-M9-REFINE",
                feature: "resource-aware refinement declaration",
                span: refinement.span,
            }),
            UntypedDeclaration::Unsupported(node) => issues.push(M9SurfaceIssue {
                code: "NMLT-M9-UNSUPPORTED-DECLARATION",
                feature: unsupported_declaration_name(node.kind),
                span: node.source.span,
            }),
            UntypedDeclaration::Error(node) => issues.push(M9SurfaceIssue {
                code: "NMLT-M9-SURFACE-INCOMPLETE",
                feature: "recovered declaration",
                span: node.source.span,
            }),
        }
    }
}

fn collect_m9_system_issues(system: &UntypedSystem, issues: &mut Vec<M9SurfaceIssue>) {
    for parameter in &system.parameters {
        issues.push(M9SurfaceIssue {
            code: "NMLT-M9-SYSTEM-PARAMETER",
            feature: "system parameter",
            span: parameter.span(),
        });
    }
    for member in &system.members {
        match member {
            UntypedMember::Binding(binding) => match binding.kind {
                BindingKind::State | BindingKind::Capability => {}
                BindingKind::Const => issues.push(M9SurfaceIssue {
                    code: "NMLT-M9-SYSTEM-CONSTANT",
                    feature: "system constant",
                    span: binding.span,
                }),
                BindingKind::Input => issues.push(M9SurfaceIssue {
                    code: "NMLT-M9-SYSTEM-INPUT",
                    feature: "system input",
                    span: binding.span,
                }),
            },
            UntypedMember::Port(port) => issues.push(M9SurfaceIssue {
                code: "NMLT-M9-PORT",
                feature: "port declaration",
                span: port.span,
            }),
            UntypedMember::Action(action) => collect_m9_action_issues(action, issues),
            UntypedMember::Property(property) => {
                if property.kind == PropertyKind::Resource {
                    issues.push(M9SurfaceIssue {
                        code: "NMLT-M9-RESOURCE-PROPERTY",
                        feature: "resource property",
                        span: property.span,
                    });
                }
            }
            UntypedMember::Observation(observation) => {
                if observation.kind == ObservationKind::Hide {
                    if observation.hides_actions() {
                        issues.push(M9SurfaceIssue {
                            code: "NMLT-M9-HIDE-ACTION",
                            feature: "hide action (refinement-hidden label)",
                            span: observation.span,
                        });
                    } else {
                        issues.push(M9SurfaceIssue {
                            code: "NMLT-M9-HIDING",
                            feature: "hiding declaration",
                            span: observation.span,
                        });
                    }
                }
            }
            UntypedMember::SurfaceOnly(node) => issues.push(M9SurfaceIssue {
                code: "NMLT-M9-UNSUPPORTED-MEMBER",
                feature: "unsupported system member",
                span: node.source.span,
            }),
            UntypedMember::Error(node) => issues.push(M9SurfaceIssue {
                code: "NMLT-M9-SURFACE-INCOMPLETE",
                feature: "recovered system member",
                span: node.source.span,
            }),
        }
    }
}

fn collect_m9_action_issues(action: &UntypedAction, issues: &mut Vec<M9SurfaceIssue>) {
    if action.polarity.is_some() {
        issues.push(M9SurfaceIssue {
            code: "NMLT-M9-ACTION-POLARITY",
            feature: "action input/output polarity",
            span: action.span,
        });
    }
    if let Some(grade) = &action.grade {
        issues.push(M9SurfaceIssue {
            code: "NMLT-M9-ACTION-GRADE",
            feature: "action grade",
            span: grade.span,
        });
    }
    for statement in &action.statements {
        match statement {
            UntypedStatement::Update { target, .. } => match target {
                UntypedUpdateTarget::Location { root, source, .. } if root.text != source.text => {
                    issues.push(M9SurfaceIssue {
                        code: "NMLT-M9-SELECTED-UPDATE",
                        feature: "field or indexed update target",
                        span: source.span,
                    });
                }
                UntypedUpdateTarget::Unsupported { source, .. } => {
                    issues.push(M9SurfaceIssue {
                        code: "NMLT-M9-UPDATE-TARGET",
                        feature: "unsupported update target",
                        span: source.span,
                    });
                }
                UntypedUpdateTarget::Location { .. } => {}
            },
            UntypedStatement::SurfaceOnly(node) => issues.push(M9SurfaceIssue {
                code: "NMLT-M9-UNSUPPORTED-STATEMENT",
                feature: "unsupported action statement",
                span: node.source.span,
            }),
            UntypedStatement::Error(node) => issues.push(M9SurfaceIssue {
                code: "NMLT-M9-SURFACE-INCOMPLETE",
                feature: "recovered action statement",
                span: node.source.span,
            }),
            UntypedStatement::Require { .. }
            | UntypedStatement::Emit { .. }
            | UntypedStatement::Consume { .. } => {}
            UntypedStatement::Rely { span, .. } => issues.push(M9SurfaceIssue {
                code: "NMLT-M9-RELY",
                feature: "action rely contract",
                span: *span,
            }),
            UntypedStatement::Guarantee { span, .. } => issues.push(M9SurfaceIssue {
                code: "NMLT-M9-GUARANTEE",
                feature: "action guarantee contract",
                span: *span,
            }),
        }
    }
}

const fn unsupported_declaration_name(kind: SyntaxKind) -> &'static str {
    match kind {
        SyntaxKind::DataDecl => "data declaration",
        SyntaxKind::TypeDecl => "type alias",
        SyntaxKind::RecordDecl => "record declaration",
        SyntaxKind::FunctionDecl => "function declaration",
        _ => "unsupported declaration",
    }
}

fn projection_coverage(root: &GreenNode, file: &UntypedFile) -> ProjectionCoverage {
    let mut expected = Vec::new();
    census_semantic_nodes(root, 0, &mut expected);
    let mut projected = Vec::new();
    census_projected_file(file, &mut projected);

    let expected_counts = origin_counts(&expected);
    let projected_counts = origin_counts(&projected);
    let missing = expected
        .iter()
        .copied()
        .filter(|origin| {
            projected_counts.get(origin).copied().unwrap_or(0)
                < expected_counts.get(origin).copied().unwrap_or(0)
        })
        .collect::<Vec<_>>();
    let duplicated = projected
        .iter()
        .copied()
        .filter(|origin| {
            projected_counts.get(origin).copied().unwrap_or(0)
                > expected_counts.get(origin).copied().unwrap_or(0)
        })
        .collect::<Vec<_>>();

    ProjectionCoverage {
        expected,
        projected,
        missing: deduplicate_origins(missing),
        duplicated: deduplicate_origins(duplicated),
    }
}

fn origin_counts(origins: &[SurfaceOrigin]) -> HashMap<SurfaceOrigin, usize> {
    let mut counts = HashMap::new();
    for origin in origins {
        let count = counts.entry(*origin).or_insert(0_usize);
        *count += 1;
    }
    counts
}

fn deduplicate_origins(origins: Vec<SurfaceOrigin>) -> Vec<SurfaceOrigin> {
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for origin in origins {
        if seen.insert(origin) {
            unique.push(origin);
        }
    }
    unique
}

fn census_semantic_nodes(node: &GreenNode, base: usize, origins: &mut Vec<SurfaceOrigin>) {
    if is_semantic_node(node.kind()) {
        origins.push(SurfaceOrigin {
            kind: node.kind(),
            span: Span::new(base, base + node.text_len()),
        });
    }
    for child in direct_nodes(node, base) {
        census_semantic_nodes(child.node, child.span.start, origins);
    }
}

const fn is_semantic_node(kind: SyntaxKind) -> bool {
    match kind {
        SyntaxKind::SourceFile
        | SyntaxKind::SystemBody
        | SyntaxKind::ComposeBody
        | SyntaxKind::RefineBody
        | SyntaxKind::ParameterList
        | SyntaxKind::ActionBody => false,
        SyntaxKind::ModuleDecl
        | SyntaxKind::ImportDecl
        | SyntaxKind::DataDecl
        | SyntaxKind::TypeDecl
        | SyntaxKind::RecordDecl
        | SyntaxKind::FunctionDecl
        | SyntaxKind::EnumDecl
        | SyntaxKind::EnumVariant
        | SyntaxKind::SystemDecl
        | SyntaxKind::Parameter
        | SyntaxKind::ConstDecl
        | SyntaxKind::InputDecl
        | SyntaxKind::StateDecl
        | SyntaxKind::CapabilityDecl
        | SyntaxKind::PortDecl
        | SyntaxKind::ActionDecl
        | SyntaxKind::GradeClause
        | SyntaxKind::RequireStmt
        | SyntaxKind::UpdateStmt
        | SyntaxKind::EmitStmt
        | SyntaxKind::ConsumeStmt
        | SyntaxKind::RelyStmt
        | SyntaxKind::GuaranteeStmt
        | SyntaxKind::SafetyDecl
        | SyntaxKind::TemporalDecl
        | SyntaxKind::ResourceDecl
        | SyntaxKind::ObserveDecl
        | SyntaxKind::HideDecl
        | SyntaxKind::ComposeDecl
        | SyntaxKind::ConnectDecl
        | SyntaxKind::RefineDecl
        | SyntaxKind::MapStateDecl
        | SyntaxKind::TypeExpr
        | SyntaxKind::Expr
        | SyntaxKind::Error => true,
    }
}

fn census_projected_file(file: &UntypedFile, origins: &mut Vec<SurfaceOrigin>) {
    for declaration in &file.declarations {
        census_projected_declaration(declaration, origins);
    }
}

fn census_projected_declaration(
    declaration: &UntypedDeclaration,
    origins: &mut Vec<SurfaceOrigin>,
) {
    match declaration {
        UntypedDeclaration::Module(module) => {
            origins.push(SurfaceOrigin {
                kind: SyntaxKind::ModuleDecl,
                span: module.span,
            });
            for declaration in &module.declarations {
                census_projected_declaration(declaration, origins);
            }
        }
        UntypedDeclaration::Import(import) => origins.push(SurfaceOrigin {
            kind: SyntaxKind::ImportDecl,
            span: import.span,
        }),
        UntypedDeclaration::Enum(enumeration) => {
            origins.push(SurfaceOrigin {
                kind: SyntaxKind::EnumDecl,
                span: enumeration.span,
            });
            for item in &enumeration.variants {
                match item {
                    UntypedEnumItem::Variant(variant) => origins.push(SurfaceOrigin {
                        kind: SyntaxKind::EnumVariant,
                        span: variant.span,
                    }),
                    UntypedEnumItem::SurfaceOnly(node) => origins.push(SurfaceOrigin {
                        kind: node.kind,
                        span: node.source.span,
                    }),
                    UntypedEnumItem::Error(node) => origins.push(SurfaceOrigin {
                        kind: SyntaxKind::Error,
                        span: node.source.span,
                    }),
                }
            }
        }
        UntypedDeclaration::System(system) => census_projected_system(system, origins),
        UntypedDeclaration::Compose(compose) => {
            origins.push(SurfaceOrigin {
                kind: SyntaxKind::ComposeDecl,
                span: compose.span,
            });
            for item in &compose.connections {
                match item {
                    UntypedComposeItem::Connect(connect) => origins.push(SurfaceOrigin {
                        kind: SyntaxKind::ConnectDecl,
                        span: connect.span,
                    }),
                    UntypedComposeItem::SurfaceOnly(node) => origins.push(SurfaceOrigin {
                        kind: node.kind,
                        span: node.source.span,
                    }),
                    UntypedComposeItem::Error(node) => origins.push(SurfaceOrigin {
                        kind: SyntaxKind::Error,
                        span: node.source.span,
                    }),
                }
            }
        }
        UntypedDeclaration::Connect(connect) => origins.push(SurfaceOrigin {
            kind: SyntaxKind::ConnectDecl,
            span: connect.span,
        }),
        UntypedDeclaration::Refinement(refinement) => {
            origins.push(SurfaceOrigin {
                kind: SyntaxKind::RefineDecl,
                span: refinement.span,
            });
            for item in &refinement.items {
                match item {
                    UntypedRefinementItem::StateMap(mapping) => origins.push(SurfaceOrigin {
                        kind: SyntaxKind::MapStateDecl,
                        span: mapping.span,
                    }),
                    UntypedRefinementItem::HiddenAction(observation) => {
                        origins.push(SurfaceOrigin {
                            kind: SyntaxKind::HideDecl,
                            span: observation.span,
                        });
                        extend_raw(observation.expression.as_ref(), origins);
                    }
                    UntypedRefinementItem::SurfaceOnly(node) => origins.push(SurfaceOrigin {
                        kind: node.kind,
                        span: node.source.span,
                    }),
                    UntypedRefinementItem::Error(node) => origins.push(SurfaceOrigin {
                        kind: SyntaxKind::Error,
                        span: node.source.span,
                    }),
                }
            }
        }
        UntypedDeclaration::Unsupported(node) => origins.push(SurfaceOrigin {
            kind: node.kind,
            span: node.source.span,
        }),
        UntypedDeclaration::Error(node) => origins.push(SurfaceOrigin {
            kind: SyntaxKind::Error,
            span: node.source.span,
        }),
    }
}

fn census_projected_system(system: &UntypedSystem, origins: &mut Vec<SurfaceOrigin>) {
    origins.push(SurfaceOrigin {
        kind: SyntaxKind::SystemDecl,
        span: system.span,
    });
    for parameter in &system.parameters {
        census_projected_parameter(parameter, origins);
    }
    for member in &system.members {
        match member {
            UntypedMember::Binding(binding) => {
                origins.push(SurfaceOrigin {
                    kind: match binding.kind {
                        BindingKind::Const => SyntaxKind::ConstDecl,
                        BindingKind::Input => SyntaxKind::InputDecl,
                        BindingKind::State => SyntaxKind::StateDecl,
                        BindingKind::Capability => SyntaxKind::CapabilityDecl,
                    },
                    span: binding.span,
                });
                extend_raw(binding.declared_type.as_ref(), origins);
                extend_raw(binding.initializer.as_ref(), origins);
            }
            UntypedMember::Port(port) => {
                origins.push(SurfaceOrigin {
                    kind: SyntaxKind::PortDecl,
                    span: port.span,
                });
                extend_raw(port.declared_type.as_ref(), origins);
            }
            UntypedMember::Action(action) => census_projected_action(action, origins),
            UntypedMember::Property(property) => {
                origins.push(SurfaceOrigin {
                    kind: match property.kind {
                        PropertyKind::Safety => SyntaxKind::SafetyDecl,
                        PropertyKind::Temporal => SyntaxKind::TemporalDecl,
                        PropertyKind::Resource => SyntaxKind::ResourceDecl,
                    },
                    span: property.span,
                });
                extend_raw(property.expression.as_ref(), origins);
            }
            UntypedMember::Observation(observation) => {
                origins.push(SurfaceOrigin {
                    kind: match observation.kind {
                        ObservationKind::Observe => SyntaxKind::ObserveDecl,
                        ObservationKind::Hide => SyntaxKind::HideDecl,
                    },
                    span: observation.span,
                });
                extend_raw(observation.expression.as_ref(), origins);
            }
            UntypedMember::SurfaceOnly(node) => origins.push(SurfaceOrigin {
                kind: node.kind,
                span: node.source.span,
            }),
            UntypedMember::Error(node) => origins.push(SurfaceOrigin {
                kind: SyntaxKind::Error,
                span: node.source.span,
            }),
        }
    }
}

fn census_projected_parameter(item: &UntypedParameterItem, origins: &mut Vec<SurfaceOrigin>) {
    match item {
        UntypedParameterItem::Parameter(parameter) => {
            origins.push(SurfaceOrigin {
                kind: SyntaxKind::Parameter,
                span: parameter.span,
            });
            extend_raw(parameter.declared_type.as_ref(), origins);
        }
        UntypedParameterItem::SurfaceOnly(node) => origins.push(SurfaceOrigin {
            kind: node.kind,
            span: node.source.span,
        }),
        UntypedParameterItem::Error(node) => origins.push(SurfaceOrigin {
            kind: SyntaxKind::Error,
            span: node.source.span,
        }),
    }
}

fn census_projected_action(action: &UntypedAction, origins: &mut Vec<SurfaceOrigin>) {
    origins.push(SurfaceOrigin {
        kind: SyntaxKind::ActionDecl,
        span: action.span,
    });
    for parameter in &action.parameters {
        census_projected_parameter(parameter, origins);
    }
    if let Some(grade) = &action.grade {
        origins.push(SurfaceOrigin {
            kind: SyntaxKind::GradeClause,
            span: grade.span,
        });
    }
    for statement in &action.statements {
        match statement {
            UntypedStatement::Require { condition, span } => {
                origins.push(SurfaceOrigin {
                    kind: SyntaxKind::RequireStmt,
                    span: *span,
                });
                origins.push(condition.origin);
            }
            UntypedStatement::Update {
                target,
                value,
                span,
            } => {
                origins.push(SurfaceOrigin {
                    kind: SyntaxKind::UpdateStmt,
                    span: *span,
                });
                origins.push(match target {
                    UntypedUpdateTarget::Location { origin, .. }
                    | UntypedUpdateTarget::Unsupported { origin, .. } => *origin,
                });
                origins.push(value.origin);
            }
            UntypedStatement::Emit { value, span } => {
                origins.push(SurfaceOrigin {
                    kind: SyntaxKind::EmitStmt,
                    span: *span,
                });
                origins.push(value.origin);
            }
            UntypedStatement::Consume { capability, span } => {
                origins.push(SurfaceOrigin {
                    kind: SyntaxKind::ConsumeStmt,
                    span: *span,
                });
                origins.push(capability.origin);
            }
            UntypedStatement::Rely { fact, span } => {
                origins.push(SurfaceOrigin {
                    kind: SyntaxKind::RelyStmt,
                    span: *span,
                });
                origins.push(fact.origin);
            }
            UntypedStatement::Guarantee { fact, span } => {
                origins.push(SurfaceOrigin {
                    kind: SyntaxKind::GuaranteeStmt,
                    span: *span,
                });
                origins.push(fact.origin);
            }
            UntypedStatement::SurfaceOnly(node) => origins.push(SurfaceOrigin {
                kind: node.kind,
                span: node.source.span,
            }),
            UntypedStatement::Error(node) => origins.push(SurfaceOrigin {
                kind: SyntaxKind::Error,
                span: node.source.span,
            }),
        }
    }
}

fn extend_raw(term: Option<&RawTerm>, origins: &mut Vec<SurfaceOrigin>) {
    if let Some(term) = term {
        origins.push(term.origin);
    }
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

fn action_name_and_polarity(
    node: &GreenNode,
    base: usize,
) -> (Option<SpannedText>, Option<SurfacePolarity>) {
    let first = identifier(node, base, 1);
    let second = identifier(node, base, 2);
    match (first, second) {
        (Some(direction), Some(name)) => {
            if let Some(polarity) = SurfacePolarity::from_direction_text(&direction.text) {
                (Some(name), Some(polarity))
            } else {
                (Some(direction), None)
            }
        }
        (name, _) => (name, None),
    }
}

fn identifier(node: &GreenNode, base: usize, index: usize) -> Option<SpannedText> {
    let mut offset = base;
    let mut seen = 0_usize;
    for child in node.children() {
        if let GreenElement::Token(token) = child {
            if token.kind() == TokenKind::Identifier {
                if seen == index {
                    return Some(SpannedText {
                        text: token.text().to_owned(),
                        span: Span::new(offset, offset + token.text_len()),
                    });
                }
                seen += 1;
            }
        }
        offset += child.text_len();
    }
    None
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
        origin: SurfaceOrigin {
            kind: node.node.kind(),
            span: node.span,
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

fn observation_names(expression: Option<&RawTerm>) -> Vec<SpannedText> {
    expression.map(ident_spans_in_term).unwrap_or_default()
}

fn classify_hide_expression(expression: Option<&RawTerm>) -> (Option<HideSort>, Vec<SpannedText>) {
    let Some(term) = expression else {
        return (Some(HideSort::StateFields), Vec::new());
    };
    let idents = ident_spans_in_term(term);
    if idents.first().map(|name| name.text.as_str()) == Some("action") && idents.len() > 1 {
        (
            Some(HideSort::Actions),
            idents.into_iter().skip(1).collect(),
        )
    } else {
        (Some(HideSort::StateFields), idents)
    }
}

fn ident_spans_in_term(term: &RawTerm) -> Vec<SpannedText> {
    let text = &term.source.text;
    let base = term.source.span.start;
    let bytes = text.as_bytes();
    let mut names = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let ch = bytes[index];
        if ch == b',' || ch.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == b'_' {
            let start = index;
            index += 1;
            while index < bytes.len() {
                let next = bytes[index];
                if next.is_ascii_alphanumeric() || next == b'_' {
                    index += 1;
                } else {
                    break;
                }
            }
            names.push(SpannedText {
                text: text[start..index].to_string(),
                span: Span::new(base + start, base + index),
            });
            continue;
        }
        index += 1;
    }
    names
}

#[cfg(test)]
mod tests {
    use crate::{GreenElement, GreenNode, GreenToken, Span, SyntaxKind, TokenKind, parse_cst};

    use super::{
        NodeAt, ProjectionIssueKind, Projector, UntypedMember, UntypedParameterItem,
        UntypedStatement, UntypedUpdateTarget, project_untyped,
    };

    #[test]
    fn parameter_projection_retains_error_and_surface_only_children_in_order() {
        let parameter = GreenNode::new(
            SyntaxKind::Parameter,
            vec![
                GreenElement::Token(GreenToken::new(TokenKind::Identifier, "ok")),
                GreenElement::Node(GreenNode::new(
                    SyntaxKind::TypeExpr,
                    vec![GreenElement::Token(GreenToken::new(
                        TokenKind::Identifier,
                        "Nat",
                    ))],
                )),
            ],
        );
        let error = GreenNode::new(
            SyntaxKind::Error,
            vec![GreenElement::Token(GreenToken::new(
                TokenKind::Unknown,
                "?",
            ))],
        );
        let future_surface_node = GreenNode::new(
            SyntaxKind::EnumVariant,
            vec![GreenElement::Token(GreenToken::new(
                TokenKind::Identifier,
                "future",
            ))],
        );
        let list = GreenNode::new(
            SyntaxKind::ParameterList,
            vec![
                GreenElement::Node(parameter),
                GreenElement::Node(error),
                GreenElement::Node(future_surface_node),
            ],
        );
        let mut projector = Projector { issues: Vec::new() };

        let items = projector.project_parameters(NodeAt {
            node: &list,
            span: Span::new(0, list.text_len()),
        });

        assert!(matches!(
            items.as_slice(),
            [
                UntypedParameterItem::Parameter(_),
                UntypedParameterItem::Error(_),
                UntypedParameterItem::SurfaceOnly(_),
            ]
        ));
        assert!(
            projector
                .issues
                .iter()
                .any(|issue| matches!(issue.kind, ProjectionIssueKind::RecoveryNode))
        );
        assert!(projector.issues.iter().any(|issue| matches!(
            issue.kind,
            ProjectionIssueKind::UnexpectedProjectedNode {
                kind: SyntaxKind::EnumVariant
            }
        )));
    }

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
        let systems = projection.file.systems();
        assert_eq!(systems.len(), 1);
        assert_eq!(systems[0].name.as_ref().unwrap().text, "Counter");
        let action = systems[0]
            .members
            .iter()
            .find_map(|member| match member {
                UntypedMember::Action(action) => Some(action),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            action
                .supported_parameters()
                .next()
                .unwrap()
                .name
                .as_ref()
                .unwrap()
                .text,
            "by"
        );
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
        let systems = projection.file.systems();
        let UntypedMember::Action(action) = &systems[0].members[0] else {
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
