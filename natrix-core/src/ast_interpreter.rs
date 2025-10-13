use crate::ast::{BinaryOp, Expr, ExprKind, Stmt, StmtKind, UnaryOp};
use crate::ctx::{CompilerContext, Name};
use crate::error::{err_at, error_at, NxResult};
use crate::src::Span;
use crate::value::{Value, ValueType};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io;
use std::io::Write;

pub struct Interpreter<'ctx, W: Write = io::Stdout> {
    ctx: &'ctx CompilerContext,
    output: W,
}

impl<'ctx> Interpreter<'ctx, io::Stdout> {
    pub fn new(ctx: &'ctx CompilerContext) -> Self {
        Self {
            ctx,
            output: io::stdout(),
        }
    }
}

impl<'ctx, W: Write> Interpreter<'ctx, W> {
    pub fn with_output(ctx: &'ctx CompilerContext, output: W) -> Self {
        Self { ctx, output }
    }

    fn print(&mut self, span: Span, value: &Value) -> NxResult<()> {
        self.output
            .write_fmt(format_args!("{}\n", value))
            .map_err(|e| error_at(span, e.to_string()))
    }

    // TODO instead of stmt, accept a function and arguments, return the ret value
    pub fn invoke(&mut self, stmt: &Stmt) -> NxResult<Value> {
        let mut env = Env::new();
        self.do_stmt(&mut env, stmt)
    }

    // TODO return Action instead
    fn do_stmt(&mut self, env: &mut Env, stmt: &Stmt) -> NxResult<Value> {
        match &stmt.kind {
            StmtKind::Assign { left, right } => {
                let ExprKind::Var(name) = left.kind else {
                    unreachable!();
                };
                let val = self.eval(env, right)?;
                match env.vars.get_mut(&name) {
                    None => err_at(
                        left.span,
                        format!("undeclared variable {:?}", self.ctx.interner.resolve(name)),
                    ),
                    Some(slot) => {
                        *slot = val.clone();
                        Ok(val)
                    }
                }
            }
            StmtKind::Block(stmts) => {
                let mut result = Value::NULL;
                for stmt in stmts {
                    result = self.do_stmt(env, stmt)?;
                }
                Ok(result)
            }
            StmtKind::Expr(expr) => self.eval(env, expr),
            StmtKind::Print(expr) => {
                let value = self.eval(env, expr)?;
                self.print(expr.span, &value)?;
                Ok(value)
            }
            StmtKind::VarDecl {
                name,
                name_span,
                init,
            } => {
                let val = self.eval(env, init)?;
                match env.vars.entry(*name) {
                    Entry::Vacant(e) => Ok(e.insert(val).clone()),
                    Entry::Occupied(_) => err_at(
                        *name_span,
                        format!(
                            "variable {:?} already defined",
                            self.ctx.interner.resolve(*name)
                        ),
                    ),
                }
            }
        }
    }

    fn eval(&mut self, env: &mut Env, expr: &Expr) -> NxResult<Value> {
        match &expr.kind {
            ExprKind::Binary {
                op,
                op_span,
                left,
                right,
            } => eval_binary(*op, *op_span, self.eval(env, left)?, self.eval(env, right)?),
            ExprKind::BoolLiteral(value) => Ok(Value::from_bool(*value)),
            ExprKind::FloatLiteral(value) => Ok(Value::from_float(*value)),
            ExprKind::IntLiteral(value) => Ok(Value::from_int(*value)),
            ExprKind::LogicalBinary {
                and,
                op_span: _,
                left,
                right,
            } => {
                if *and {
                    Ok(Value::from_bool(
                        self.eval_bool(env, left)? && self.eval_bool(env, right)?,
                    ))
                } else {
                    Ok(Value::from_bool(
                        self.eval_bool(env, left)? || self.eval_bool(env, right)?,
                    ))
                }
            }
            ExprKind::NullLiteral => Ok(Value::NULL),
            ExprKind::Paren(inner) => self.eval(env, inner),
            ExprKind::Unary { op, op_span, expr } => {
                eval_unary(*op, *op_span, self.eval(env, expr)?)
            }
            ExprKind::Var(name) => match env.vars.get(name) {
                Some(val) => Ok(val.clone()),
                None => err_at(
                    expr.span,
                    format!("undeclared variable {:?}", self.ctx.interner.resolve(*name)),
                ),
            },
        }
    }

    fn eval_bool(&mut self, env: &mut Env, expr: &Expr) -> NxResult<bool> {
        let value = self.eval(env, expr)?;
        if value.get_type() != ValueType::Bool {
            err_at(expr.span, "expected a boolean value")
        } else {
            Ok(value.unwrap_bool())
        }
    }
}

struct Env {
    vars: HashMap<Name, Value>,
}

impl Env {
    fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }
}

