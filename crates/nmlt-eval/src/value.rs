use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use nmlt_ir::CoreBehaviorTerm;

use crate::{EvalState, qualify};

/// Values in the supported finite behavior slice. Enumeration identity includes
/// its type, so constructors from different declarations remain distinct.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EvalValue {
    Bool(bool),
    Unit,
    Enum {
        type_name: String,
        constructor: String,
    },
}

impl EvalValue {
    #[must_use]
    pub fn type_name(&self) -> &str {
        match self {
            Self::Bool(_) => "Bool",
            Self::Unit => "Unit",
            Self::Enum { type_name, .. } => type_name,
        }
    }
}

impl fmt::Display for EvalValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(value) => value.fmt(formatter),
            Self::Unit => formatter.write_str("()"),
            Self::Enum {
                type_name,
                constructor,
            } => write!(formatter, "{type_name}.{constructor}"),
        }
    }
}

pub(crate) fn eval_value(
    term: &CoreBehaviorTerm,
    state: &EvalState,
    system: &str,
    enums: &BTreeMap<String, BTreeSet<String>>,
) -> Option<EvalValue> {
    let value = match term {
        CoreBehaviorTerm::Bool { value, .. } => EvalValue::Bool(*value),
        CoreBehaviorTerm::Unit { .. } => EvalValue::Unit,
        CoreBehaviorTerm::Enum {
            r#type,
            constructor,
        } => {
            if matches!(r#type.as_str(), "Bool" | "Unit")
                || !enums.get(r#type)?.contains(constructor)
            {
                return None;
            }
            EvalValue::Enum {
                type_name: r#type.clone(),
                constructor: constructor.clone(),
            }
        }
        CoreBehaviorTerm::Read { field, .. } => {
            state.values.get(&qualify(state, system, field))?.clone()
        }
        CoreBehaviorTerm::Not { value, .. } => {
            let EvalValue::Bool(value) = eval_value(value, state, system, enums)? else {
                return None;
            };
            EvalValue::Bool(!value)
        }
        CoreBehaviorTerm::Equal { left, right, .. } => {
            let left = eval_value(left, state, system, enums)?;
            let right = eval_value(right, state, system, enums)?;
            if left.type_name() != right.type_name() {
                return None;
            }
            EvalValue::Bool(left == right)
        }
    };
    (value.type_name() == term.ty()).then_some(value)
}
