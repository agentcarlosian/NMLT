use nmlt_core::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
    Bool,
    Nat,
    Int,
    Named(String),
    /// An unqualified constructor such as `authorized`, resolved by context.
    Symbol,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Symbol(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOp {
    Not,
    Negate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    Implies,
    Or,
    And,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Add,
    Subtract,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    Value(Value),
    Name(String),
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        arguments: Vec<Expr>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateVar {
    pub name: String,
    pub ty: Type,
    pub initial: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Update {
    pub target: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Action {
    pub name: String,
    pub guards: Vec<Expr>,
    pub updates: Vec<Update>,
    pub consumes: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropertyKind {
    Safety,
    Temporal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Property {
    pub name: String,
    pub kind: PropertyKind,
    pub expression: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Model {
    pub system_name: String,
    pub states: Vec<StateVar>,
    pub capabilities: Vec<String>,
    pub actions: Vec<Action>,
    pub properties: Vec<Property>,
    pub observations: Vec<String>,
    pub span: Span,
}
