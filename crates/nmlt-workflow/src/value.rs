use crate::*;

/// Bound value traversal before structural validation or copying. The unit is
/// one node plus UTF-8 payload/name bytes, not a host memory measurement.
pub(crate) fn units(value: &Value) -> Option<usize> {
    fn visit(v: &Value, depth: usize, nodes: &mut usize, bytes: &mut usize) -> Option<()> {
        if depth > 16 {
            return None;
        }
        *nodes += 1;
        if *nodes > MAX_VALUE_NODES {
            return None;
        }
        match v {
            Value::Text(s) | Value::Err(s) => {
                if s.len() > 4096 {
                    return None;
                }
                *bytes = bytes.checked_add(s.len())?;
            }
            Value::Ok(v) => visit(v, depth + 1, nodes, bytes)?,
            Value::List(items) => {
                if items.len() > MAX_LIST_ITEMS {
                    return None;
                }
                for v in items {
                    visit(v, depth + 1, nodes, bytes)?;
                }
            }
            Value::Record { name, fields } => {
                if fields.len() > 16 {
                    return None;
                }
                *bytes = bytes.checked_add(name.len())?;
                for (name, v) in fields {
                    *bytes = bytes.checked_add(name.len())?;
                    if *bytes > MAX_VALUE_BYTES {
                        return None;
                    }
                    visit(v, depth + 1, nodes, bytes)?;
                }
            }
            Value::Bool(_) | Value::Int(_) => {}
        }
        (*bytes <= MAX_VALUE_BYTES).then_some(())
    }
    let (mut nodes, mut bytes) = (0, 0);
    visit(value, 0, &mut nodes, &mut bytes)?;
    Some(nodes + bytes)
}

pub(crate) fn conforms(
    value: &Value,
    ty: &Type,
    records: &BTreeMap<String, Vec<Parameter>>,
) -> bool {
    fn check(v: &Value, ty: &Type, records: &BTreeMap<String, Vec<Parameter>>) -> bool {
        match (v, ty) {
            (Value::Bool(_), Type::Bool)
            | (Value::Int(_), Type::Int)
            | (Value::Text(_), Type::Text)
            | (Value::Err(_), Type::Outcome(_)) => true,
            (Value::Ok(v), Type::Outcome(t)) => check(v, t, records),
            (Value::List(items), Type::List(t)) => items.iter().all(|v| check(v, t, records)),
            (Value::Record { name, fields }, Type::Record(expected)) if name == expected => {
                records.get(name).is_some_and(|definition| {
                    fields.len() == definition.len()
                        && definition.iter().all(|f| {
                            fields
                                .get(&f.name)
                                .is_some_and(|v| check(v, &f.ty, records))
                        })
                })
            }
            _ => false,
        }
    }
    units(value).is_some() && check(value, ty, records)
}

pub(crate) fn decode(
    program: &Program,
    ty: &Type,
    json: &serde_json::Value,
) -> Result<Value, String> {
    fn read(
        p: &Program,
        ty: &Type,
        j: &serde_json::Value,
        depth: usize,
        nodes: &mut usize,
        bytes: &mut usize,
    ) -> Result<Value, String> {
        use serde_json::Value as J;
        if depth > 16 {
            return Err("input nesting exceeds sixteen".into());
        }
        *nodes += 1;
        *bytes += match j {
            J::String(s) => s.len(),
            _ => 0,
        };
        if let Type::Record(name) = ty {
            *bytes += name.len();
            if let J::Object(fields) = j {
                *bytes += fields.keys().map(String::len).sum::<usize>();
            }
        }
        // Err is represented by a single Value node, but its JSON payload still costs bytes.
        if let (Type::Outcome(_), J::Object(fields)) = (ty, j)
            && let Some(J::String(s)) = fields.get("Err")
        {
            *bytes += s.len();
        }
        if *nodes > MAX_VALUE_NODES || *bytes > MAX_VALUE_BYTES {
            return Err("input exceeds aggregate value bounds".into());
        }
        Ok(match (ty, j) {
            (Type::Bool, J::Bool(v)) => Value::Bool(*v),
            (Type::Int, J::Number(v)) => Value::Int(v.as_i64().ok_or("input Int must fit i64")?),
            (Type::Text, J::String(v)) if v.len() <= 4096 => Value::Text(v.clone()),
            (Type::List(t), J::Array(items)) if items.len() <= MAX_LIST_ITEMS => Value::List(
                items
                    .iter()
                    .map(|v| read(p, t, v, depth + 1, nodes, bytes))
                    .collect::<Result<_, _>>()?,
            ),
            (Type::Record(name), J::Object(fields)) => {
                let definition = p.records.get(name).ok_or("unknown input record type")?;
                if fields.len() != definition.len() {
                    return Err("record input requires exactly its declared fields".into());
                }
                let mut result = BTreeMap::new();
                for f in definition {
                    let j = fields
                        .get(&f.name)
                        .ok_or_else(|| format!("missing record field `{}`", f.name))?;
                    result.insert(f.name.clone(), read(p, &f.ty, j, depth + 1, nodes, bytes)?);
                }
                Value::Record {
                    name: name.clone(),
                    fields: result,
                }
            }
            (Type::Outcome(t), J::Object(fields)) if fields.len() == 1 => {
                if let Some(j) = fields.get("Ok") {
                    Value::Ok(Box::new(read(p, t, j, depth + 1, nodes, bytes)?))
                } else if let Some(J::String(s)) = fields.get("Err") {
                    if s.len() > 4096 {
                        return Err("error text exceeds 4096 bytes".into());
                    }
                    Value::Err(s.clone())
                } else {
                    return Err("Outcome input requires exactly Ok or Err".into());
                }
            }
            _ => return Err(format!("input must match {ty:?} and its value bounds")),
        })
    }
    let value = read(program, ty, json, 0, &mut 0, &mut 0)?;
    if !program.conforms(&value, ty) {
        return Err("input exceeds aggregate value bounds".into());
    }
    Ok(value)
}
