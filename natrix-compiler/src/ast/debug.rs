use crate::ast::{
    AssignTarget, AssignTargetKind, Expr, ExprKind, FunDecl, Param, Program, Stmt, StmtKind,
};
use crate::ctx::{CompilerContext, Name};
use crate::src::Span;
use crate::util::tree::{def_formatter, impl_node_debug};
use std::fmt::{self, Debug, Formatter};

def_formatter!(AstFormatter);

impl_node_debug!(Program as program => ProgramDebug AstFormatter);

impl<'a> Debug for ProgramDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt.header(f, "Program", self.program.span)?;
        for d in self.program.decls.iter() {
            self.fmt.fun_decl(f, d)?
        }
        Ok(())
    }
}

impl_node_debug!(FunDecl as fun_decl => FunDeclDebug AstFormatter);

impl<'a> Debug for FunDeclDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt
            .header_with_name(f, "FunDecl", self.fun_decl.name_span, self.fun_decl.name)?;
        for p in self.fun_decl.params.iter() {
            self.fmt.param(f, p)?
        }
        self.fmt.stmt(f, &self.fun_decl.body)
    }
}

impl_node_debug!(Param as param => ParamDebug AstFormatter);

impl<'a> Debug for ParamDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt
            .header_with_name(f, "Param", self.param.name_span, self.param.name)
    }
}

impl_node_debug!(Stmt as stmt => StmtDebug AstFormatter);

impl Debug for StmtDebug<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let span = self.stmt.span;
        match &self.stmt.kind {
            StmtKind::Assign { target, value } => {
                self.fmt.header(f, "Assign", span)?;
                self.fmt.assign_target(f, target)?;
                self.fmt.expr(f, value)
            }
            StmtKind::Block(stmts) => {
                self.fmt.header(f, "Block", span)?;
                for stmt in stmts {
                    self.fmt.stmt(f, stmt)?;
                }
                Ok(())
            }
            StmtKind::Break => self.fmt.header(f, "Break", span),
            StmtKind::Continue => self.fmt.header(f, "Continue", span),
            StmtKind::Expr(expr) => {
                self.fmt.header(f, "Expr", span)?;
                self.fmt.expr(f, expr)
            }
            StmtKind::If {
                cond,
                then_body,
                else_body,
            } => {
                self.fmt.header(f, "If", span)?;
                self.fmt.expr(f, cond)?;
                self.fmt.stmt(f, then_body)?;
                if let Some(else_body) = else_body {
                    self.fmt.stmt(f, else_body)?;
                };
                Ok(())
            }
            StmtKind::Return(expr) => {
                self.fmt.header(f, "Return", span)?;
                if let Some(expr) = expr {
                    self.fmt.expr(f, expr)
                } else {
                    Ok(())
                }
            }
            StmtKind::VarDecl {
                name,
                name_span,
                init,
            } => {
                self.fmt.header(f, "VarDecl", span)?;
                self.fmt
                    .property_name_with_span(f, "name", *name, *name_span)?;
                self.fmt.expr(f, init)
            }
            StmtKind::While { cond, body } => {
                self.fmt.header(f, "While", span)?;
                self.fmt.expr(f, cond)?;
                self.fmt.stmt(f, body)?;
                Ok(())
            }
        }
    }
}

impl_node_debug!(Expr as expr => ExprDebug AstFormatter);

impl Debug for ExprDebug<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let span = self.expr.span;
        match &self.expr.kind {
            ExprKind::ArrayAccess { array, index } => {
                self.fmt.header(f, "ArrayAccess", span)?;
                self.fmt.expr(f, array)?;
                self.fmt.expr(f, index)
            }
            ExprKind::Binary {
                op,
                op_span,
                left,
                right,
            } => {
                self.fmt.header(f, "Binary", span)?;
                self.fmt.property_with_span(f, "op", *op, *op_span)?;
                self.fmt.expr(f, left)?;
                self.fmt.expr(f, right)
            }
            ExprKind::BoolLiteral(value) => {
                self.fmt.header_with_value(f, "BoolLiteral", span, value)
            }
            ExprKind::Call { callee, args } => {
                self.fmt.header(f, "Call", span)?;
                self.fmt.expr(f, callee)?;
                for a in args {
                    self.fmt.expr(f, a)?;
                }
                Ok(())
            }
            ExprKind::FloatLiteral(value) => {
                self.fmt.header_with_value(f, "FloatLiteral", span, value)
            }
            ExprKind::IntLiteral(value) => self.fmt.header_with_value(f, "IntLiteral", span, value),
            ExprKind::ListLiteral(vec) => {
                self.fmt.header(f, "ListLiteral", span)?;
                for e in vec {
                    self.fmt.expr(f, e)?;
                }
                Ok(())
            }
            ExprKind::LogicalBinary {
                and,
                op_span,
                left,
                right,
            } => {
                self.fmt.header(f, "LogicalBinary", span)?;
                self.fmt.property_with_span(f, "and", and, *op_span)?;
                self.fmt.expr(f, left)?;
                self.fmt.expr(f, right)
            }
            ExprKind::NullLiteral => self.fmt.header(f, "NullLiteral", span),
            ExprKind::Paren(inner) => {
                self.fmt.header(f, "Paren", span)?;
                self.fmt.expr(f, inner)
            }
            ExprKind::StringLiteral(value) => {
                self.fmt.header_with_value(f, "StringLiteral", span, value)
            }
            ExprKind::Unary { op, op_span, expr } => {
                self.fmt.header(f, "Unary", span)?;
                self.fmt.property_with_span(f, "op", *op, *op_span)?;
                self.fmt.expr(f, expr)
            }
            ExprKind::Var(name) => self.fmt.header_with_name(f, "Var", span, *name),
        }
    }
}

impl_node_debug!(AssignTarget as assign_target => AssignTargetDebug AstFormatter);

impl Debug for AssignTargetDebug<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let span = self.assign_target.span;
        match &self.assign_target.kind {
            AssignTargetKind::ArrayAccess { array, index } => {
                self.fmt.header(f, "ArrayAccess", span)?;
                self.fmt.expr(f, array)?;
                self.fmt.expr(f, index)
            }
            AssignTargetKind::Var(name) => self.fmt.header_with_name(f, "Var", span, *name),
        }
    }
}
