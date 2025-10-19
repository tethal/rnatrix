use crate::error::SourceResult;
use crate::src::Span;
use std::fmt::Debug;

pub struct Ins {
    pub kind: InsKind,
    pub span: Span,
}

impl Ins {
    pub fn new(kind: InsKind, span: Span) -> Self {
        Self { kind, span }
    }
}

pub enum InsKind {
    Add,
    Div,
    Eq,
    Ge,
    Gt,
    Le,
    Load1,
    LoadBuiltin(usize),
    LoadGlobal(usize),
    LoadLocal(usize),
    Lt,
    Mod,
    Mul,
    Ne,
    Neg,
    Not,
    Pop,
    Push0,
    Push1,
    PushFalse,
    PushInt(i64),
    PushNull,
    PushTrue,
    Ret,
    StoreGlobal(usize),
    StoreLocal(usize),
    Sub,
}

pub struct BytecodeBuilder {
    pub ins: Vec<Ins>,
}

impl BytecodeBuilder {
    pub fn new() -> Self {
        Self { ins: Vec::new() }
    }

    pub fn append(&mut self, span: Span, ins_kind: InsKind) -> SourceResult<()> {
        self.ins.push(Ins::new(ins_kind, span));
        Ok(())
    }
}

impl Debug for BytecodeBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ins in &self.ins {
            write!(f, "{:?}\n", ins.kind)?;
        }
        Ok(())
    }
}

impl Debug for InsKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsKind::Add => write!(f, "  add"),
            InsKind::Div => write!(f, "  div"),
            InsKind::Eq => write!(f, "  eq"),
            InsKind::Ge => write!(f, "  ge"),
            InsKind::Gt => write!(f, "  gt"),
            InsKind::Le => write!(f, "  le"),
            InsKind::Load1 => write!(f, "  load_1"),
            InsKind::LoadBuiltin(index) => write!(f, "  load_builtin #{}", index),
            InsKind::LoadGlobal(index) => write!(f, "  load_global #{}", index),
            InsKind::LoadLocal(index) => write!(f, "  load_local #{}", index),
            InsKind::Lt => write!(f, "  lt"),
            InsKind::Mod => write!(f, "  mod"),
            InsKind::Mul => write!(f, "  mul"),
            InsKind::Ne => write!(f, "  ne"),
            InsKind::Neg => write!(f, "  neg"),
            InsKind::Not => write!(f, "  not"),
            InsKind::Pop => write!(f, "  pop"),
            InsKind::Push0 => write!(f, "  push_0"),
            InsKind::Push1 => write!(f, "  push_1"),
            InsKind::PushFalse => write!(f, "  push_false"),
            InsKind::PushInt(v) => write!(f, "  push_int {}", v),
            InsKind::PushNull => write!(f, "  push_null"),
            InsKind::PushTrue => write!(f, "  push_true"),
            InsKind::Ret => write!(f, "  ret"),
            InsKind::StoreGlobal(index) => write!(f, "  store_global #{}", index),
            InsKind::StoreLocal(index) => write!(f, "  store_local #{}", index),
            InsKind::Sub => write!(f, "  sub"),
        }
    }
}
