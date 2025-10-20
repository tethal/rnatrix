use crate::bc::builder::{BytecodeBuilder, InsKind, Label};
use crate::ctx::CompilerContext;
use crate::error::SourceResult;
use crate::hir::{
    Expr, ExprKind, Function, GlobalKind, LocalId, LocalKind, LoopId, Program, Stmt, StmtKind,
};
use natrix_runtime::bc::Bytecode;
use natrix_runtime::value::{BinaryOp, CodeHandle, FunctionObject, UnaryOp, Value};
use std::cmp::max;
use std::collections::HashMap;
use std::rc::Rc;

pub fn compile(ctx: &CompilerContext, program: &Program) -> SourceResult<Bytecode> {
    let mut compiler = Compiler::new(ctx);
    // TODO support more functions
    debug_assert!(program.globals.len() == 1);
    let GlobalKind::Function(f) = &program.globals.get(0).unwrap().kind;
    compiler.do_function(&f)?;

    let bb = compiler.bb;
    println!("Code:\n{:?}", bb);
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
        code: bb.encode(),
        globals,
    })
}

struct FunctionInfo {
    max_slots: usize,
}

// TODO: split to two - extract FunctionCompiler
struct Compiler<'ctx> {
    ctx: &'ctx CompilerContext,
    used_slots: usize,
    max_slots: usize,
    local_slots: Vec<usize>, // indexed by LocalId
    functions: Vec<FunctionInfo>,
    loop_labels: HashMap<LoopId, (Label, Label)>, // break target, continue target
    bb: BytecodeBuilder,
}

