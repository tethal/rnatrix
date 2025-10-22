use crate::ctx::{CompilerContext, Name};
use crate::hir::{
    Expr, ExprKind, FunDecl, GlobalInfo, GlobalKind, LocalInfo, LocalKind, Program, Stmt, StmtKind,
};
use crate::src::Span;
use crate::util::tree::{def_formatter, impl_node_debug};
use std::fmt;
use std::fmt::{Debug, Formatter};

def_formatter!(HirFormatter);

impl_node_debug!(Program as program => ProgramDebug HirFormatter);

impl<'a> Debug for ProgramDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt.header(f, "Program", self.program.span)?;
        for g in self.program.globals.iter() {
            self.fmt.global(f, g)?
        }
        Ok(())
    }
}

impl_node_debug!(GlobalInfo as global => GlobalInfoDebug HirFormatter);

impl<'a> Debug for GlobalInfoDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:?}: ", self.fmt.indent_str(), self.global.id)?;
        self.fmt.name(f, self.global.name)?;
        self.fmt.span(f, self.global.name_span)?;
        write!(f, "\n")?;
        match &self.global.kind {
            GlobalKind::Function(function) => self.fmt.function(f, function),
        }
    }
}

impl_node_debug!(FunDecl as function => FunDeclDebug HirFormatter);

impl<'a> Debug for FunDeclDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}Function:\n", self.fmt.indent_str())?;
        for l in self.function.locals.iter() {
            self.fmt.local(f, l)?
        }
        for stmt in &self.function.body {
            self.fmt.stmt(f, stmt)?;
        }
        Ok(())
    }
}

impl_node_debug!(LocalInfo as local => LocalInfoDebug HirFormatter);

impl<'a> Debug for LocalInfoDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:?}: ", self.fmt.indent_str(), self.local.id)?;
        self.fmt.name(f, self.local.name)?;
        write!(f, " ")?;
        match &self.local.kind {
            LocalKind::Parameter(index) => write!(f, "Param#{:?}", index)?,
            LocalKind::LocalVariable => write!(f, "LocalVariable")?,
        }
        self.fmt.span(f, self.local.name_span)?;
        write!(f, "\n")
    }
}

impl_node_debug!(Stmt as stmt => StmtDebug HirFormatter);

impl<'a> Debug for StmtDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let span = self.stmt.span;
        match &self.stmt.kind {
            StmtKind::Block(stmts) => {
                self.fmt.header(f, "Block", span)?;
                for stmt in stmts {
                    self.fmt.stmt(f, stmt)?;
                }
                Ok(())
            }
            StmtKind::Break(id) => self.fmt.header_with_value(f, "Break", span, id),
            StmtKind::Continue(id) => self.fmt.header_with_value(f, "Continue", span, id),
            StmtKind::Expr(expr) => {
                self.fmt.header(f, "Expr", span)?;
                self.fmt.expr(f, expr)
            }
            StmtKind::If(cond, then_body, else_body) => {
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
                self.fmt.expr(f, expr)
            }
            StmtKind::SetItem(array, index, value) => {
                self.fmt.header(f, "SetItem", span)?;
                self.fmt.expr(f, array)?;
                self.fmt.expr(f, index)?;
                self.fmt.expr(f, value)
            }
            StmtKind::StoreGlobal(id, value) => {
                self.fmt.header_with_value(f, "StoreGlobal", span, id)?;
                self.fmt.expr(f, value)
            }
            StmtKind::StoreLocal(id, value) => {
                self.fmt.header_with_value(f, "StoreLocal", span, id)?;
                self.fmt.expr(f, value)
            }
            StmtKind::VarDecl(id, value) => {
                self.fmt.header_with_value(f, "VarDecl", span, id)?;
                self.fmt.expr(f, value)
            }
            StmtKind::While(id, cond, body) => {
                self.fmt.header_with_value(f, "While", span, id)?;
                self.fmt.expr(f, cond)?;
                self.fmt.stmt(f, body)
            }
        }
    }
}

impl_node_debug!(Expr as expr => ExprDebug HirFormatter);

impl<'a> Debug for ExprDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let span = self.expr.span;
        match &self.expr.kind {
            ExprKind::Binary(op, op_span, left, right) => {
                self.fmt.header_with_value(f, "Binary", *op_span, *op)?;
                self.fmt.expr(f, left)?;
                self.fmt.expr(f, right)
            }
            ExprKind::Call(callee, args) => {
                self.fmt.header(f, "Call", span)?;
                self.fmt.expr(f, callee)?;
                for a in args {
                    self.fmt.expr(f, a)?;
                }
                Ok(())
            }
            ExprKind::ConstBool(value) => self.fmt.header_with_value(f, "ConstBool", span, value),
            ExprKind::ConstFloat(value) => self.fmt.header_with_value(f, "ConstFloat", span, value),
            ExprKind::ConstInt(value) => self.fmt.header_with_value(f, "ConstInt", span, value),
            ExprKind::ConstNull => self.fmt.header(f, "ConstNull", span),
            ExprKind::ConstString(value) => {
                self.fmt.header_with_value(f, "ConstString", span, value)
            }
            ExprKind::GetItem(array, index) => {
                self.fmt.header(f, "GetItem", span)?;
                self.fmt.expr(f, array)?;
                self.fmt.expr(f, index)
            }
            ExprKind::LoadBuiltin(builtin) => {
                self.fmt.header_with_value(f, "LoadBuiltin", span, builtin)
            }
            ExprKind::LoadGlobal(id) => self.fmt.header_with_value(f, "LoadGlobal", span, id),
            ExprKind::LoadLocal(id) => self.fmt.header_with_value(f, "LoadLocal", span, id),
            ExprKind::LogicalBinary(and, op_span, left, right) => {
                self.fmt.header(f, "LogicalBinary", span)?;
                self.fmt.property_with_span(f, "and", and, *op_span)?;
                self.fmt.expr(f, left)?;
                self.fmt.expr(f, right)
            }
            ExprKind::MakeList(vec) => {
                self.fmt.header(f, "MakeList", span)?;
                for e in vec {
                    self.fmt.expr(f, e)?;
                }
                Ok(())
            }
            ExprKind::Unary(op, op_span, expr) => {
                self.fmt.header_with_value(f, "Unary", *op_span, *op)?;
                self.fmt.expr(f, expr)
            }
        }
    }
}
