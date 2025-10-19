use crate::ctx::Name;
use crate::src::Span;
use crate::util::tree::def_node;
pub use interpreter::Interpreter;
use natrix_runtime::value::{BinaryOp, UnaryOp};
use std::rc::Rc;

mod debug;
mod interpreter;

def_node!(Program {
    decls: Vec<FunDecl>,
    span: Span,
});

def_node!(FunDecl {
    name: Name,
    name_span: Span,
    params: Vec<Param>,
    body: Vec<Stmt>,
    body_span: Span,
});

def_node!(Param {
    name: Name,
    name_span: Span,
});

def_node!(Stmt {
    kind: StmtKind,
    span: Span,
});

def_node!(Expr {
    kind: ExprKind,
    span: Span,
});

def_node!(AssignTarget {
    kind: AssignTargetKind,
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
        callee: Box<Expr>,
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
    StringLiteral(Rc<str>),
    Unary {
        op: UnaryOp,
        op_span: Span,
        expr: Box<Expr>,
    },
    Var(Name),
}

pub enum AssignTargetKind {
    ArrayAccess { array: Box<Expr>, index: Box<Expr> },
    Var(Name),
}

pub enum StmtKind {
    Assign {
        target: AssignTarget,
        value: Expr,
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
