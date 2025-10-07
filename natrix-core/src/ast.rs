use crate::src::{Sources, Span};
use std::fmt::{self, Debug, Formatter};

pub use crate::ast_debug::AstDebug;

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
}

/// An expression node in the AST.
///
/// # Span Invariant
///
/// The `span` field is `Some` for all AST nodes created by the parser, allowing
/// accurate error reporting and source location tracking. The `Option` wrapper
/// exists to support future AST transformations and synthetic nodes that have no
/// source location (e.g., compiler-generated code, optimizations, desugaring).
///
/// **Parser code may safely `.unwrap()` the span** - a `None` value indicates
/// a bug in the parser itself.
pub struct Expr {
    pub kind: ExprKind,
    pub span: Option<Span>,
}

impl Expr {
    pub fn boxed(kind: ExprKind, span: Span) -> Box<Self> {
        Box::new(Self {
            kind,
            span: Some(span),
        })
    }

    pub fn debug_with<'a>(&'a self, sources: &'a Sources) -> AstDebug<'a> {
        AstDebug::with_sources(self, sources)
    }
}

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        AstDebug::new(self).fmt(f)
    }
}
