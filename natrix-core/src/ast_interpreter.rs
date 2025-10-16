use crate::ast::{BinaryOp, Expr, ExprKind, FunDecl, Program, Stmt, StmtKind, UnaryOp};
use crate::ctx::{CompilerContext, Name};
use crate::error::{err, err_at, error_at, NxResult};
use crate::src::Span;
use crate::value::{
    CodeHandle, FunctionObject, Value, ValueType, BUILTIN_FLOAT, BUILTIN_INT,
    BUILTIN_LEN, BUILTIN_PRINT, BUILTIN_STR,
};
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::Into;
use std::io;
use std::io::Write;
use std::rc::Rc;

pub struct Interpreter<'ctx, W: Write = io::Stdout> {
    ctx: &'ctx CompilerContext,
    output: W,
    globals: Rc<Env>,
    fun_decls: Vec<Rc<FunDecl>>,
}

impl<'ctx> Interpreter<'ctx, io::Stdout> {
    pub fn new(ctx: &'ctx mut CompilerContext) -> Self {
        let globals = Env::new_root(ctx);
        Self {
            ctx,
            output: io::stdout(),
            globals,
            fun_decls: Vec::new(),
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
    fn new_root(ctx: &mut CompilerContext) -> Rc<Env> {
        let env = Rc::new(Env {
            vars: RefCell::new(HashMap::new()),
            parent: None,
        });
        env.define_builtin(ctx, "print", 1, BUILTIN_PRINT);
        env.define_builtin(ctx, "len", 1, BUILTIN_LEN);
        env.define_builtin(ctx, "str", 1, BUILTIN_STR);
        env.define_builtin(ctx, "int", 1, BUILTIN_INT);
        env.define_builtin(ctx, "float", 1, BUILTIN_FLOAT);
        env
    }

    fn new(parent: &Rc<Env>) -> Rc<Env> {
        Rc::new(Self {
            vars: RefCell::new(HashMap::new()),
            parent: Some(parent.clone()),
        })
    }

    fn define_builtin(
        &self,
        ctx: &mut CompilerContext,
        name: &str,
        arity: usize,
        code_handle: CodeHandle,
    ) {
        self.declare(
            ctx.interner.intern(name),
            Value::from_function(Rc::new(FunctionObject {
                name: name.into(),
                arity,
                code_handle,
            })),
        )
        .expect("duplicate built-in function");
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
    pub fn with_output(ctx: &'ctx mut CompilerContext, output: W) -> Self {
        let globals = Env::new_root(ctx);
        Self {
            ctx,
            output,
            globals,
            fun_decls: Vec::new(),
        }
    }

    fn print(&mut self, span: Span, value: &Value) -> NxResult<()> {
        self.output
            .write_fmt(format_args!("{}\n", value))
            .map_err(|e| error_at(span, e.to_string()))
    }

    pub fn run(&mut self, program: Program, args: Vec<Value>) -> NxResult<Value> {
        let main_name = self.ctx.interner.lookup("main");
        let mut main_fun: Option<(Value, Span)> = None;
        for decl in program.decls {
            let decl = Rc::new(decl);
            let index = self.fun_decls.len();
            self.fun_decls.push(decl.clone());
            let fun_obj = Value::from_function(Rc::new(FunctionObject {
                name: self.ctx.interner.resolve(decl.name).into(),
                arity: decl.params.len(),
                code_handle: CodeHandle(index),
            }));
            if main_name == Some(decl.name) {
                main_fun = Some((fun_obj.clone(), decl.name_span));
            }
            self.globals.declare(decl.name, fun_obj).map_err(|_| {
                error_at(
                    decl.name_span,
                    format!(
                        "function {} already defined",
                        self.ctx.interner.resolve(decl.name)
                    ),
                )
            })?;
        }
        match main_fun {
            Some((fun_decl, span)) => self.dispatch(span, fun_decl, args),
            None => err("no main function defined"),
        }
    }

    fn dispatch(&mut self, span: Span, callee: Value, args: Vec<Value>) -> NxResult<Value> {
        if !callee.is_function() {
            return err_at(span, format!("not a function: {}", callee));
        }
        let fun_obj = callee.unwrap_function();

        if args.len() != fun_obj.arity {
            return err_at(
                span,
                format!(
                    "function {} expects {} argument{}, but {} were provided",
                    fun_obj.name,
                    fun_obj.arity,
                    if fun_obj.arity == 1 { "" } else { "s" },
                    args.len()
                ),
            );
        }

        match fun_obj.code_handle {
            BUILTIN_FLOAT => args[0].float(span),
            BUILTIN_INT => args[0].int(span),
            BUILTIN_LEN => args[0].len(span),
            BUILTIN_PRINT => {
                self.print(span, &args[0])?;
                Ok(Value::NULL)
            }
            BUILTIN_STR => Ok(args[0].str()),
            CodeHandle(index) => self.invoke(self.fun_decls.get(index).unwrap().clone(), args),
        }
    }

    fn invoke(&mut self, fun_decl: Rc<FunDecl>, args: Vec<Value>) -> NxResult<Value> {
        let env = Env::new(&self.globals);
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
                match &left.kind {
                    ExprKind::Var(name) => {
                        let val = self.eval(env, right)?;
                        env.assign(*name, val).map_err(|_| {
                            error_at(
                                left.span,
                                format!(
                                    "undeclared variable {:?}",
                                    self.ctx.interner.resolve(*name)
                                ),
                            )
                        })?;
                    }
                    ExprKind::ArrayAccess { array, index } => {
                        let array = self.eval(env, &array)?;
                        let index = self.eval(env, &index)?;
                        let val = self.eval(env, right)?;
                        array.set_item(index, val, left.span)?;
                    }
                    _ => unreachable!(),
                }
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
            ExprKind::ArrayAccess { array, index } => {
                let array = self.eval(env, array)?;
                let index = self.eval(env, index)?;
                array.get_item(index, expr.span)
            }
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
            ExprKind::Call { callee, args } => {
                let callee = self.eval(env, callee)?;
                let mut arg_values = Vec::with_capacity(args.len());
                for arg in args {
                    arg_values.push(self.eval(env, arg)?);
                }
                self.dispatch(expr.span, callee, arg_values)
            }
            ExprKind::FloatLiteral(value) => Ok(Value::from_float(*value)),
            ExprKind::IntLiteral(value) => Ok(Value::from_int(*value)),
            ExprKind::ListLiteral(exprs) => {
                let mut values = Vec::with_capacity(exprs.len());
                for expr in exprs {
                    values.push(self.eval(env, expr)?);
                }
                Ok(Value::from_list(Rc::new(RefCell::new(values))))
            }
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
            ExprKind::StringLiteral(value) => Ok(Value::from_string(value.clone())),
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
