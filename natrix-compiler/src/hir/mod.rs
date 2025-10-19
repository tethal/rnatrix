mod debug;

use crate::ctx::Name;
use crate::src::Span;
use crate::util::tree::def_node;
use natrix_runtime::runtime::Builtin;
use natrix_runtime::value::{BinaryOp, UnaryOp};

#[derive(Debug, Copy, Clone)]
pub struct GlobalId(pub usize);
#[derive(Debug, Copy, Clone)]
pub struct LocalId(pub usize);

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
    Function(Function),
}

def_node!(Function {
    locals: Vec<LocalInfo>,
    body: Vec<Stmt>,
});

def_node!(LocalInfo {
    id: LocalId,
    name: Name,
    name_span: Span,
    kind: LocalKind,
});

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
    Expr(Expr),
    Return(Expr),
    StoreGlobal(GlobalId, Expr),
    StoreLocal(LocalId, Expr),
    VarDecl(LocalId, Expr),
}

def_node!(Expr {
    kind: ExprKind,
    span: Span
});

pub enum ExprKind {
    Binary {
        op: BinaryOp,
        op_span: Span,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    ConstBool(bool),
    ConstInt(i64),
    ConstNull,
    LoadBuiltin(Builtin),
    LoadGlobal(GlobalId),
    LoadLocal(LocalId),
    Unary {
        op: UnaryOp,
        op_span: Span,
        expr: Box<Expr>,
    },
}
