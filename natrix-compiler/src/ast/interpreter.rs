use crate::ast::{AssignTargetKind, Expr, ExprKind, FunDecl, Program, Stmt, StmtKind};
use crate::ctx::{CompilerContext, Name};
use crate::error::{err_at, AttachErrSpan, SourceResult};
use crate::src::Span;
use natrix_runtime::nx_err::{nx_err, nx_error, NxResult};
use natrix_runtime::runtime::{Builtin, Runtime};
use natrix_runtime::value::{CodeHandle, FunctionObject, Value, ValueType};
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::Into;
use std::rc::Rc;

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
        let mut vars: HashMap<Name, Value> = HashMap::new();
        for builtin in Builtin::ALL {
            Env::define_builtin(ctx, &mut vars, builtin);
        }
        let env = Rc::new(Env {
            vars: RefCell::new(vars),
            parent: None,
        });
        // wrap builtin (read-only) scope in a global, writable scope
        Env::new(env)
    }

    fn new(parent: Rc<Env>) -> Rc<Env> {
        Rc::new(Self {
            vars: RefCell::new(HashMap::new()),
            parent: Some(parent),
        })
    }

    fn define_builtin(
        ctx: &mut CompilerContext,
        vars: &mut HashMap<Name, Value>,
        builtin: &Builtin,
    ) {
        vars.insert(
            ctx.interner.intern(builtin.name()),
            Value::from_function(Rc::new(FunctionObject {
                name: builtin.name().into(),
                arity: builtin.arity(),
                num_locals: 0,
                code_handle: builtin.as_code_handle(),
            })),
        );
    }

    fn lookup(&self, ctx: &CompilerContext, name: &Name) -> NxResult<Value> {
        match self.vars.borrow().get(name).cloned() {
            Some(val) => Ok(val),
            None => match &self.parent {
                Some(parent) => parent.lookup(ctx, name),
                None => nx_err(format!(
                    "undeclared variable {:?}",
                    ctx.interner.resolve(*name)
                )),
            },
        }
    }

    fn declare(&self, ctx: &CompilerContext, name: Name, value: Value) -> NxResult<()> {
        match self.vars.borrow_mut().entry(name) {
            Entry::Vacant(e) => {
                e.insert(value);
                Ok(())
            }
            Entry::Occupied(_) => nx_err(format!(
                "symbol {} already defined in this scope",
                ctx.interner.resolve(name)
            )),
        }
    }

    fn assign(&self, ctx: &CompilerContext, name: Name, value: Value) -> NxResult<()> {
        if let Some(slot) = self.vars.borrow_mut().get_mut(&name) {
            if self.parent.is_none() {
                nx_err("built-in function cannot be assigned to")
            } else {
                *slot = value;
                Ok(())
            }
        } else {
            self.parent
                .as_ref()
                .ok_or(nx_error(format!(
                    "undeclared variable {:?}",
                    ctx.interner.resolve(name)
                )))?
                .assign(ctx, name, value)
        }
    }
}

pub struct Interpreter<'ctx> {
    ctx: &'ctx CompilerContext,
    rt: &'ctx mut Runtime,
    globals: Rc<Env>,
    fun_decls: Vec<Rc<FunDecl>>,
}

impl<'ctx> Interpreter<'ctx> {
    pub fn new(ctx: &'ctx mut CompilerContext, rt: &'ctx mut Runtime) -> Self {
        let globals = Env::new_root(ctx);
        Self {
            ctx,
            rt,
            globals,
            fun_decls: Vec::new(),
        }
    }

    pub fn run(&mut self, program: Program, args: Vec<Value>) -> SourceResult<Value> {
        let main_name = self.ctx.interner.lookup("main");
        let mut main_fun: Option<(Value, Span)> = None;
        for decl in program.decls {
            let index = self.fun_decls.len();
            let fun_obj = Value::from_function(Rc::new(FunctionObject {
                name: self.ctx.interner.resolve(decl.name).into(),
                arity: decl.params.len(),
                num_locals: 0,
                code_handle: CodeHandle(index),
            }));
            if main_name == Some(decl.name) {
                main_fun = Some((fun_obj.clone(), decl.name_span));
            }
            self.globals
                .declare(self.ctx, decl.name, fun_obj)
                .err_at(decl.name_span)?;
            self.fun_decls.push(Rc::new(decl));
        }
        match main_fun {
            Some((fun_decl, span)) => self.dispatch(span, fun_decl, args),
            None => err_at(program.span, "no main function defined"),
        }
    }

    fn dispatch(&mut self, span: Span, callee: Value, args: Vec<Value>) -> SourceResult<Value> {
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

        match Builtin::from_code_handle(fun_obj.code_handle) {
            Some(builtin) => self.rt.dispatch_builtin(builtin, &args).err_at(span),
            None => self.invoke(
                self.fun_decls.get(fun_obj.code_handle.0).unwrap().clone(),
                args,
            ),
        }
    }

    fn invoke(&mut self, fun_decl: Rc<FunDecl>, args: Vec<Value>) -> SourceResult<Value> {
        let env = Env::new(self.globals.clone());
        for (param, arg) in fun_decl.params.iter().zip(args) {
            env.declare(self.ctx, param.name, arg)
                .err_at(param.name_span)?;
        }
        match self.do_stmt(&env, &fun_decl.body)? {
            StmtFlow::Next => Ok(Value::NULL),
            StmtFlow::Return(value) => Ok(value),
            StmtFlow::Break(span) => err_at(span, "break outside a loop"),
            StmtFlow::Continue(span) => err_at(span, "continue outside a loop"),
        }
    }

    fn do_stmt(&mut self, env: &Rc<Env>, stmt: &Stmt) -> SourceResult<StmtFlow> {
        match &stmt.kind {
            StmtKind::Assign { target, value } => {
                match &target.kind {
                    AssignTargetKind::Var(name) => {
                        let val = self.eval(env, value)?;
                        env.assign(self.ctx, *name, val).err_at(target.span)?;
                    }
                    AssignTargetKind::ArrayAccess { array, index } => {
                        let array = self.eval(env, &array)?;
                        let index = self.eval(env, &index)?;
                        let val = self.eval(env, value)?;
                        array.set_item(index, val).err_at(target.span)?;
                    }
                }
                Ok(StmtFlow::Next)
            }
            StmtKind::Block(stmts) => {
                let inner_env = Env::new(env.clone());
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
                env.declare(self.ctx, *name, val).err_at(*name_span)?;
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

    fn eval(&mut self, env: &Rc<Env>, expr: &Expr) -> SourceResult<Value> {
        match &expr.kind {
            ExprKind::ArrayAccess { array, index } => {
                let array = self.eval(env, array)?;
                let index = self.eval(env, index)?;
                array.get_item(index).err_at(expr.span)
            }
            ExprKind::Binary {
                op,
                op_span,
                left,
                right,
            } => {
                let left = self.eval(env, left)?;
                let right = self.eval(env, right)?;
                op.eval(&left, &right).err_at(*op_span)
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
                op.eval(&val).err_at(*op_span)
            }
            ExprKind::Var(name) => env.lookup(self.ctx, name).err_at(expr.span),
        }
    }

    fn eval_bool(&mut self, env: &Rc<Env>, expr: &Expr) -> SourceResult<bool> {
        let value = self.eval(env, expr)?;
        if value.get_type() != ValueType::Bool {
            err_at(expr.span, "expected a boolean value")
        } else {
            Ok(value.unwrap_bool())
        }
    }
}
