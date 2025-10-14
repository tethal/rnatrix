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

    fn declare(&self, name: Name, value: Value) -> Result<(), ()> {
        match self.vars.borrow_mut().entry(name) {
            Entry::Vacant(e) => {
                e.insert(value);
                Ok(())
            }
            Entry::Occupied(_) => Err(()),
        }
    }

    fn assign(&self, name: Name, value: Value) -> Result<(), ()> {
        if let Some(slot) = self.vars.borrow_mut().get_mut(&name) {
            *slot = value;
            Ok(())
        } else {
            self.parent.as_ref().ok_or(())?.assign(name, value)
        }
    }
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
        let env = Env::new_root();
        for (param, arg) in fun_decl.params.iter().zip(args) {
            env.declare(param.name, arg).map_err(|_| {
                error_at(
                    param.name_span,
                    format!(
                        "parameter {} already defined",
                        self.ctx.interner.resolve(param.name)
                    ),
                )
            })?;
        }
        match self.do_stmt(&env, &fun_decl.body)? {
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
                env.assign(name, val).map_err(|_| {
                    error_at(
                        left.span,
                        format!("undeclared variable {:?}", self.ctx.interner.resolve(name)),
                    )
                })?;
                Ok(StmtFlow::Next)
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
            StmtKind::Break => Ok(StmtFlow::Break(stmt.span)),
            StmtKind::Continue => Ok(StmtFlow::Continue(stmt.span)),
            StmtKind::Expr(expr) => {
                self.eval(env, expr)?;
                Ok(StmtFlow::Next)
            }
            StmtKind::If {
                cond,
                then_body,
                else_body,
            } => {
                if self.eval_bool(env, cond)? {
                    self.do_stmt(env, then_body)
                } else if let Some(else_body) = else_body {
                    self.do_stmt(env, else_body)
                } else {
                    Ok(StmtFlow::Next)
                }
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
                env.declare(*name, val).map_err(|_| {
                    error_at(
                        *name_span,
                        format!(
                            "variable {:?} already defined",
                            self.ctx.interner.resolve(*name)
                        ),
                    )
                })?;
                Ok(StmtFlow::Next)
            }
            StmtKind::While { cond, body } => {
                while self.eval_bool(env, cond)? {
                    match self.do_stmt(&env, body)? {
                        StmtFlow::Next => {}
                        StmtFlow::Break(_) => break,
                        StmtFlow::Continue(_) => continue,
                        StmtFlow::Return(value) => return Ok(StmtFlow::Return(value)),
                    }
                }
                Ok(StmtFlow::Next)
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
            } => {
                let left = self.eval(env, left)?;
                let right = self.eval(env, right)?;
                match op {
                    BinaryOp::Add => left.add(&right, *op_span),
                    BinaryOp::Sub => left.sub(&right, *op_span),
                    BinaryOp::Mul => left.mul(&right, *op_span),
                    BinaryOp::Div => left.div(&right, *op_span),
                    BinaryOp::Mod => left.rem(&right, *op_span),
                    BinaryOp::Eq => left.eq(&right, *op_span),
                    BinaryOp::Ne => left.ne(&right, *op_span),
                    BinaryOp::Ge => left.ge(&right, *op_span),
                    BinaryOp::Gt => left.gt(&right, *op_span),
                    BinaryOp::Le => left.le(&right, *op_span),
                    BinaryOp::Lt => left.lt(&right, *op_span),
                }
            }
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
                let val = self.eval(env, expr)?;
                match op {
                    UnaryOp::Neg => val.negate(*op_span),
                    UnaryOp::Not => val.not(*op_span),
                }
            }
            ExprKind::Var(name) => match env.lookup(name) {
                Some(val) => Ok(val),
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
