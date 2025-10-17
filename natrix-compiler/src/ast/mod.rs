use crate::ctx::Name;
use crate::src::Span;
use natrix_runtime::nx_err::NxResult;
use natrix_runtime::value::Value;
use std::fmt::Debug;
use std::rc::Rc;

mod debug;
mod interpreter;

pub use interpreter::Interpreter;

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

ast_node!(AssignTarget {
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

impl BinaryOp {
    pub fn eval(&self, left: &Value, right: &Value) -> NxResult<Value> {
        match self {
            BinaryOp::Add => left.add(&right),
            BinaryOp::Sub => left.sub(&right),
            BinaryOp::Mul => left.mul(&right),
            BinaryOp::Div => left.div(&right),
            BinaryOp::Mod => left.rem(&right),
            BinaryOp::Eq => left.eq(&right),
            BinaryOp::Ne => left.ne(&right),
            BinaryOp::Ge => left.ge(&right),
            BinaryOp::Gt => left.gt(&right),
            BinaryOp::Le => left.le(&right),
            BinaryOp::Lt => left.lt(&right),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
}

impl UnaryOp {
    pub fn eval(&self, arg: &Value) -> NxResult<Value> {
        match self {
            UnaryOp::Neg => arg.negate(),
            UnaryOp::Not => arg.not(),
        }
    }
}
