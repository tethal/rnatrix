mod debug;

use crate::ctx::Name;
use crate::src::Span;
use crate::util::tree::def_node;
use natrix_runtime::value::{BinaryOp, Builtin, UnaryOp};
use std::rc::Rc;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct GlobalId(pub usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LoopId(pub usize);

def_node!(Program {
    globals: Vec<GlobalInfo>,
    span: Span,
});

def_node!(GlobalInfo {
    id: GlobalId,
    name: Name,
    name_span: Span,
    kind: GlobalKind,
});

pub enum GlobalKind {
    Function(FunDecl),
}

def_node!(FunDecl {
    param_count: usize,
    locals: Vec<LocalInfo>, // invariant - first param_count elements are LocalKind::Parameter
    body: Vec<Stmt>,
});

def_node!(LocalInfo {
    id: LocalId,
    name: Name,
    name_span: Span,
    kind: LocalKind,
});

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LocalKind {
    Parameter(usize),
    LocalVariable,
}

def_node!(Stmt {
    kind: StmtKind,
    span: Span
});

pub enum StmtKind {
    Block(Vec<Stmt>),
    Break(LoopId),
    Continue(LoopId),
    Expr(Expr),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    Return(Expr),
    SetItem(Expr, Expr, Expr),
    StoreGlobal(GlobalId, Expr),
    StoreLocal(LocalId, Expr),
    VarDecl(LocalId, Expr),
    While(LoopId, Expr, Box<Stmt>),
}

def_node!(Expr {
    kind: ExprKind,
    span: Span
});

pub enum ExprKind {
    Binary(BinaryOp, Span, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    ConstBool(bool),
    ConstFloat(f64),
    ConstInt(i64),
    ConstNull,
    ConstString(Rc<str>),
    GetItem(Box<Expr>, Box<Expr>),
    LoadBuiltin(Builtin),
    LoadGlobal(GlobalId),
    LoadLocal(LocalId),
    LogicalBinary(bool, Span, Box<Expr>, Box<Expr>),
    MakeList(Vec<Expr>),
    Unary(UnaryOp, Span, Box<Expr>),
}
