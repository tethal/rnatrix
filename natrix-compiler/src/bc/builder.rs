use crate::src::Span;
use natrix_runtime::bc::Opcode;
use natrix_runtime::leb128::{encode_sleb128, encode_uleb128};
use std::fmt::{Debug, Display};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Label(usize);

impl Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "L{}", self.0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InsKind {
    Add,
    Call(usize),
    Div,
    Eq,
    Ge,
    GetItem,
    Gt,
    JFalse(Label),
    Jmp(Label),
    JTrue(Label),
    LabelDef(Label),
    Le,
    Load0,
    LoadBuiltin(usize),
    LoadGlobal(usize),
    LoadLocal(usize),
    Lt,
    MakeList(usize),
    Mod,
    Mul,
    Ne,
    Neg,
    Not,
    Pop,
    Push0,
    Push1,
    PushConst(usize),
    PushFalse,
    PushInt(i64),
    PushNull,
    PushTrue,
    Ret,
    SetItem,
    StoreGlobal(usize),
    StoreLocal(usize),
    Sub,
}

pub struct Ins {
    pub kind: InsKind,
    pub span: Span,
}

impl Ins {
    pub fn new(kind: InsKind, span: Span) -> Self {
        Self { kind, span }
    }

    fn encoding(&self) -> (Opcode, Immediates) {
        match self.kind {
            InsKind::Add => (Opcode::Add, Immediates::None),
            InsKind::Call(arg_count) => (Opcode::Call, Immediates::Usize(arg_count)),
            InsKind::Div => (Opcode::Div, Immediates::None),
            InsKind::Eq => (Opcode::Eq, Immediates::None),
            InsKind::Ge => (Opcode::Ge, Immediates::None),
            InsKind::GetItem => (Opcode::GetItem, Immediates::None),
            InsKind::Gt => (Opcode::Gt, Immediates::None),
            InsKind::JFalse(label) => (Opcode::JFalse, Immediates::Label(label)),
            InsKind::Jmp(label) => (Opcode::Jmp, Immediates::Label(label)),
            InsKind::JTrue(label) => (Opcode::JTrue, Immediates::Label(label)),
            InsKind::LabelDef(_) => unreachable!(),
            InsKind::Le => (Opcode::Le, Immediates::None),
            InsKind::Load0 => (Opcode::Load0, Immediates::None),
            InsKind::LoadBuiltin(i) => (Opcode::LoadBuiltin, Immediates::Usize(i)),
            InsKind::LoadGlobal(i) => (Opcode::LoadGlobal, Immediates::Usize(i)),
            InsKind::LoadLocal(i) => (Opcode::LoadLocal, Immediates::Usize(i)),
            InsKind::Lt => (Opcode::Lt, Immediates::None),
            InsKind::MakeList(i) => (Opcode::MakeList, Immediates::Usize(i)),
            InsKind::Mod => (Opcode::Mod, Immediates::None),
            InsKind::Mul => (Opcode::Mul, Immediates::None),
            InsKind::Ne => (Opcode::Ne, Immediates::None),
            InsKind::Neg => (Opcode::Neg, Immediates::None),
            InsKind::Not => (Opcode::Not, Immediates::None),
            InsKind::Pop => (Opcode::Pop, Immediates::None),
            InsKind::Push0 => (Opcode::Push0, Immediates::None),
            InsKind::Push1 => (Opcode::Push1, Immediates::None),
            InsKind::PushConst(i) => (Opcode::PushConst, Immediates::Usize(i)),
            InsKind::PushFalse => (Opcode::PushFalse, Immediates::None),
            InsKind::PushInt(v) => (Opcode::PushInt, Immediates::I64(v)),
            InsKind::PushNull => (Opcode::PushNull, Immediates::None),
            InsKind::PushTrue => (Opcode::PushTrue, Immediates::None),
            InsKind::Ret => (Opcode::Ret, Immediates::None),
            InsKind::SetItem => (Opcode::SetItem, Immediates::None),
            InsKind::StoreGlobal(i) => (Opcode::StoreGlobal, Immediates::Usize(i)),
            InsKind::StoreLocal(i) => (Opcode::StoreLocal, Immediates::Usize(i)),
            InsKind::Sub => (Opcode::Sub, Immediates::None),
        }
    }
}

impl Debug for Ins {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let InsKind::LabelDef(label) = self.kind {
            write!(f, "{}:", label)
        } else {
            let (opcode, immediates) = self.encoding();
            write!(f, "  {}", opcode.name())?;
            match immediates {
                Immediates::None => Ok(()),
                Immediates::Usize(i) => write!(f, " {}", i),
                Immediates::I64(i) => write!(f, " {}", i),
                Immediates::Label(label) => write!(f, " {}", label),
            }
        }
    }
}

pub struct BytecodeBuilder {
    pub ins: Vec<Ins>,
    label_count: usize,
}

impl BytecodeBuilder {
    pub fn new() -> Self {
        Self {
            ins: Vec::new(),
            label_count: 0,
        }
    }

    pub fn new_label(&mut self) -> Label {
        let label = Label(self.label_count);
        self.label_count += 1;
        label
    }

    pub fn define_label(&mut self, span: Span, label: Label) {
        assert!(
            !self.ins.iter().any(|i| i.kind == InsKind::LabelDef(label)),
            "label {:?} already defined",
            label
        );
        self.ins.push(Ins::new(InsKind::LabelDef(label), span));
    }

    pub fn append(&mut self, span: Span, ins_kind: InsKind) {
        assert!(!matches!(ins_kind, InsKind::LabelDef(_)));
        self.ins.push(Ins::new(ins_kind, span));
    }

    pub fn encode(&self) -> Vec<u8> {
        let (_, mut label_offsets) = self.encode_pass(|_, _| 0);
        loop {
            let (code, new_label_offsets) =
                self.encode_pass(|from, to_label| label_offsets[to_label.0] as i64 - from as i64);
            if new_label_offsets == label_offsets {
                return code;
            }
            assert!(
                new_label_offsets
                    .iter()
                    .zip(label_offsets.iter())
                    .all(|(n, o)| n >= o),
                "label offsets can only grow"
            );
            label_offsets = new_label_offsets;
        }
    }

    fn encode_pass<F: Fn(usize, Label) -> i64>(&self, calc_delta: F) -> (Vec<u8>, Vec<usize>) {
        let mut label_offsets = Vec::new();
        label_offsets.resize(self.label_count, 0);
        let mut code = Vec::new();
        for ins in self.ins.iter() {
            if let InsKind::LabelDef(label) = ins.kind {
                label_offsets[label.0] = code.len();
            } else {
                let (opcode, immediates) = ins.encoding();
                code.push(opcode as u8);
                match immediates {
                    Immediates::None => {}
                    Immediates::Usize(i) => encode_uleb128(i, |b| code.push(b)),
                    Immediates::I64(i) => encode_sleb128(i, |b| code.push(b)),
                    Immediates::Label(label) => {
                        encode_sleb128(calc_delta(code.len() - 1, label), |b| code.push(b));
                    }
                }
            }
        }
        (code, label_offsets)
    }
}

impl Debug for BytecodeBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ins in &self.ins {
            write!(f, "{:?}\n", ins)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Immediates {
    None,
    Usize(usize),
    I64(i64),
    Label(Label),
}
