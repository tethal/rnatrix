use crate::bc::builder::{BytecodeBuilder, InsKind};
use crate::bc::encoder::encode;
use crate::ctx::CompilerContext;
use crate::error::SourceResult;
use crate::hir::{Expr, ExprKind, Function, GlobalKind, LocalKind, Program, Stmt, StmtKind};
use natrix_runtime::bc::Bytecode;
use natrix_runtime::value::{BinaryOp, CodeHandle, FunctionObject, UnaryOp, Value};
use std::cmp::max;
use std::rc::Rc;

struct Compiler<'ctx> {
    ctx: &'ctx CompilerContext,
    used_slots: usize,
    max_slots: usize,
    local_slots: Vec<usize>, // indexed by LocalId
    functions: Vec<FunctionInfo>,
    bb: BytecodeBuilder,
}

struct FunctionInfo {
    max_slots: usize,
}

pub fn compile(ctx: &CompilerContext, program: &Program) -> SourceResult<Bytecode> {
    let mut compiler = Compiler::new(ctx);
    // TODO support more functions
    debug_assert!(program.globals.len() == 1);
    let GlobalKind::Function(f) = &program.globals.get(0).unwrap().kind;
    compiler.do_function(&f)?;

    let ir = compiler.bb;
    println!("Code:\n{:?}", ir);
    // let globals = program.globals.iter().map(|g| match g.kind {
    //     GlobalKind::Function(function) -> {
    //
    //     }
    // }).collect();
    let globals = vec![Value::from_function(Rc::new(FunctionObject {
        name: "main".into(),
        param_count: 1,
        max_slots: compiler.functions[0].max_slots,
        code_handle: CodeHandle(0),
    }))];
    Ok(Bytecode {
        code: encode(&ir),
        globals,
    })
}

impl<'ctx> Compiler<'ctx> {
    fn new(ctx: &'ctx CompilerContext) -> Self {
        Self {
            ctx,
            used_slots: 0,
            max_slots: 0,
            local_slots: Vec::new(),
            functions: Vec::new(),
            bb: BytecodeBuilder::new(),
        }
    }

    fn do_function(&mut self, f: &Function) -> SourceResult<()> {
        self.local_slots.resize(f.locals.len(), 0);
        for i in 0..f.param_count {
            assert_eq!(f.locals[i].kind, LocalKind::Parameter(i));
            self.local_slots[i] = i;
        }
        self.used_slots = f.param_count;
        self.max_slots = f.param_count;
        self.do_block(&f.body)?;
        self.functions.push(FunctionInfo {
            max_slots: self.max_slots,
        });
        Ok(())
    }

    fn do_block(&mut self, stmts: &Vec<Stmt>) -> SourceResult<()> {
        let saved_slots = self.used_slots;
        for stmt in stmts {
            self.do_stmt(&stmt)?;
        }
        self.used_slots = saved_slots;
        Ok(())
    }

    fn do_stmt(&mut self, stmt: &Stmt) -> SourceResult<()> {
        match &stmt.kind {
            StmtKind::Block(stmts) => self.do_block(&stmts),
            StmtKind::Expr(expr) => {
                self.do_expr(&expr)?;
                self.bb.append(stmt.span, InsKind::Pop)
            }
            StmtKind::Return(expr) => {
                self.do_expr(&expr)?;
                self.bb.append(stmt.span, InsKind::Ret)
            }
            StmtKind::StoreGlobal(id, expr) => {
                self.do_expr(&expr)?;
                self.bb.append(stmt.span, InsKind::StoreGlobal(id.0))
            }
            StmtKind::StoreLocal(id, expr) => {
                let slot = self.local_slots[id.0] + 1; // 0 is reserved (callee function object)
                self.do_expr(&expr)?;
                self.bb.append(stmt.span, InsKind::StoreLocal(slot))
            }
            StmtKind::VarDecl(id, expr) => {
                let slot = self.used_slots;
                self.local_slots[id.0] = slot;
                self.used_slots += 1;
                self.max_slots = max(self.max_slots, self.used_slots);
                self.do_expr(&expr)?;
                self.bb.append(stmt.span, InsKind::StoreLocal(slot + 1))
            }
        }
    }

    fn do_expr(&mut self, expr: &Expr) -> SourceResult<()> {
        match &expr.kind {
            ExprKind::Binary {
                op,
                op_span,
                left,
                right,
            } => {
                self.do_expr(&left)?;
                self.do_expr(&right)?;
                match op {
                    BinaryOp::Add => self.bb.append(*op_span, InsKind::Add),
                    BinaryOp::Sub => self.bb.append(*op_span, InsKind::Sub),
                    BinaryOp::Mul => self.bb.append(*op_span, InsKind::Mul),
                    BinaryOp::Div => self.bb.append(*op_span, InsKind::Div),
                    BinaryOp::Mod => self.bb.append(*op_span, InsKind::Mod),
                    BinaryOp::Eq => self.bb.append(*op_span, InsKind::Eq),
                    BinaryOp::Ne => self.bb.append(*op_span, InsKind::Ne),
                    BinaryOp::Ge => self.bb.append(*op_span, InsKind::Ge),
                    BinaryOp::Gt => self.bb.append(*op_span, InsKind::Gt),
                    BinaryOp::Le => self.bb.append(*op_span, InsKind::Le),
                    BinaryOp::Lt => self.bb.append(*op_span, InsKind::Lt),
                }
            }
            ExprKind::ConstBool(v) if *v => self.bb.append(expr.span, InsKind::PushTrue),
            ExprKind::ConstBool(_) => self.bb.append(expr.span, InsKind::PushFalse),
            ExprKind::ConstInt(v) if *v == 0 => self.bb.append(expr.span, InsKind::Push0),
            ExprKind::ConstInt(v) if *v == 1 => self.bb.append(expr.span, InsKind::Push1),
            ExprKind::ConstInt(v) => self.bb.append(expr.span, InsKind::PushInt(*v)),
            ExprKind::ConstNull => self.bb.append(expr.span, InsKind::PushNull),
            ExprKind::LoadBuiltin(builtin) => self
                .bb
                .append(expr.span, InsKind::LoadBuiltin(builtin.index())),
            ExprKind::LoadGlobal(id) => self.bb.append(expr.span, InsKind::LoadGlobal(id.0)),
            ExprKind::LoadLocal(id) => {
                let slot = self.local_slots[id.0] + 1; // 0 is reserved (callee function object)
                if slot == 1 {
                    self.bb.append(expr.span, InsKind::Load1)
                } else {
                    self.bb.append(expr.span, InsKind::LoadLocal(slot))
                }
            }
            ExprKind::Unary { op, op_span, expr } => {
                self.do_expr(&expr)?;
                match op {
                    UnaryOp::Neg => self.bb.append(*op_span, InsKind::Neg),
                    UnaryOp::Not => self.bb.append(*op_span, InsKind::Not),
                }
            }
        }
    }
}
