use serde::{Deserialize, Serialize};

use crate::{Error, sha256, valid_digest, valid_name};

pub const RESPONSE_SCHEMA: &str = "nmlt-job-response-v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Bool,
    Int,
    Text,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Text(String),
}

impl Value {
    #[must_use]
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Bool(_) => ValueType::Bool,
            Self::Int(_) => ValueType::Int,
            Self::Text(_) => ValueType::Text,
        }
    }
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if matches!(self, Self::Text(text) if text.len() > 4096) {
            return Err(Error("text exceeds 4096-byte protocol bound".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Adapter {
    pub name: String,
    pub version: u32,
    pub input_type: ValueType,
    pub output_type: ValueType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub adapter: Adapter,
    pub context_sha256: String,
    pub input: Value,
    pub reserved_work: u64,
}

impl Request {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if !valid_name(&self.adapter.name) || self.adapter.version != 1 {
            return Err(Error(
                "invalid adapter name or unsupported adapter version".into(),
            ));
        }
        if !valid_digest(&self.context_sha256) || self.input.value_type() != self.adapter.input_type
        {
            return Err(Error("invalid request context or input type".into()));
        }
        if self.reserved_work == 0 {
            return Err(Error("work reservation must be positive".into()));
        }
        self.input.validate()
    }
    pub(crate) fn input_digest(&self) -> Result<String, Error> {
        Ok(sha256(&serde_json::to_vec(&self.input)?))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptId {
    pub run: String,
    pub task: String,
    pub slot: u32,
    pub generation: u32,
}

/// A revision is consumed on each control transition. This is runtime affinity,
/// not authentication of an OS principal or a secret bearer capability.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    pub attempt: AttemptId,
    pub owner: String,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub attempt: AttemptId,
    pub adapter: Adapter,
    pub dispatched_by: String,
    pub context_sha256: String,
    pub input_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dispatch {
    pub binding: Binding,
    pub input: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResponseOutcome {
    Completed { value: Value },
    Failed { message: String },
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub schema: String,
    pub binding: Binding,
    pub outcome: ResponseOutcome,
    /// Adapter-reported work. None stays unknown, including after cancellation.
    pub observed_work: Option<u64>,
}

impl Response {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.schema != RESPONSE_SCHEMA || self.binding.adapter.version != 1 {
            return Err(Error(
                "unsupported job response schema or adapter version".into(),
            ));
        }
        match &self.outcome {
            ResponseOutcome::Completed { value } => value.validate()?,
            ResponseOutcome::Failed { message } if message.is_empty() || message.len() > 4096 => {
                return Err(Error("invalid failure message".into()));
            }
            _ => {}
        }
        Ok(())
    }
}
