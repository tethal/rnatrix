mod scope;

use crate::analyze::scope::{BlockScope, FunctionScope, GlobalScope, LocalScope, Lookup, Symbol};
use crate::ast;
use crate::ctx::CompilerContext;
use crate::error::{err_at, SourceResult};
use crate::hir;
use crate::hir::{GlobalId, GlobalInfo, GlobalKind, LocalKind};
use std::rc::Rc;

pub fn analyze(ctx: &CompilerContext, ast: &ast::Program) -> SourceResult<hir::Program> {
    let global_scope = GlobalScope::new(ctx);
    for (id, ast_decl) in ast.decls.iter().enumerate() {
        global_scope.declare(ctx, ast_decl.name, ast_decl.name_span, GlobalId(id))?;
    }
    let mut globals = Vec::new();
    for (id, ast_decl) in ast.decls.iter().enumerate() {
        globals.push(GlobalInfo::new(
            GlobalId(id),
            ast_decl.name,
            ast_decl.name_span,
            GlobalKind::Function(do_fun_decl(ctx, global_scope.clone(), &ast_decl)?),
        ));
    }
    Ok(hir::Program::new(globals, ast.span))
}

fn do_fun_decl(
    ctx: &CompilerContext,
    global_scope: Rc<GlobalScope>,
    ast: &ast::FunDecl,
) -> SourceResult<hir::Function> {
    let function_scope = FunctionScope::new(global_scope.clone());
    for (i, param) in ast.params.iter().enumerate() {
        function_scope.declare(ctx, param.name, param.name_span, LocalKind::Parameter(i))?;
    }
    let mut body = do_block(ctx, function_scope.clone(), &ast.body)?;
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
    Ok(hir::Function::new(
        ast.params.len(),
        function_scope.take_locals(),
        body,
    ))
}

fn do_block(
    ctx: &CompilerContext,
    scope: Rc<dyn LocalScope>,
    ast: &Vec<ast::Stmt>,
) -> SourceResult<Vec<hir::Stmt>> {
    let block_scope = BlockScope::new(scope);
    let s = ast
        .iter()
        .map(|stmt| do_stmt(ctx, &block_scope, stmt))
        .collect::<SourceResult<Vec<hir::Stmt>>>()?;
    Ok(s)
}

fn do_stmt(
    ctx: &CompilerContext,
    scope: &Rc<BlockScope>,
    ast: &ast::Stmt,
) -> SourceResult<hir::Stmt> {
    match &ast.kind {
        ast::StmtKind::Assign { target, value } => match &target.kind {
            ast::AssignTargetKind::ArrayAccess { array: _, index: _ } => todo!("Array access"),
            ast::AssignTargetKind::Var(name) => {
                let symbol = scope.lookup(ctx, name, target.span)?;
                let value = do_expr(ctx, scope, value)?;
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
            hir::StmtKind::Block(do_block(ctx, scope.clone(), stmts)?),
            ast.span,
        )),
        // Break
        // Continue
        ast::StmtKind::Expr(expr) => {
            let expr = do_expr(ctx, scope, expr)?;
            Ok(hir::Stmt::new(hir::StmtKind::Expr(expr), ast.span))
        }
        // If
        ast::StmtKind::Return(expr) => {
            let e = match expr {
                Some(e) => do_expr(ctx, scope, e)?,
                None => hir::Expr::new(hir::ExprKind::ConstNull, ast.span),
            };
            Ok(hir::Stmt::new(hir::StmtKind::Return(e), ast.span))
        }
        ast::StmtKind::VarDecl {
            name,
            name_span,
            init,
        } => {
            let value = do_expr(ctx, &scope, init)?;
            let id = scope.declare(ctx, *name, *name_span, LocalKind::LocalVariable)?;
            Ok(hir::Stmt::new(hir::StmtKind::VarDecl(id, value), ast.span))
        }
        // While
        _ => todo!("{:?}", ast),
    }
}

fn do_expr(
    ctx: &CompilerContext,
    scope: &Rc<BlockScope>,
    ast: &ast::Expr,
) -> SourceResult<hir::Expr> {
    match &ast.kind {
        // ArrayAccess
        ast::ExprKind::Binary {
            op,
            op_span,
            left,
            right,
        } => {
            let left = do_expr(ctx, scope, left)?;
            let right = do_expr(ctx, scope, right)?;
            Ok(hir::Expr::new(
                hir::ExprKind::Binary {
                    op: *op,
                    op_span: *op_span,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                ast.span,
            ))
        }
        ast::ExprKind::BoolLiteral(v) => Ok(hir::Expr::new(hir::ExprKind::ConstBool(*v), ast.span)),
        // Call
        // FloatLiteral
        ast::ExprKind::IntLiteral(v) => Ok(hir::Expr::new(hir::ExprKind::ConstInt(*v), ast.span)),
        // ListLiteral
        // LogicalBinary
        ast::ExprKind::NullLiteral => Ok(hir::Expr::new(hir::ExprKind::ConstNull, ast.span)),
        ast::ExprKind::Paren(expr) => do_expr(ctx, scope, expr),
        // StringLiteral
        ast::ExprKind::Unary { op, op_span, expr } => {
            let expr = do_expr(ctx, scope, expr)?;
            Ok(hir::Expr::new(
                hir::ExprKind::Unary {
                    op: *op,
                    op_span: *op_span,
                    expr: Box::new(expr),
                },
                ast.span,
            ))
        }
        ast::ExprKind::Var(name) => Ok(hir::Expr::new(
            match scope.lookup(ctx, name, ast.span)? {
                Symbol::Builtin(builtin) => hir::ExprKind::LoadBuiltin(builtin),
                Symbol::Global(id) => hir::ExprKind::LoadGlobal(id),
                Symbol::Local(id) => hir::ExprKind::LoadLocal(id),
            },
            ast.span,
        )),
        _ => todo!("{:?}", ast),
    }
}
