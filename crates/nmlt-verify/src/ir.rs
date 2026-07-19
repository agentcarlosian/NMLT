use crate::identity::Sha256Id;
use std::collections::BTreeSet;
use std::fmt;

pub const VC_SCHEMA: &str = "nmlt-vc/1";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateRef {
    Current(usize),
    Next(usize),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BoolExpr {
    Const(bool),
    Var(StateRef),
    Not(Box<Self>),
    And(Vec<Self>),
    Or(Vec<Self>),
    Implies(Box<Self>, Box<Self>),
    Iff(Box<Self>, Box<Self>),
}

impl BoolExpr {
    pub fn current(index: usize) -> Self {
        Self::Var(StateRef::Current(index))
    }

    pub fn next(index: usize) -> Self {
        Self::Var(StateRef::Next(index))
    }

    pub fn negate(expr: Self) -> Self {
        Self::Not(Box::new(expr))
    }

    pub fn implies(left: Self, right: Self) -> Self {
        Self::Implies(Box::new(left), Box::new(right))
    }

    pub fn iff(left: Self, right: Self) -> Self {
        Self::Iff(Box::new(left), Box::new(right))
    }

    pub(crate) fn write_canonical(&self, output: &mut String) {
        match self {
            Self::Const(value) => output.push_str(if *value { "true" } else { "false" }),
            Self::Var(StateRef::Current(index)) => output.push_str(&format!("(cur {index})")),
            Self::Var(StateRef::Next(index)) => output.push_str(&format!("(next {index})")),
            Self::Not(expr) => {
                output.push_str("(not ");
                expr.write_canonical(output);
                output.push(')');
            }
            Self::And(items) | Self::Or(items) => {
                output.push('(');
                output.push_str(if matches!(self, Self::And(_)) {
                    "and"
                } else {
                    "or"
                });
                for item in items {
                    output.push(' ');
                    item.write_canonical(output);
                }
                output.push(')');
            }
            Self::Implies(left, right) | Self::Iff(left, right) => {
                output.push('(');
                output.push_str(if matches!(self, Self::Implies(..)) {
                    "implies"
                } else {
                    "iff"
                });
                output.push(' ');
                left.write_canonical(output);
                output.push(' ');
                right.write_canonical(output);
                output.push(')');
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationIdentity {
    pub model: Sha256Id,
    pub claim: Sha256Id,
    pub configuration: Sha256Id,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationConfig {
    pub finite_domain: bool,
    pub terminal_stutter: bool,
    pub assumptions: Vec<String>,
}

impl VerificationConfig {
    pub fn identity(&self) -> Sha256Id {
        let mut canonical = format!(
            "nmlt-verification-config/1\nfinite_domain={}\nterminal_stutter={}\n",
            self.finite_domain, self.terminal_stutter
        );
        let mut assumptions = self.assumptions.clone();
        assumptions.sort();
        assumptions.dedup();
        for assumption in assumptions {
            canonical.push_str(&format!("assumption:{}:{}\n", assumption.len(), assumption));
        }
        Sha256Id::digest(canonical.as_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiniteSafetyVc {
    pub identity: VerificationIdentity,
    pub config: VerificationConfig,
    pub variables: Vec<String>,
    pub initial: BoolExpr,
    pub transition: BoolExpr,
    pub property: BoolExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrError {
    EmptyVariables,
    TooManyVariables(usize),
    DuplicateVariable(String),
    InvalidVariable(String),
    IndexOutOfRange {
        index: usize,
        variables: usize,
    },
    NextStateInInitial,
    NextStateInProperty,
    ConfigurationIdentityMismatch {
        expected: Sha256Id,
        actual: Sha256Id,
    },
}

impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyVariables => write!(f, "a finite verification condition needs a variable"),
            Self::TooManyVariables(count) => write!(
                f,
                "{count} Boolean variables exceed the exact-enumeration limit of 20"
            ),
            Self::DuplicateVariable(name) => write!(f, "duplicate variable {name:?}"),
            Self::InvalidVariable(name) => write!(f, "invalid variable name {name:?}"),
            Self::IndexOutOfRange { index, variables } => write!(
                f,
                "expression variable index {index} is outside 0..{variables}"
            ),
            Self::NextStateInInitial => write!(f, "initial formula contains a next-state variable"),
            Self::NextStateInProperty => write!(f, "property contains a next-state variable"),
            Self::ConfigurationIdentityMismatch { expected, actual } => write!(
                f,
                "configuration identity is stale: expected {expected}, supplied {actual}"
            ),
        }
    }
}

impl std::error::Error for IrError {}

impl FiniteSafetyVc {
    pub fn validate(&self) -> Result<(), IrError> {
        if self.variables.is_empty() {
            return Err(IrError::EmptyVariables);
        }
        if self.variables.len() > 20 {
            return Err(IrError::TooManyVariables(self.variables.len()));
        }
        let mut names = BTreeSet::new();
        for name in &self.variables {
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            {
                return Err(IrError::InvalidVariable(name.clone()));
            }
            if !names.insert(name) {
                return Err(IrError::DuplicateVariable(name.clone()));
            }
        }
        validate_expr(
            &self.initial,
            self.variables.len(),
            false,
            IrError::NextStateInInitial,
        )?;
        validate_expr(
            &self.transition,
            self.variables.len(),
            true,
            IrError::NextStateInInitial,
        )?;
        validate_expr(
            &self.property,
            self.variables.len(),
            false,
            IrError::NextStateInProperty,
        )?;
        let expected = self.config.identity();
        if expected != self.identity.configuration {
            return Err(IrError::ConfigurationIdentityMismatch {
                expected,
                actual: self.identity.configuration.clone(),
            });
        }
        Ok(())
    }

    /// Canonical identity of the complete verification condition. Length prefixes
    /// make this serialization unambiguous; caller-supplied identities are inputs.
    pub fn digest(&self) -> Result<Sha256Id, IrError> {
        self.validate()?;
        let mut canonical = String::new();
        canonical.push_str(VC_SCHEMA);
        canonical.push('\n');
        for (key, value) in [
            ("model", self.identity.model.as_str()),
            ("claim", self.identity.claim.as_str()),
            ("configuration", self.identity.configuration.as_str()),
        ] {
            canonical.push_str(&format!("{key}:{}:{value}\n", value.len()));
        }
        for variable in &self.variables {
            canonical.push_str(&format!("var:{}:{variable}\n", variable.len()));
        }
        for (label, expression) in [
            ("initial", &self.initial),
            ("transition", &self.transition),
            ("property", &self.property),
        ] {
            canonical.push_str(label);
            canonical.push(':');
            expression.write_canonical(&mut canonical);
            canonical.push('\n');
        }
        Ok(Sha256Id::digest(canonical.as_bytes()))
    }

    pub fn state_count(&self) -> usize {
        1_usize << self.variables.len()
    }
}

fn validate_expr(
    expression: &BoolExpr,
    variables: usize,
    allow_next: bool,
    next_error: IrError,
) -> Result<(), IrError> {
    match expression {
        BoolExpr::Const(_) => Ok(()),
        BoolExpr::Var(StateRef::Current(index)) => {
            if *index < variables {
                Ok(())
            } else {
                Err(IrError::IndexOutOfRange {
                    index: *index,
                    variables,
                })
            }
        }
        BoolExpr::Var(StateRef::Next(index)) => {
            if *index >= variables {
                Err(IrError::IndexOutOfRange {
                    index: *index,
                    variables,
                })
            } else if allow_next {
                Ok(())
            } else {
                Err(next_error)
            }
        }
        BoolExpr::Not(item) => validate_expr(item, variables, allow_next, next_error),
        BoolExpr::And(items) | BoolExpr::Or(items) => {
            for item in items {
                validate_expr(item, variables, allow_next, next_error.clone())?;
            }
            Ok(())
        }
        BoolExpr::Implies(left, right) | BoolExpr::Iff(left, right) => {
            validate_expr(left, variables, allow_next, next_error.clone())?;
            validate_expr(right, variables, allow_next, next_error)
        }
    }
}

pub(crate) fn eval(expression: &BoolExpr, current: usize, next: usize) -> bool {
    match expression {
        BoolExpr::Const(value) => *value,
        BoolExpr::Var(StateRef::Current(index)) => current & (1 << index) != 0,
        BoolExpr::Var(StateRef::Next(index)) => next & (1 << index) != 0,
        BoolExpr::Not(item) => !eval(item, current, next),
        BoolExpr::And(items) => items.iter().all(|item| eval(item, current, next)),
        BoolExpr::Or(items) => items.iter().any(|item| eval(item, current, next)),
        BoolExpr::Implies(left, right) => !eval(left, current, next) || eval(right, current, next),
        BoolExpr::Iff(left, right) => eval(left, current, next) == eval(right, current, next),
    }
}
