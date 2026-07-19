use std::fmt;
use std::sync::Arc;

use crate::{Span, TokenKind};

/// The structural categories represented by the lossless concrete syntax tree.
///
/// These kinds deliberately describe syntax, not resolved or typed meanings.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SyntaxKind {
    SourceFile,
    ModuleDecl,
    ImportDecl,
    DataDecl,
    TypeDecl,
    RecordDecl,
    FunctionDecl,
    EnumDecl,
    EnumVariant,
    SystemDecl,
    SystemBody,
    ParameterList,
    Parameter,
    ConstDecl,
    InputDecl,
    StateDecl,
    CapabilityDecl,
    PortDecl,
    ActionDecl,
    GradeClause,
    ActionBody,
    RequireStmt,
    UpdateStmt,
    EmitStmt,
    ConsumeStmt,
    SafetyDecl,
    TemporalDecl,
    ResourceDecl,
    ObserveDecl,
    HideDecl,
    TypeExpr,
    Expr,
    Error,
}

/// An immutable token leaf. Its text is owned by the green tree so the tree can
/// reconstruct the source without retaining the parser's input buffer.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct GreenToken {
    kind: TokenKind,
    text: Arc<str>,
}

impl GreenToken {
    pub(crate) fn new(kind: TokenKind, text: &str) -> Self {
        Self {
            kind,
            text: Arc::from(text),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> TokenKind {
        self.kind
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn text_len(&self) -> usize {
        self.text.len()
    }
}

impl fmt::Debug for GreenToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GreenToken")
            .field("kind", &self.kind)
            .field("text", &self.text)
            .finish()
    }
}

/// One immutable child of a green node.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum GreenElement {
    Node(GreenNode),
    Token(GreenToken),
}

impl GreenElement {
    #[must_use]
    pub fn text_len(&self) -> usize {
        match self {
            Self::Node(node) => node.text_len(),
            Self::Token(token) => token.text_len(),
        }
    }

    #[must_use]
    pub const fn as_node(&self) -> Option<&GreenNode> {
        match self {
            Self::Node(node) => Some(node),
            Self::Token(_) => None,
        }
    }

    #[must_use]
    pub const fn as_token(&self) -> Option<&GreenToken> {
        match self {
            Self::Node(_) => None,
            Self::Token(token) => Some(token),
        }
    }
}

#[derive(Eq, Hash, PartialEq)]
struct GreenNodeData {
    kind: SyntaxKind,
    text_len: usize,
    children: Arc<[GreenElement]>,
}

/// A cheap-to-clone immutable green node.
///
/// Nodes have no parent or absolute offset. Consumers derive byte offsets while
/// walking from the root, which keeps subtrees shareable.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct GreenNode(Arc<GreenNodeData>);

impl GreenNode {
    pub(crate) fn new(kind: SyntaxKind, children: Vec<GreenElement>) -> Self {
        let text_len = children.iter().map(GreenElement::text_len).sum();
        Self(Arc::new(GreenNodeData {
            kind,
            text_len,
            children: children.into(),
        }))
    }

    #[must_use]
    pub fn kind(&self) -> SyntaxKind {
        self.0.kind
    }

    #[must_use]
    pub fn text_len(&self) -> usize {
        self.0.text_len
    }

    #[must_use]
    pub fn children(&self) -> &[GreenElement] {
        &self.0.children
    }

    /// Reconstruct the exact source bytes represented by this subtree.
    #[must_use]
    pub fn reconstruct(&self) -> String {
        let mut output = String::with_capacity(self.text_len());
        self.write_to(&mut output);
        output
    }