impl<'ctx> Compiler<'ctx> {
    fn new(ctx: &'ctx CompilerContext) -> Self {
        Self {
            ctx,
            used_slots: 0,
            max_slots: 0,
            local_slots: Vec::new(),
            functions: Vec::new(),
            loop_labels: HashMap::new(),
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

    fn do_block(&mut self, stmts: &[Stmt]) -> SourceResult<()> {
        let saved_slots = self.used_slots;
        for stmt in stmts {
            self.do_stmt(&stmt)?;
        }
        self.used_slots = saved_slots;
        Ok(())
    }

    fn do_stmt(&mut self, stmt: &Stmt) -> SourceResult<()> {
        match &stmt.kind {
            StmtKind::Block(stmts) => self.do_block(&stmts)?,
            StmtKind::Break(loop_id) => {
                let (l_break, _continue) = self.loop_labels[loop_id];
                self.bb.append(stmt.span, InsKind::Jmp(l_break));
            }
            StmtKind::Continue(loop_id) => {
                let (_break, l_continue) = self.loop_labels[loop_id];
                self.bb.append(stmt.span, InsKind::Jmp(l_continue));
            }
            StmtKind::Expr(expr) => {
                self.do_expr(&expr)?;
                self.bb.append(stmt.span, InsKind::Pop);
            }
            StmtKind::If(cond, then_body, else_body) => {
                if let Some(else_body) = else_body {
                    let l_true = self.bb.new_label();
                    let l_false = self.bb.new_label();
                    let l_end = self.bb.new_label();
                    self.do_cond(cond, l_true, l_false, false)?;
                    self.bb.define_label(then_body.span, l_true);
                    self.do_stmt(then_body)?;
                    self.bb.append(then_body.span.tail(), InsKind::Jmp(l_end));
                    self.bb.define_label(else_body.span, l_false);
                    self.do_stmt(else_body)?;
                    self.bb.define_label(else_body.span.tail(), l_end);
                } else {
                    let l_true = self.bb.new_label();
                    let l_false = self.bb.new_label();
                    self.do_cond(cond, l_true, l_false, false)?;
                    self.bb.define_label(then_body.span, l_true);
                    self.do_stmt(then_body)?;
                    self.bb.define_label(then_body.span.tail(), l_false);
                }
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
                let slot = self.local_slot(*id);
                self.do_expr(&expr)?;
                self.bb.append(stmt.span, InsKind::StoreLocal(slot))
            }
            StmtKind::VarDecl(id, expr) => {
                self.local_slots[id.0] = self.used_slots;
                self.used_slots += 1;
                self.max_slots = max(self.max_slots, self.used_slots);
                self.do_expr(&expr)?;
                self.bb
                    .append(stmt.span, InsKind::StoreLocal(self.local_slot(*id)))
            }
            StmtKind::While(loop_id, cond, body) => {
                let l_head = self.bb.new_label();
                let l_body = self.bb.new_label();
                let l_exit = self.bb.new_label();
                self.loop_labels.insert(*loop_id, (l_exit, l_head));
                self.bb.define_label(stmt.span, l_head);
                self.do_cond(cond, l_body, l_exit, false)?;
                self.bb.define_label(body.span, l_body);
                self.do_stmt(&body)?;
                self.bb.append(stmt.span, InsKind::Jmp(l_head));
                self.bb.define_label(body.span.tail(), l_exit);
            }
        }
        Ok(())
    }

    fn do_expr(&mut self, expr: &Expr) -> SourceResult<()> {
        match &expr.kind {
            ExprKind::Binary(op, op_span, left, right) => {
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
                let slot = self.local_slot(*id);
                if slot == 1 {
                    self.bb.append(expr.span, InsKind::Load1)
                } else {
                    self.bb.append(expr.span, InsKind::LoadLocal(slot))
                }
            }
            ExprKind::LogicalBinary(_, op_span, _, _) => {
                let l_true = self.bb.new_label();
                let l_false = self.bb.new_label();
                let l_end = self.bb.new_label();
                self.do_cond(expr, l_true, l_false, false)?;
                self.bb.define_label(*op_span, l_true);
                self.bb.append(*op_span, InsKind::PushTrue);
                self.bb.append(*op_span, InsKind::Jmp(l_end));
                self.bb.define_label(*op_span, l_false);
                self.bb.append(*op_span, InsKind::PushFalse);
                self.bb.define_label(*op_span, l_end);
            }
            ExprKind::Unary(op, op_span, expr) => {
                self.do_expr(&expr)?;
                match op {
                    UnaryOp::Neg => self.bb.append(*op_span, InsKind::Neg),
                    UnaryOp::Not => self.bb.append(*op_span, InsKind::Not),
                }
            }
        }
        Ok(())
    }

    // requirements:
    // - if `expr` evaluates to `negate`, jump to the l_false label, otherwise jump to the l_true label
    // - l_true will be placed right after the code generated by this function
    fn do_cond(
        &mut self,
        expr: &Expr,
        l_true: Label,
        l_false: Label,
        negate: bool,
    ) -> SourceResult<()> {
        match &expr.kind {
            ExprKind::Unary(op, _op_span, expr) if *op == UnaryOp::Not => {
                self.do_cond(expr, l_true, l_false, !negate)
            }
            ExprKind::LogicalBinary(and, op_span, left, right) if *and => {
                let l_rhs = self.bb.new_label();
                if negate {
                    self.do_cond(left, l_rhs, l_true, false)?;
                } else {
                    self.do_cond(left, l_rhs, l_false, false)?;
                }
                self.bb.define_label(*op_span, l_rhs);
                self.do_cond(right, l_true, l_false, negate)
            }
            ExprKind::LogicalBinary(and, op_span, left, right) => {
                let l_rhs = self.bb.new_label();
                if negate {
                    self.do_cond(left, l_rhs, l_false, true)?;
                } else {
                    self.do_cond(left, l_rhs, l_true, true)?;
                }
                self.bb.define_label(*op_span, l_rhs);
                self.do_cond(right, l_true, l_false, negate)
            }
            _ => {
                self.do_expr(&expr)?;
                if negate {
                    self.bb.append(expr.span, InsKind::JTrue(l_false))
                } else {
                    self.bb.append(expr.span, InsKind::JFalse(l_false))
                }
                Ok(())
            }
        }
    }

    fn local_slot(&self, id: LocalId) -> usize {
        // Add one because slot 0 is reserved for the callee function object
        self.local_slots[id.0] + 1
    }
}
