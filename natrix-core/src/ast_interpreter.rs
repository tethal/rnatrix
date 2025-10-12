use crate::ast::{BinaryOp, Expr, ExprKind, Stmt, StmtKind, UnaryOp};
use crate::ctx::{CompilerContext, Name};
use crate::error::{err_at, NxResult};
use crate::src::Span;
use crate::value::{Value, ValueType};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub type EvalResult = NxResult<Value>;

struct Env<'a> {
    ctx: &'a CompilerContext,
    vars: HashMap<Name, Value>,
}

impl<'a> Env<'a> {
    fn new(ctx: &'a CompilerContext) -> Self {
        Self {
            ctx,
            vars: HashMap::new(),
        }
    }
}

pub fn run(ctx: &CompilerContext, stmt: &Stmt) -> EvalResult {
    let mut env = Env::new(ctx);
    execute(&mut env, stmt)
}

fn execute(env: &mut Env, stmt: &Stmt) -> EvalResult {
    match &stmt.kind {
        StmtKind::Expr(expr) => eval(env, expr),
        StmtKind::Block(stmts) => {
            let mut result = Value::NULL;
            for stmt in stmts {
                result = execute(env, stmt)?;
            }
            Ok(result)
        }
        StmtKind::Assign { left, right } => {
            let ExprKind::Var(name) = left.kind else {
                unreachable!();
            };
            let val = eval(env, right)?;
            match env.vars.get_mut(&name) {
                None => err_at(
                    left.span,
                    format!("undeclared variable {:?}", env.ctx.interner.resolve(name)),
                ),
                Some(slot) => {
                    *slot = val.clone();
                    Ok(val)
                }
            }
        }
        StmtKind::VarDecl {
            name,
            name_span,
            init,
        } => {
            let val = eval(env, init)?;
            match env.vars.entry(*name) {
                Entry::Vacant(e) => Ok(e.insert(val).clone()),
                Entry::Occupied(_) => err_at(
                    *name_span,
                    format!(
                        "variable {:?} already defined",
                        env.ctx.interner.resolve(*name)
                    ),
                ),
            }
        }
    }
}

fn eval(env: &mut Env, expr: &Expr) -> EvalResult {
    match &expr.kind {
        ExprKind::IntLiteral(value) => Ok(Value::from_int(*value)),
        ExprKind::FloatLiteral(value) => Ok(Value::from_float(*value)),
        ExprKind::BoolLiteral(value) => Ok(Value::from_bool(*value)),
        ExprKind::NullLiteral => Ok(Value::NULL),
        ExprKind::Paren(inner) => eval(env, inner),
        ExprKind::Unary { op, op_span, expr } => eval_unary(*op, *op_span, eval(env, expr)?),
        ExprKind::Binary {
            op,
            op_span,
            left,
            right,
        } => eval_binary(*op, *op_span, eval(env, left)?, eval(env, right)?),
        ExprKind::Var(name) => match env.vars.get(name) {
            Some(val) => Ok(val.clone()),
            None => err_at(
                expr.span,
                format!("undeclared variable {:?}", env.ctx.interner.resolve(*name)),
            ),
        },
    }
}

fn eval_unary(op: UnaryOp, op_span: Span, value: Value) -> EvalResult {
    match op {
        UnaryOp::Neg => match value.get_type() {
            ValueType::Int => Ok(Value::from_int(value.as_int().unwrap().wrapping_neg())),
            ValueType::Float => Ok(Value::from_float(-value.as_float().unwrap())),
            t => err_at(
                op_span,
                format!("unary negation cannot be applied to {:?}", t),
            ),
        },
    }
}

fn eval_binary(op: BinaryOp, op_span: Span, left: Value, right: Value) -> EvalResult {
    match op {
        BinaryOp::Add => eval_binary_add(op_span, left, right),
        BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
            eval_numeric_binary(op, op_span, left, right)
        }
    }
}

fn eval_binary_add(op_span: Span, left: Value, right: Value) -> EvalResult {
    // TODO handle string concatenation here
    eval_numeric_binary(BinaryOp::Add, op_span, left, right)
}

fn eval_numeric_binary(op: BinaryOp, op_span: Span, left: Value, right: Value) -> EvalResult {
    match (left.get_type(), right.get_type()) {
        (ValueType::Int, ValueType::Int) => {
            eval_integer_binary(op, op_span, left.as_int().unwrap(), right.as_int().unwrap())
        }
        (ValueType::Int, ValueType::Float) => {
            eval_float_binary(op, left.as_int().unwrap() as f64, right.as_float().unwrap())
        }
        (ValueType::Float, ValueType::Int) => {
            eval_float_binary(op, left.as_float().unwrap(), right.as_int().unwrap() as f64)
        }
        (ValueType::Float, ValueType::Float) => {
            eval_float_binary(op, left.as_float().unwrap(), right.as_float().unwrap())
        }
        (lt, rt) => err_at(
            op_span,
            format!(
                "binary operator {:?} cannot be applied to {:?} and {:?}",
                op, lt, rt
            ),
        ),
    }
}

fn eval_integer_binary(op: BinaryOp, op_span: Span, left: i64, right: i64) -> EvalResult {
    let result = match op {
        BinaryOp::Add => left.wrapping_add(right),
        BinaryOp::Sub => left.wrapping_sub(right),
        BinaryOp::Mul => left.wrapping_mul(right),
        BinaryOp::Div => {
            if right == 0 {
                return err_at(op_span, "division by zero");
            }
            left.wrapping_div(right)
        }
    };
    Ok(Value::from_int(result))
}

fn eval_float_binary(op: BinaryOp, left: f64, right: f64) -> EvalResult {
    let result = match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Div => left / right,
    };
    Ok(Value::from_float(result))
}
