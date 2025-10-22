mod scope;

use crate::analyze::scope::{BlockScope, FunctionScope, GlobalScope, LocalScope, Lookup, Symbol};
use crate::ast;
use crate::ctx::CompilerContext;
use crate::error::{err_at, SourceResult};
use crate::hir;
use crate::hir::{GlobalId, GlobalInfo, GlobalKind, LocalKind, LoopId};
use std::rc::Rc;

pub fn analyze(ctx: &CompilerContext, ast: &ast::Program) -> SourceResult<hir::Program> {
    let mut analyzer = Analyzer::new(ctx);
    analyzer.do_program(ast)
}

struct Analyzer<'a> {
    ctx: &'a CompilerContext,
    global_scope: Rc<GlobalScope>,
    next_loop_id: usize,
}

impl<'a> Analyzer<'a> {
    fn new(ctx: &'a CompilerContext) -> Self {
        Self {
            ctx,
            global_scope: GlobalScope::new(ctx),
            next_loop_id: 0,
        }
    }

    fn do_program(&mut self, ast: &ast::Program) -> SourceResult<hir::Program> {
        for (id, ast_decl) in ast.decls.iter().enumerate() {
            self.global_scope
                .declare(self.ctx, ast_decl.name, ast_decl.name_span, GlobalId(id))?;
        }
        let mut globals = Vec::new();
        for (id, ast_decl) in ast.decls.iter().enumerate() {
            globals.push(GlobalInfo::new(
                GlobalId(id),
                ast_decl.name,
                ast_decl.name_span,
                GlobalKind::Function(self.do_fun_decl(&ast_decl)?),
            ));
        }
        Ok(hir::Program::new(globals, ast.span))
    }

    fn do_fun_decl(&mut self, ast: &ast::FunDecl) -> SourceResult<hir::FunDecl> {
        let function_scope = FunctionScope::new(self.global_scope.clone());
        for (i, param) in ast.params.iter().enumerate() {
            function_scope.declare(
                self.ctx,
                param.name,
                param.name_span,
                LocalKind::Parameter(i),
            )?;
        }
        let mut body = self.do_block(function_scope.clone(), None, &ast.body)?;
        if !body
            .last()
            .is_some_and(|s| matches!(s.kind, hir::StmtKind::Return(_)))
        {
            let span = ast.body_span.tail();
            body.push(hir::Stmt::new(
                hir::StmtKind::Return(hir::Expr::new(hir::ExprKind::ConstNull, span)),
                span,
            ));
        }
        Ok(hir::FunDecl::new(
            ast.params.len(),
            function_scope.take_locals(),
            body,
        ))
    }

    fn do_block(
        &mut self,
        scope: Rc<dyn LocalScope>,
        enclosing_loop: Option<LoopId>,
        ast: &Vec<ast::Stmt>,
    ) -> SourceResult<Vec<hir::Stmt>> {
        let block_scope = BlockScope::new(scope);
        let s = ast
            .iter()
            .map(|stmt| self.do_stmt(&block_scope, enclosing_loop, stmt))
            .collect::<SourceResult<Vec<hir::Stmt>>>()?;
        Ok(s)
    }

    fn do_stmt(
        &mut self,
        scope: &Rc<BlockScope>,
        enclosing_loop: Option<LoopId>,
        ast: &ast::Stmt,
    ) -> SourceResult<hir::Stmt> {
        match &ast.kind {
            ast::StmtKind::Assign { target, value } => match &target.kind {
                ast::AssignTargetKind::ArrayAccess { array, index } => {
                    let array = self.do_expr(scope, array)?;
                    let index = self.do_expr(scope, index)?;
                    let value = self.do_expr(scope, value)?;
                    Ok(hir::Stmt::new(
                        hir::StmtKind::SetItem(array, index, value),
                        ast.span,
                    ))
                }
                ast::AssignTargetKind::Var(name) => {
                    let symbol = scope.lookup(self.ctx, name, target.span)?;
                    let value = self.do_expr(scope, value)?;
                    match symbol {
                        Symbol::Builtin(_) => {
                            err_at(target.span, "built-in function cannot be assigned to")
                        }
                        Symbol::Global(id) => Ok(hir::Stmt::new(
                            hir::StmtKind::StoreGlobal(id, value),
                            target.span,
                        )),
                        Symbol::Local(id) => Ok(hir::Stmt::new(
                            hir::StmtKind::StoreLocal(id, value),
                            target.span,
                        )),
                    }
                }
            },
            ast::StmtKind::Block(stmts) => Ok(hir::Stmt::new(
                hir::StmtKind::Block(self.do_block(scope.clone(), enclosing_loop, stmts)?),
                ast.span,
            )),
            ast::StmtKind::Break => {
                if let Some(loop_id) = enclosing_loop {
                    Ok(hir::Stmt::new(hir::StmtKind::Break(loop_id), ast.span))
                } else {
                    err_at(ast.span, "break outside a loop")
                }
            }
            ast::StmtKind::Continue => {
                if let Some(loop_id) = enclosing_loop {
                    Ok(hir::Stmt::new(hir::StmtKind::Continue(loop_id), ast.span))
                } else {
                    err_at(ast.span, "continue outside a loop")
                }
            }
            ast::StmtKind::Expr(expr) => {
                let expr = self.do_expr(scope, expr)?;
                Ok(hir::Stmt::new(hir::StmtKind::Expr(expr), ast.span))
            }
            ast::StmtKind::If {
                cond,
                then_body,
                else_body,
            } => {
                let cond = self.do_expr(scope, cond)?;
                let then_body = self.do_stmt(&scope, enclosing_loop, then_body)?;
                let else_body = if let Some(stmt) = else_body {
                    Some(self.do_stmt(&scope, enclosing_loop, stmt)?)
                } else {
                    None
                };
                Ok(hir::Stmt::new(
                    hir::StmtKind::If(cond, Box::new(then_body), else_body.map(Box::new)),
                    ast.span,
                ))
            }
            ast::StmtKind::Return(expr) => {
                let e = match expr {
                    Some(e) => self.do_expr(scope, e)?,
                    None => hir::Expr::new(hir::ExprKind::ConstNull, ast.span),
                };
                Ok(hir::Stmt::new(hir::StmtKind::Return(e), ast.span))
            }
            ast::StmtKind::VarDecl {
                name,
                name_span,
                init,
            } => {
                let value = self.do_expr(&scope, init)?;
                let id = scope.declare(self.ctx, *name, *name_span, LocalKind::LocalVariable)?;
                Ok(hir::Stmt::new(hir::StmtKind::VarDecl(id, value), ast.span))
            }
            ast::StmtKind::While { cond, body } => {
                let loop_id = LoopId(self.next_loop_id);
                self.next_loop_id += 1;
                let cond = self.do_expr(scope, cond)?;
                let body = self.do_stmt(&scope, Some(loop_id), body)?;
                Ok(hir::Stmt::new(
                    hir::StmtKind::While(loop_id, cond, Box::new(body)),
                    ast.span,
                ))
            }
        }
    }

