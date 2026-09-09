//! A deterministic adapter prototype. Its exact arithmetic output is checked
//! independently of accepting the response envelope. It executes no Lean jobs.

use crate::{
    Adapter, Dispatch, Error, RESPONSE_SCHEMA, Response, ResponseOutcome, Value, ValueType, sha256,
};

#[must_use]
pub fn adapter() -> Adapter {
    Adapter {
        name: "local-square".into(),
        version: 1,
        input_type: ValueType::Int,
        output_type: ValueType::Int,
    }
}

pub fn evaluate(dispatch: &Dispatch) -> Result<Response, Error> {
    if dispatch.binding.adapter != adapter()
        || dispatch.binding.input_sha256 != sha256(&serde_json::to_vec(&dispatch.input)?)
    {
        return Err(Error(
            "square adapter contract or input identity mismatch".into(),
        ));
    }
    let Value::Int(value) = dispatch.input else {
        return Err(Error("square adapter expects Int".into()));
    };
    let outcome = if value < 0 {
        ResponseOutcome::Failed {
            message: "negative input".into(),
        }
    } else if let Some(square) = value.checked_mul(value) {
        ResponseOutcome::Completed {
            value: Value::Int(square),
        }
    } else {
        ResponseOutcome::Failed {
            message: "square exceeds Int range".into(),
        }
    };
    Ok(Response {
        schema: RESPONSE_SCHEMA.into(),
        binding: dispatch.binding.clone(),
        outcome,
        observed_work: Some(1),
    })
}

pub fn validate(dispatch: &Dispatch, response: &Response) -> Result<(), Error> {
    if &evaluate(dispatch)? != response {
        return Err(Error(
            "square adapter result failed its exact output contract".into(),
        ));
    }
    Ok(())
}
