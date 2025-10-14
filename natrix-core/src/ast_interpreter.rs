use crate::ast::{BinaryOp, Expr, ExprKind, FunDecl, Program, Stmt, StmtKind, UnaryOp};
use crate::ctx::{CompilerContext, Name};
use crate::error::{err, err_at, error_at, NxResult};
use crate::src::Span;
use crate::value::{Value, ValueType};
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::rc::Rc;

pub struct Interpreter<'ctx, W: Write = io::Stdout> {
    ctx: &'ctx CompilerContext,
    output: W,
    functions: HashMap<Name, Rc<FunDecl>>,
}

impl<'ctx> Interpreter<'ctx, io::Stdout> {
    pub fn new(ctx: &'ctx CompilerContext) -> Self {
        Self {
            ctx,
            output: io::stdout(),
            functions: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
enum StmtFlow {
    Next,           // Normal execution continues
    Return(Value),  // Early return from function
    Break(Span),    // Exit innermost loop
    Continue(Span), // Skip to next loop iteration
}

impl<'ctx, W: Write> Interpreter<'ctx, W> {
    pub fn with_output(ctx: &'ctx CompilerContext, output: W) -> Self {
        Self {
            ctx,
            output,
            functions: HashMap::new(),
        }
    }

    fn print(&mut self, span: Span, value: &Value) -> NxResult<()> {
        self.output
            .write_fmt(format_args!("{}\n", value))
            .map_err(|e| error_at(span, e.to_string()))
    }

    fn find_fun_decl(&self, name: Name) -> Option<Rc<FunDecl>> {
        self.functions.get(&name).cloned()
    }

    pub fn run(&mut self, program: Program, args: Vec<Value>) -> NxResult<Value> {
        let main_name = self.ctx.interner.lookup("main");
        for decl in program.decls {
            self.functions.insert(decl.name, Rc::new(decl));
        }
        let fun_decl_opt = main_name.and_then(|name| self.find_fun_decl(name));
        match fun_decl_opt {
            Some(fun_decl) => self.invoke(None, fun_decl, args),
            None => err("no main function defined"),
        }
    }

    pub fn invoke(
        &mut self,
        call_site_span: Option<Span>,
        fun_decl: Rc<FunDecl>,
        args: Vec<Value>,
    ) -> NxResult<Value> {
        if args.len() != fun_decl.params.len() {
            return err_at(
                call_site_span.unwrap_or(fun_decl.name_span),
                format!(
                    "function expects {} argument{}, but {} were provided",
                    fun_decl.params.len(),
                    if fun_decl.params.len() == 1 { "" } else { "s" },
                    args.len()
                ),
            );
        }
        let mut env = Env::new_root();
        for (param, arg) in fun_decl.params.iter().zip(args) {
            env.vars.borrow_mut().insert(param.name, arg);
        }
        match self.do_stmt(&mut env, &fun_decl.body)? {
            StmtFlow::Next => Ok(Value::NULL),
            StmtFlow::Return(value) => Ok(value),
            StmtFlow::Break(span) => err_at(span, "break outside a loop"),
            StmtFlow::Continue(span) => err_at(span, "continue outside a loop"),
        }
    }

    fn do_stmt(&mut self, env: &Rc<Env>, stmt: &Stmt) -> NxResult<StmtFlow> {
        match &stmt.kind {
            StmtKind::Assign { left, right } => {
                let ExprKind::Var(name) = left.kind else {
                    unreachable!();
                };
                let val = self.eval(env, right)?;
                match env.vars.borrow_mut().get_mut(&name) {
                    None => err_at(
                        left.span,
                        format!("undeclared variable {:?}", self.ctx.interner.resolve(name)),
                    ),
                    Some(slot) => {
                        *slot = val;
                        Ok(StmtFlow::Next)
                    }
                }
            }
            StmtKind::Block(stmts) => {
                let inner_env = Env::new(env);
                for stmt in stmts {
                    let flow = self.do_stmt(&inner_env, stmt)?;
                    if !matches!(flow, StmtFlow::Next) {
                        return Ok(flow);
                    }
                }
                Ok(StmtFlow::Next)
            }
            StmtKind::Expr(expr) => {
                self.eval(env, expr)?;
                Ok(StmtFlow::Next)
            }
            StmtKind::Print(expr) => {
                let value = self.eval(env, expr)?;
                self.print(expr.span, &value)?;
                Ok(StmtFlow::Next)
            }
            StmtKind::Return(expr) => {
                let value = match expr {
                    Some(expr) => self.eval(env, expr)?,
                    None => Value::NULL,
                };
                Ok(StmtFlow::Return(value))
            }
            StmtKind::VarDecl {
                name,
                name_span,
                init,
            } => {
                let val = self.eval(env, init)?;
                match env.vars.borrow_mut().entry(*name) {
                    Entry::Vacant(e) => {
                        e.insert(val);
                        Ok(StmtFlow::Next)
                    }
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

    fn eval(&mut self, env: &Rc<Env>, expr: &Expr) -> NxResult<Value> {
        match &expr.kind {
            ExprKind::Binary {
                op,
                op_span,
                left,
                right,
            } => eval_binary(*op, *op_span, self.eval(env, left)?, self.eval(env, right)?),
            ExprKind::BoolLiteral(value) => Ok(Value::from_bool(*value)),
            ExprKind::Call {
                name,
                name_span,
                args,
            } => {
                let mut arg_values = Vec::with_capacity(args.len());
                for arg in args {
                    arg_values.push(self.eval(env, arg)?);
                }
                match self.find_fun_decl(*name) {
                    Some(fun_decl) => self.invoke(Some(*name_span), fun_decl, arg_values),
                    None => err_at(
                        *name_span,
                        format!("undeclared function {:?}", self.ctx.interner.resolve(*name)),
                    ),
                }
            }
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
            ExprKind::Var(name) => match env.lookup(name) {
                Some(val) => Ok(val.clone()),
                None => err_at(
                    expr.span,
                    format!("undeclared variable {:?}", self.ctx.interner.resolve(*name)),
                ),
            },
        }
    }

    fn eval_bool(&mut self, env: &Rc<Env>, expr: &Expr) -> NxResult<bool> {
        let value = self.eval(env, expr)?;
        if value.get_type() != ValueType::Bool {
            err_at(expr.span, "expected a boolean value")
        } else {
            Ok(value.unwrap_bool())
        }
    }
}

struct Env {
    vars: RefCell<HashMap<Name, Value>>,
    parent: Option<Rc<Env>>,
}

impl Env {
    fn new_root() -> Rc<Env> {
        Rc::new(Env {
            vars: RefCell::new(HashMap::new()),
            parent: None,
        })
    }

    fn new(parent: &Rc<Env>) -> Rc<Env> {
        Rc::new(Self {
            vars: RefCell::new(HashMap::new()),
            parent: Some(parent.clone()),
        })
    }

    fn lookup(&self, name: &Name) -> Option<Value> {
        self.vars
            .borrow()
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_ref()?.lookup(name))
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