    /// Return all token leaves with offsets derived from their tree position.
    #[must_use]
    pub fn tokens_with_spans(&self) -> Vec<SpannedGreenToken<'_>> {
        let mut tokens = Vec::new();
        collect_tokens(self, 0, &mut tokens);
        tokens
    }

    /// Build ephemeral red-style views with parent and absolute-offset context.
    #[must_use]
    pub fn nodes_with_spans(&self) -> Vec<SpannedGreenNode<'_>> {
        let mut nodes = Vec::new();
        collect_nodes(self, None, 0, &mut nodes);
        nodes
    }

    /// Find descendant nodes of a particular structural kind.
    #[must_use]
    pub fn descendants(&self, kind: SyntaxKind) -> Vec<&GreenNode> {
        let mut nodes = Vec::new();
        collect_descendants(self, kind, &mut nodes);
        nodes
    }

    fn write_to(&self, output: &mut String) {
        for child in self.children() {
            match child {
                GreenElement::Node(node) => node.write_to(output),
                GreenElement::Token(token) => output.push_str(token.text()),
            }
        }
    }
}

impl fmt::Debug for GreenNode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GreenNode")
            .field("kind", &self.kind())
            .field("text_len", &self.text_len())
            .field("children", &self.children())
            .finish()
    }
}

/// A borrowed green token with its absolute half-open byte span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpannedGreenToken<'tree> {
    pub token: &'tree GreenToken,
    pub span: Span,
}

/// An ephemeral red-style node view over immutable green data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpannedGreenNode<'tree> {
    pub node: &'tree GreenNode,
    pub parent: Option<&'tree GreenNode>,
    pub span: Span,
}

fn collect_tokens<'tree>(
    node: &'tree GreenNode,
    mut offset: usize,
    output: &mut Vec<SpannedGreenToken<'tree>>,
) -> usize {
    for child in node.children() {
        match child {
            GreenElement::Node(child_node) => {
                offset = collect_tokens(child_node, offset, output);
            }
            GreenElement::Token(token) => {
                let end = offset + token.text_len();
                output.push(SpannedGreenToken {
                    token,
                    span: Span::new(offset, end),
                });
                offset = end;
            }
        }
    }
    offset
}

fn collect_nodes<'tree>(
    node: &'tree GreenNode,
    parent: Option<&'tree GreenNode>,
    offset: usize,
    output: &mut Vec<SpannedGreenNode<'tree>>,
) {
    output.push(SpannedGreenNode {
        node,
        parent,
        span: Span::new(offset, offset + node.text_len()),
    });
    let mut child_offset = offset;
    for child in node.children() {
        if let GreenElement::Node(child_node) = child {
            collect_nodes(child_node, Some(node), child_offset, output);
        }
        child_offset += child.text_len();
    }
}

fn collect_descendants<'tree>(
    node: &'tree GreenNode,
    kind: SyntaxKind,
    output: &mut Vec<&'tree GreenNode>,
) {
    for child in node.children() {
        let GreenElement::Node(child_node) = child else {
            continue;
        };
        if child_node.kind() == kind {
            output.push(child_node);
        }
        collect_descendants(child_node, kind, output);
    }
}

#[cfg(test)]
mod tests {
    use super::{GreenElement, GreenNode, GreenToken, SyntaxKind};
    use crate::TokenKind;

    #[test]
    fn immutable_tree_reconstructs_and_derives_offsets() {
        let child = GreenNode::new(
            SyntaxKind::SystemDecl,
            vec![GreenElement::Token(GreenToken::new(
                TokenKind::Identifier,
                "system",
            ))],
        );
        let root = GreenNode::new(
            SyntaxKind::SourceFile,
            vec![
                GreenElement::Node(child.clone()),
                GreenElement::Token(GreenToken::new(TokenKind::Whitespace, "\n")),
            ],
        );

        assert_eq!(root.reconstruct(), "system\n");
        assert_eq!(root.text_len(), 7);
        assert_eq!(root.tokens_with_spans()[0].span, crate::Span::new(0, 6));
        assert_eq!(root.descendants(SyntaxKind::SystemDecl), vec![&child]);
        let nodes = root.nodes_with_spans();
        assert_eq!(nodes[0].span, crate::Span::new(0, 7));
        assert_eq!(nodes[1].span, crate::Span::new(0, 6));
        assert_eq!(nodes[1].parent, Some(&root));
    }
}
