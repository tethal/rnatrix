use crate::ctx::Name;
use crate::src::Span;
use std::fmt::Debug;
use std::rc::Rc;

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
    ArrayAccess {
        array: Box<Expr>,
        index: Box<Expr>,
    },
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
    ListLiteral(Vec<Expr>),
    LogicalBinary {
        and: bool,
        op_span: Span,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    NullLiteral,
    Paren(Box<Expr>),
    StringLiteral(Rc<String>),
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
    Break,
    Continue,
    Expr(Expr),
    If {
        cond: Expr,
        then_body: Box<Stmt>,
        else_body: Option<Box<Stmt>>,
    },
    Return(Option<Expr>),
    VarDecl {
        name: Name,
        name_span: Span,
        init: Expr,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
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