    fn do_expr(&mut self, scope: &Rc<BlockScope>, ast: &ast::Expr) -> SourceResult<hir::Expr> {
        match &ast.kind {
            ast::ExprKind::ArrayAccess { array, index } => {
                let array = self.do_expr(scope, array)?;
                let index = self.do_expr(scope, index)?;
                Ok(hir::Expr::new(
                    hir::ExprKind::GetItem(Box::new(array), Box::new(index)),
                    ast.span,
                ))
            }
            ast::ExprKind::Binary {
                op,
                op_span,
                left,
                right,
            } => {
                let left = self.do_expr(scope, left)?;
                let right = self.do_expr(scope, right)?;
                Ok(hir::Expr::new(
                    hir::ExprKind::Binary(*op, *op_span, Box::new(left), Box::new(right)),
                    ast.span,
                ))
            }
            ast::ExprKind::BoolLiteral(v) => {
                Ok(hir::Expr::new(hir::ExprKind::ConstBool(*v), ast.span))
            }
            ast::ExprKind::Call { callee, args } => {
                let callee = self.do_expr(scope, callee)?;
                let args = args
                    .iter()
                    .map(|arg| self.do_expr(scope, arg))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(hir::Expr::new(
                    hir::ExprKind::Call(Box::new(callee), args),
                    ast.span,
                ))
            }
            ast::ExprKind::FloatLiteral(v) => {
                Ok(hir::Expr::new(hir::ExprKind::ConstFloat(*v), ast.span))
            }
            ast::ExprKind::IntLiteral(v) => {
                Ok(hir::Expr::new(hir::ExprKind::ConstInt(*v), ast.span))
            }
            ast::ExprKind::ListLiteral(elements) => {
                let elements = elements
                    .iter()
                    .map(|e| self.do_expr(scope, e))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(hir::Expr::new(hir::ExprKind::MakeList(elements), ast.span))
            }
            ast::ExprKind::LogicalBinary {
                and,
                op_span,
                left,
                right,
            } => {
                let left = self.do_expr(scope, left)?;
                let right = self.do_expr(scope, right)?;
                Ok(hir::Expr::new(
                    hir::ExprKind::LogicalBinary(*and, *op_span, Box::new(left), Box::new(right)),
                    ast.span,
                ))
            }
            ast::ExprKind::NullLiteral => Ok(hir::Expr::new(hir::ExprKind::ConstNull, ast.span)),
            ast::ExprKind::Paren(expr) => self.do_expr(scope, expr),
            ast::ExprKind::StringLiteral(v) => Ok(hir::Expr::new(
                hir::ExprKind::ConstString(v.clone()),
                ast.span,
            )),
            ast::ExprKind::Unary { op, op_span, expr } => {
                let expr = self.do_expr(scope, expr)?;
                Ok(hir::Expr::new(
                    hir::ExprKind::Unary(*op, *op_span, Box::new(expr)),
                    ast.span,
                ))
            }
            ast::ExprKind::Var(name) => Ok(hir::Expr::new(
                match scope.lookup(self.ctx, name, ast.span)? {
                    Symbol::Builtin(builtin) => hir::ExprKind::LoadBuiltin(builtin),
                    Symbol::Global(id) => hir::ExprKind::LoadGlobal(id),
                    Symbol::Local(id) => hir::ExprKind::LoadLocal(id),
                },
                ast.span,
            )),
        }
    }
}
