use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobError {
    Limit,
    HostFailure,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "input",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum JobRequest {
    Square(i64),
    Lean(String),
    LeanCheck { statement: String, proof: String },
    LeanProject { alias: String, proof: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobOperation {
    Poll,
    Cancel,
    Collect,
}

/// Explicit embedding boundary. Implementations own scheduling, durability,
/// response validation, and limits. Job controls never enter workflow values.
pub trait JobHost {
    fn square(&mut self, input: i64, at: Location) -> Result<Result<i64, String>, JobError>;
    /// IDs are host-local references, never source values or live runtime controls.
    fn start(&mut self, _: JobRequest, _: Location) -> Result<u32, JobError> {
        Err(JobError::HostFailure)
    }
    fn control(&mut self, _: u32, _: JobOperation, _: Location) -> Result<Value, JobError> {
        Err(JobError::HostFailure)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Stop {
    Returned { value: Value },
    StepLimit { at: Location },
    DepthLimit { at: Location },
    IntegerOverflow { at: Location },
    ValueLimit { at: Location },
    ValueWorkLimit { at: Location },
    JobStopped { at: Location, reason: JobError },
}
impl Stop {
    pub fn returned(&self) -> bool {
        matches!(self, Self::Returned { .. })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Execution {
    pub steps: u32,
    pub stop: Stop,
}

/// Deterministic, pure execution. One step is entry to a typed expression node.
/// A returned Err is an ordinary domain outcome, not an execution failure.
pub fn execute(
    program: &Program,
    entry: &str,
    inputs: &Inputs,
    max_steps: u32,
) -> Result<Execution, String> {
    if program.requires_jobs(entry)? {
        return Err("entry requires local jobs; use an explicit bounded job host".into());
    }
    struct NoJobs;
    impl JobHost for NoJobs {
        fn square(&mut self, _: i64, _: Location) -> Result<Result<i64, String>, JobError> {
            Err(JobError::HostFailure)
        }
    }
    execute_with_host(program, entry, inputs, max_steps, &mut NoJobs)
}

/// Validate all entry inputs and execution limits before creating host state.
pub fn validate_inputs(
    program: &Program,
    entry: &str,
    inputs: &Inputs,
    max_steps: u32,
) -> Result<(), String> {
    prepare(program, entry, inputs, max_steps).map(|_| ())
}

fn prepare<'a>(
    program: &'a Program,
    entry: &str,
    inputs: &Inputs,
    max_steps: u32,
) -> Result<(&'a Function, Vec<Binding>, usize), String> {
    if !(1..=100_000).contains(&max_steps) {
        return Err("max-steps must be in 1..100000".into());
    }
    let function = program
        .functions
        .iter()
        .find(|f| f.name == entry)
        .ok_or_else(|| format!("unknown workflow entry `{entry}`"))?;
    if matches!(function.result, Type::Job(_))
        || function
            .parameters
            .iter()
            .any(|p| matches!(p.ty, Type::Job(_)))
    {
        return Err("Job parameters/results require an enclosing workflow; handles cannot cross an external entry boundary".into());
    }
    if inputs.len() != function.parameters.len() {
        return Err("entry requires exactly its declared named inputs".into());
    }
    let mut env = vec![];
    let mut value_work = 0;
    for parameter in &function.parameters {
        let value = inputs
            .get(&parameter.name)
            .ok_or_else(|| format!("missing input `{}`", parameter.name))?;
        if !program.conforms(value, &parameter.ty) {
            return Err(format!(
                "input `{}` must have type {:?} and satisfy its value bounds",
                parameter.name, parameter.ty
            ));
        }
        value_work += value::units(value).expect("validated input");
        if value_work > MAX_VALUE_WORK {
            return Err("inputs exceed total value-work bound".into());
        }
        env.push(Binding::Data(value.clone()));
    }
    Ok((function, env, value_work))
}

pub fn execute_with_host(
    program: &Program,
    entry: &str,
    inputs: &Inputs,
    max_steps: u32,
    host: &mut dyn JobHost,
) -> Result<Execution, String> {
    let (function, mut env, value_work) = prepare(program, entry, inputs, max_steps)?;
    let mut machine = Machine {
        program,
        steps: 0,
        max_steps,
        value_work,
        host,
    };
    let stop = match machine.term(&function.body, &mut env, 0) {
        Ok(StackValue::Data(value)) => Stop::Returned { value },
        Ok(StackValue::Job(_)) => Stop::JobStopped {
            at: function.body.span,
            reason: JobError::HostFailure,
        },
        Err(stop) => stop,
    };
    Ok(Execution {
        steps: machine.steps,
        stop,
    })
}

struct Machine<'a> {
    program: &'a Program,
    steps: u32,
    max_steps: u32,
    value_work: usize,
    host: &'a mut dyn JobHost,
}
enum Binding {
    Data(Value),
    Job(Option<u32>),
}
enum StackValue {
    Data(Value),
    Job(u32),
}
impl From<StackValue> for Binding {
    fn from(value: StackValue) -> Self {
        match value {
            StackValue::Data(v) => Self::Data(v),
            StackValue::Job(id) => Self::Job(Some(id)),
        }
    }
}
impl Machine<'_> {
    fn data(&mut self, node: &Typed, env: &mut Vec<Binding>, depth: usize) -> Result<Value, Stop> {
        match self.term(node, env, depth)? {
            StackValue::Data(value) => Ok(value),
            StackValue::Job(_) => unreachable!("typed data context"),
        }
    }
    fn finish(&mut self, value: StackValue, at: Location) -> Result<StackValue, Stop> {
        match value {
            StackValue::Data(value) => self.account(value, at).map(StackValue::Data),
            job => Ok(job),
        }
    }
    fn term(
        &mut self,
        node: &Typed,
        env: &mut Vec<Binding>,
        depth: usize,
    ) -> Result<StackValue, Stop> {
        if self.steps == self.max_steps {
            return Err(Stop::StepLimit { at: node.span });
        }
        if depth == 64 {
            return Err(Stop::DepthLimit { at: node.span });
        }
        self.steps += 1;
        let overflow = || Stop::IntegerOverflow { at: node.span };
        let value = match &node.kind {
            TypedKind::JobLeanCheck(statement, proof)
            | TypedKind::JobLeanProject(statement, proof) => {
                let Value::Text(statement) = self.data(statement, env, depth + 1)? else {
                    unreachable!()
                };
                let Value::Text(proof) = self.data(proof, env, depth + 1)? else {
                    unreachable!()
                };
                let request = if matches!(&node.kind, TypedKind::JobLeanProject(..)) {
                    JobRequest::LeanProject {
                        alias: statement,
                        proof,
                    }
                } else {
                    JobRequest::LeanCheck { statement, proof }
                };
                let id =
                    self.host
                        .start(request, node.span)
                        .map_err(|reason| Stop::JobStopped {
                            at: node.span,
                            reason,
                        })?;
                return Ok(StackValue::Job(id));
            }
            TypedKind::JobStart(input, lean) => {
                let input = self.data(input, env, depth + 1)?;
                let request = match (lean, input) {
                    (false, Value::Int(i)) => JobRequest::Square(i),
                    (true, Value::Text(s)) => JobRequest::Lean(s),
                    _ => unreachable!(),
                };
                let id =
                    self.host
                        .start(request, node.span)
                        .map_err(|reason| Stop::JobStopped {
                            at: node.span,
                            reason,
                        })?;
                return Ok(StackValue::Job(id));
            }
            TypedKind::JobControl(op, index) => {
                let Binding::Job(id) = &mut env[*index] else {
                    unreachable!()
                };
                let job = if *op == JobOperation::Collect {
                    id.take()
                } else {
                    *id
                }
                .ok_or(Stop::JobStopped {
                    at: node.span,
                    reason: JobError::HostFailure,
                })?;
                let result =
                    self.host
                        .control(job, *op, node.span)
                        .map_err(|reason| Stop::JobStopped {
                            at: node.span,
                            reason,
                        })?;
                if !self.program.conforms(&result, &node.ty) {
                    return Err(Stop::JobStopped {
                        at: node.span,
                        reason: JobError::HostFailure,
                    });
                }
                result
            }
            TypedKind::JobSquare(input) => {
                let Value::Int(input) = self.data(input, env, depth + 1)? else {
                    unreachable!()
                };
                match self
                    .host
                    .square(input, node.span)
                    .map_err(|reason| Stop::JobStopped {
                        at: node.span,
                        reason,
                    })? {
                    Ok(value) => Value::Ok(Box::new(Value::Int(value))),
                    Err(message) => Value::Err(message),
                }
            }
            TypedKind::Literal(value) => value.clone(),
            TypedKind::Local(index) => match &mut env[*index] {
                Binding::Data(value) => value.clone(),
                Binding::Job(id) => {
                    return Ok(StackValue::Job(id.take().ok_or(Stop::JobStopped {
                        at: node.span,
                        reason: JobError::HostFailure,
                    })?));
                }
            },
            TypedKind::Record(name, fields) => {
                let mut result = BTreeMap::new();
                for (field, value) in fields {
                    result.insert(field.clone(), self.data(value, env, depth + 1)?);
                }
                Value::Record {
                    name: name.clone(),
                    fields: result,
                }
            }
            TypedKind::Field(value, field) => {
                let Value::Record { mut fields, .. } = self.data(value, env, depth + 1)? else {
                    unreachable!()
                };
                fields.remove(field).expect("typed field")
            }
            TypedKind::List(items) => Value::List(
                items
                    .iter()
                    .map(|v| self.data(v, env, depth + 1))
                    .collect::<Result<_, _>>()?,
            ),
            TypedKind::Length(items) => {
                let Value::List(items) = self.data(items, env, depth + 1)? else {
                    unreachable!()
                };
                Value::Int(items.len() as i64)
            }
            TypedKind::Get(items, index) => {
                let Value::List(items) = self.data(items, env, depth + 1)? else {
                    unreachable!()
                };
                let Value::Int(index) = self.data(index, env, depth + 1)? else {
                    unreachable!()
                };
                match usize::try_from(index).ok().and_then(|i| items.get(i)) {
                    Some(value) => Value::Ok(Box::new(value.clone())),
                    None => Value::Err("list index out of bounds".into()),
                }
            }
            TypedKind::Fold(items, initial, body) => {
                let Value::List(items) = self.data(items, env, depth + 1)? else {
                    unreachable!()
                };
                let mut acc = self.term(initial, env, depth + 1)?;
                for item in items {
                    env.push(acc.into());
                    env.push(Binding::Data(item));
                    let result = self.term(body, env, depth + 1);
                    env.pop();
                    let accumulator = env.pop();
                    if result.is_ok() && matches!(accumulator, Some(Binding::Job(Some(_)))) {
                        return Err(Stop::JobStopped {
                            at: node.span,
                            reason: JobError::HostFailure,
                        });
                    }
                    acc = result?;
                }
                return self.finish(acc, node.span);
            }
            TypedKind::Call(index, args) => {
                let mut args = args
                    .iter()
                    .map(|arg| self.term(arg, env, depth + 1).map(Binding::from))
                    .collect::<Result<Vec<_>, _>>()?;
                let result =
                    self.term(&self.program.functions[*index].body, &mut args, depth + 1)?;
                if args.iter().any(|b| matches!(b, Binding::Job(Some(_)))) {
                    return Err(Stop::JobStopped {
                        at: node.span,
                        reason: JobError::HostFailure,
                    });
                }
                return self.finish(result, node.span);
            }
            TypedKind::Let(value, body) => {
                let value = self.term(value, env, depth + 1)?;
                env.push(value.into());
                let result = self.term(body, env, depth + 1);
                let binding = env.pop();
                if result.is_ok() && matches!(binding, Some(Binding::Job(Some(_)))) {
                    return Err(Stop::JobStopped {
                        at: node.span,
                        reason: JobError::HostFailure,
                    });
                }
                return self.finish(result?, node.span);
            }
            TypedKind::If(condition, yes, no) => {
                let Value::Bool(condition) = self.data(condition, env, depth + 1)? else {
                    unreachable!()
                };
                let value = self.term(if condition { yes } else { no }, env, depth + 1)?;
                return self.finish(value, node.span);
            }
            TypedKind::Match(value, ok, err) => {
                let value = self.data(value, env, depth + 1)?;
                let (binder, body) = match value {
                    Value::Ok(v) => (*v, ok),
                    Value::Err(v) => (Value::Text(v), err),
                    _ => unreachable!(),
                };
                env.push(Binding::Data(binder));
                let result = self.term(body, env, depth + 1);
                env.pop();
                return self.finish(result?, node.span);
            }
            TypedKind::Ok(value) => Value::Ok(Box::new(self.data(value, env, depth + 1)?)),
            TypedKind::Err(value) => {
                let Value::Text(message) = self.data(value, env, depth + 1)? else {
                    unreachable!()
                };
                Value::Err(message)
            }
            TypedKind::Unary(op, value) => match self.data(value, env, depth + 1)? {
                Value::Bool(v) if op == "not" => Value::Bool(!v),
                Value::Int(v) if op == "-" => Value::Int(v.checked_neg().ok_or_else(overflow)?),
                _ => unreachable!(),
            },
            TypedKind::Binary(op, left, right) => {
                let left = self.data(left, env, depth + 1)?;
                if op == "and" && left == Value::Bool(false) {
                    return self.account(left, node.span).map(StackValue::Data);
                }
                if op == "or" && left == Value::Bool(true) {
                    return self.account(left, node.span).map(StackValue::Data);
                }
                let right = self.data(right, env, depth + 1)?;
                match (op.as_str(), left, right) {
                    ("==", a, b) => Value::Bool(a == b),
                    ("!=", a, b) => Value::Bool(a != b),
                    ("and" | "or", Value::Bool(_), Value::Bool(b)) => Value::Bool(b),
                    (op, Value::Int(a), Value::Int(b)) => match op {
                        "+" => Value::Int(a.checked_add(b).ok_or_else(overflow)?),
                        "-" => Value::Int(a.checked_sub(b).ok_or_else(overflow)?),
                        "*" => Value::Int(a.checked_mul(b).ok_or_else(overflow)?),
                        "<" => Value::Bool(a < b),
                        ">" => Value::Bool(a > b),
                        "<=" => Value::Bool(a <= b),
                        ">=" => Value::Bool(a >= b),
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                }
            }
        };
        self.account(value, node.span).map(StackValue::Data)
    }
    fn account(&mut self, value: Value, at: Location) -> Result<Value, Stop> {
        let units = value::units(&value).ok_or(Stop::ValueLimit { at })?;
        self.value_work += units;
        if self.value_work > MAX_VALUE_WORK {
            return Err(Stop::ValueWorkLimit { at });
        }
        Ok(value)
    }
}