fn eval_unary(op: UnaryOp, op_span: Span, value: Value) -> NxResult<Value> {
    match op {
        UnaryOp::Neg => match value.get_type() {
            ValueType::Int => Ok(Value::from_int(value.unwrap_int().wrapping_neg())),
            ValueType::Float => Ok(Value::from_float(-value.unwrap_float())),
            t => err_at(
                op_span,
                format!("unary negation cannot be applied to {:?}", t),
            ),
        },
        UnaryOp::Not => {
            if value.get_type() == ValueType::Bool {
                Ok(Value::from_bool(!value.unwrap_bool()))
            } else {
                err_at(
                    op_span,
                    format!(
                        "logical negation cannot be applied to {:?}",
                        value.get_type()
                    ),
                )
            }
        }
    }
}

fn eval_binary(op: BinaryOp, op_span: Span, left: Value, right: Value) -> NxResult<Value> {
    match op {
        BinaryOp::Add => eval_binary_add(op_span, left, right),
        BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
            eval_numeric_binary(op, op_span, left, right)
        }
        BinaryOp::Eq | BinaryOp::Ne => eval_equality(op, op_span, left, right),
        BinaryOp::Gt | BinaryOp::Ge | BinaryOp::Lt | BinaryOp::Le => {
            eval_comparison(op, op_span, left, right)
        }
    }
}

fn eval_binary_add(op_span: Span, left: Value, right: Value) -> NxResult<Value> {
    // TODO handle string concatenation here
    eval_numeric_binary(BinaryOp::Add, op_span, left, right)
}

fn eval_numeric_binary(op: BinaryOp, op_span: Span, left: Value, right: Value) -> NxResult<Value> {
    eval_with_numeric_coercion(
        op,
        op_span,
        left,
        right,
        |l, r| eval_integer_binary(op, op_span, l, r),
        |l, r| eval_float_binary(op, l, r),
    )
}

fn eval_integer_binary(op: BinaryOp, op_span: Span, left: i64, right: i64) -> NxResult<Value> {
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
        BinaryOp::Mod => {
            if right == 0 {
                return err_at(op_span, "division by zero");
            }
            left.wrapping_rem(right)
        }
        _ => unreachable!(),
    };
    Ok(Value::from_int(result))
}

fn eval_float_binary(op: BinaryOp, left: f64, right: f64) -> NxResult<Value> {
    let result = match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Div => left / right,
        BinaryOp::Mod => left % right,
        _ => unreachable!(),
    };
    Ok(Value::from_float(result))
}

fn eval_equality(op: BinaryOp, op_span: Span, left: Value, right: Value) -> NxResult<Value> {
    if left.is_bool() && right.is_bool() {
        let left = left.unwrap_bool();
        let right = right.unwrap_bool();
        match op {
            BinaryOp::Eq => Ok(Value::from_bool(left == right)),
            BinaryOp::Ne => Ok(Value::from_bool(left != right)),
            _ => unreachable!(),
        }
    } else {
        eval_comparison(op, op_span, left, right)
    }
}

fn eval_comparison(op: BinaryOp, op_span: Span, left: Value, right: Value) -> NxResult<Value> {
    eval_with_numeric_coercion(
        op,
        op_span,
        left,
        right,
        |l, r| eval_comparison_op(op, l, r),
        |l, r| eval_comparison_op(op, l, r),
    )
}

fn eval_comparison_op<T: PartialOrd>(op: BinaryOp, left: T, right: T) -> NxResult<Value> {
    let result = match op {
        BinaryOp::Eq => left == right,
        BinaryOp::Ne => left != right,
        BinaryOp::Ge => left >= right,
        BinaryOp::Gt => left > right,
        BinaryOp::Le => left <= right,
        BinaryOp::Lt => left < right,
        _ => unreachable!(),
    };
    Ok(Value::from_bool(result))
}

fn eval_with_numeric_coercion<I, F>(
    op: BinaryOp,
    op_span: Span,
    left: Value,
    right: Value,
    on_int: I,
    on_float: F,
) -> NxResult<Value>
where
    I: Fn(i64, i64) -> NxResult<Value>,
    F: Fn(f64, f64) -> NxResult<Value>,
{
    match (left.get_type(), right.get_type()) {
        (ValueType::Int, ValueType::Int) => on_int(left.unwrap_int(), right.unwrap_int()),
        (ValueType::Int, ValueType::Float) => {
            on_float(left.unwrap_int() as f64, right.unwrap_float())
        }
        (ValueType::Float, ValueType::Int) => {
            on_float(left.unwrap_float(), right.unwrap_int() as f64)
        }
        (ValueType::Float, ValueType::Float) => on_float(left.unwrap_float(), right.unwrap_float()),
        (lt, rt) => err_at(
            op_span,
            format!(
                "operator {:?} cannot be applied to {:?} and {:?}",
                op, lt, rt
            ),
        ),
    }
}
