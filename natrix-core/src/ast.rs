use crate::src::Span;
use std::fmt::Debug;

use crate::ctx::Name;

macro_rules! ast_node {
    ($name:ident { $($field_name:ident: $field_type:ty),+ $(,)? }) => {
        pub struct $name {
            $(pub $field_name: $field_type,)+
        }

        impl $name {
            pub fn new($($field_name: $field_type),+) -> Self {
                Self { $($field_name),+ }
            }
        }
    };
}

ast_node!(Program {
    decls: Vec<FunDecl>,
    span: Span,
});

ast_node!(FunDecl {
    name: Name,
    name_span: Span,
    params: Vec<Param>,
    body: Stmt,
});

ast_node!(Param {
    name: Name,
    name_span: Span,
});

ast_node!(Stmt {
    kind: StmtKind,
    span: Span,
});

ast_node!(Expr {
    kind: ExprKind,
    span: Span,
});

pub enum ExprKind {
    Binary {
        op: BinaryOp,
        op_span: Span,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    BoolLiteral(bool),
    Call {
        name: Name,
        name_span: Span,
        args: Vec<Expr>,
    },
    FloatLiteral(f64),
    IntLiteral(i64),
    LogicalBinary {
        and: bool,
        op_span: Span,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    NullLiteral,
    Paren(Box<Expr>),
    Unary {
        op: UnaryOp,
        op_span: Span,
        expr: Box<Expr>,
    },
    Var(Name),
}

pub enum StmtKind {
    Assign {
        left: Expr,
        right: Expr,
    },
    Block(Vec<Stmt>),
    Expr(Expr),
    Print(Expr),
    Return(Option<Expr>),
    VarDecl {
        name: Name,
        name_span: Span,
        init: Expr,
    },
}

#[derive(Debug, Copy, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Copy, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
}
