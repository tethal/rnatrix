use crate::src::Span;
use std::fmt::{self, Debug, Formatter};

use crate::ast_debug::{ExprDebug, StmtDebug};
use crate::ctx::{CompilerContext, Name};

pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

pub enum StmtKind {
    Assign {
        left: Expr,
        right: Expr,
    },
    Block(Vec<Stmt>),
    Expr(Expr),
    VarDecl {
        name: Name,
        name_span: Span,
        init: Expr,
    },
}

impl Stmt {
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn debug_with<'a>(&'a self, ctx: &'a CompilerContext) -> StmtDebug<'a> {
        StmtDebug::with_context(self, ctx)
    }
}

impl Debug for Stmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        StmtDebug::new(self).fmt(f)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Copy, Clone)]
pub enum UnaryOp {
    Neg,
}

pub enum ExprKind {
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    NullLiteral,
    Paren(Box<Expr>),
    Unary {
        op: UnaryOp,
        op_span: Span,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        op_span: Span,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Var(Name),
}

pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn debug_with<'a>(&'a self, ctx: &'a CompilerContext) -> ExprDebug<'a> {
        ExprDebug::with_context(self, ctx)
    }

    pub fn is_lvalue(&self) -> bool {
        matches!(self.kind, ExprKind::Var(_))
    }
}

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        ExprDebug::new(self).fmt(f)
    }
}
