//! Path-sensitive authority checking over the private, already typed tree.
use crate::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Local {
    Data,
    Available,
    Consumed,
}

pub(super) fn check(body: &Typed, parameters: &[Parameter]) -> Result<(), Diagnostic> {
    let mut env = parameters.iter().map(|p| local(&p.ty)).collect();
    visit(body, &mut env)?;
    if env.contains(&Local::Available) {
        return Err(error(
            body.span,
            "Job parameter must be collected or transferred on every normal path",
        ));
    }
    Ok(())
}

fn local(ty: &Type) -> Local {
    if matches!(ty, Type::Job(_)) {
        Local::Available
    } else {
        Local::Data
    }
}

fn join(at: Location, left: &[Local], right: &[Local]) -> Result<(), Diagnostic> {
    if left != right {
        return Err(error(
            at,
            "branches must consume the same Job handles; repeated regions cannot consume outer handles",
        ));
    }
    Ok(())
}

fn visit(node: &Typed, env: &mut Vec<Local>) -> Result<(), Diagnostic> {
    match &node.kind {
        TypedKind::JobStart(input, _) => visit(input, env)?,
        TypedKind::JobLeanCheck(statement, proof) => {
            visit(statement, env)?;
            visit(proof, env)?;
        }
        TypedKind::JobControl(op, index) => {
            if env[*index] != Local::Available {
                return Err(error(
                    node.span,
                    "Job handle has already been collected or transferred",
                ));
            }
            if *op == JobOperation::Collect {
                env[*index] = Local::Consumed;
            }
        }
        TypedKind::Local(index) if env[*index] != Local::Data => {
            if env[*index] != Local::Available {
                return Err(error(
                    node.span,
                    "Job handle has already been collected or transferred",
                ));
            }
            env[*index] = Local::Consumed;
        }
        TypedKind::Local(_) | TypedKind::Literal(_) => {}
        TypedKind::Let(value, body) => {
            visit(value, env)?;
            env.push(local(&value.ty));
            visit(body, env)?;
            if env.pop() == Some(Local::Available) {
                return Err(error(
                    value.span,
                    "scoped Job must be collected or transferred on every normal path",
                ));
            }
        }
        TypedKind::If(condition, yes, no) => {
            visit(condition, env)?;
            let mut other = env.clone();
            visit(yes, env)?;
            visit(no, &mut other)?;
            join(node.span, env, &other)?;
        }
        TypedKind::Match(value, ok, err) => {
            visit(value, env)?;
            let mut other = env.clone();
            env.push(Local::Data);
            visit(ok, env)?;
            env.pop();
            other.push(Local::Data);
            visit(err, &mut other)?;
            other.pop();
            join(node.span, env, &other)?;
        }
        TypedKind::Fold(items, initial, body) => {
            visit(items, env)?;
            visit(initial, env)?;
            let before = env.clone();
            env.extend([local(&initial.ty), Local::Data]);
            visit(body, env)?;
            if env[before.len()] == Local::Available {
                return Err(error(
                    body.span,
                    "each iteration must consume or transfer its Job accumulator",
                ));
            }
            env.truncate(before.len());
            join(node.span, env, &before)?;
        }
        TypedKind::Binary(op, left, right) => {
            visit(left, env)?;
            let before = env.clone();
            visit(right, env)?;
            if op == "and" || op == "or" {
                join(node.span, env, &before)?;
            }
        }
        TypedKind::Get(left, right) => {
            visit(left, env)?;
            visit(right, env)?;
        }
        TypedKind::Call(_, args) | TypedKind::List(args) => {
            for arg in args {
                visit(arg, env)?;
            }
        }
        TypedKind::Record(_, fields) => {
            for (_, value) in fields {
                visit(value, env)?;
            }
        }
        TypedKind::JobSquare(value)
        | TypedKind::Ok(value)
        | TypedKind::Err(value)
        | TypedKind::Unary(_, value)
        | TypedKind::Field(value, _)
        | TypedKind::Length(value) => visit(value, env)?,
    }
    Ok(())
}
