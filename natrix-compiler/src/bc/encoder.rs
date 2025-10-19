use crate::bc::builder::{BytecodeBuilder, InsKind};
use natrix_runtime::bc::Opcode;

pub fn encode(bb: &BytecodeBuilder) -> Vec<u8> {
    let mut code = Vec::new();

    macro_rules! sleb {
        ($value:expr) => {{
            let mut value = $value;
            loop {
                let byte = value & 0x7f;
                value >>= 7;
                let sign_bit = byte & 0x40;
                if (value == 0 && sign_bit == 0) || (value == -1 && sign_bit != 0) {
                    code.push(byte as u8);
                    break;
                } else {
                    code.push((byte | 0x80) as u8);
                }
            }
        }};
    }

    macro_rules! uleb {
        ($value:expr) => {{
            let mut value = $value;
            loop {
                let mut byte = value & 0x7f;
                value >>= 7;
                if value != 0 {
                    byte |= 0x80;
                }
                code.push(byte as u8);
                if value == 0 {
                    break;
                }
            }
        }};
    }

    for ins in bb.ins.iter() {
        match &ins.kind {
            // TODO macros, move to builder
            InsKind::Add => code.push(Opcode::Add as u8),
            InsKind::Div => code.push(Opcode::Div as u8),
            InsKind::Eq => code.push(Opcode::Eq as u8),
            InsKind::Ge => code.push(Opcode::Ge as u8),
            InsKind::Gt => code.push(Opcode::Gt as u8),
            InsKind::Le => code.push(Opcode::Le as u8),
            InsKind::Load1 => code.push(Opcode::Load1 as u8),
            InsKind::LoadBuiltin(index) => {
                code.push(Opcode::LoadBuiltin as u8);
                uleb!(*index)
            }
            InsKind::LoadGlobal(index) => {
                code.push(Opcode::LoadGlobal as u8);
                uleb!(*index)
            }
            InsKind::LoadLocal(index) => {
                code.push(Opcode::LoadLocal as u8);
                uleb!(*index)
            }
            InsKind::Lt => code.push(Opcode::Lt as u8),
            InsKind::Mod => code.push(Opcode::Mod as u8),
            InsKind::Mul => code.push(Opcode::Mul as u8),
            InsKind::Ne => code.push(Opcode::Ne as u8),
            InsKind::Neg => code.push(Opcode::Neg as u8),
            InsKind::Not => code.push(Opcode::Not as u8),
            InsKind::Pop => code.push(Opcode::Pop as u8),
            InsKind::Push0 => code.push(Opcode::Push0 as u8),
            InsKind::Push1 => code.push(Opcode::Push1 as u8),
            InsKind::PushFalse => code.push(Opcode::PushFalse as u8),
            InsKind::PushInt(v) => {
                code.push(Opcode::PushInt as u8);
                sleb!(*v);
            }
            InsKind::PushNull => code.push(Opcode::PushNull as u8),
            InsKind::PushTrue => code.push(Opcode::PushTrue as u8),
            InsKind::Ret => code.push(Opcode::Ret as u8),
            InsKind::StoreGlobal(index) => {
                code.push(Opcode::StoreGlobal as u8);
                uleb!(*index)
            }
            InsKind::StoreLocal(index) => {
                code.push(Opcode::StoreLocal as u8);
                uleb!(*index)
            }
            InsKind::Sub => code.push(Opcode::Sub as u8),
        }
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::src::Span;

    fn sleb(val: i64) -> Vec<u8> {
        let mut bb = BytecodeBuilder::new();
        bb.append(Span::DUMMY, InsKind::PushInt(val)).unwrap();
        let mut bytes = encode(&bb);
        assert_eq!(bytes.remove(0), Opcode::PushInt.as_u8());
        bytes
    }

    fn uleb(val: usize) -> Vec<u8> {
        let mut bb = BytecodeBuilder::new();
        bb.append(Span::DUMMY, InsKind::LoadBuiltin(val)).unwrap();
        let mut bytes = encode(&bb);
        assert_eq!(bytes.remove(0), Opcode::LoadBuiltin.as_u8());
        bytes
    }

    #[test]
    fn test_sleb() {
        assert_eq!(sleb(0), vec![0x00]);
        assert_eq!(sleb(1), vec![0x01]);
        assert_eq!(sleb(63), vec![0x3f]);
        assert_eq!(sleb(64), vec![0xC0, 0x00]);
        assert_eq!(sleb(65), vec![0xC1, 0x00]);
        assert_eq!(sleb(127), vec![0xFF, 0x00]);
        assert_eq!(sleb(128), vec![0x80, 0x01]);
        assert_eq!(sleb(123456), vec![0xC0, 0xC4, 0x07]);
        assert_eq!(
            sleb(i64::MAX),
            vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]
        );
        assert_eq!(sleb(-1), vec![0x7F]);
        assert_eq!(sleb(-2), vec![0x7E]);
        assert_eq!(sleb(-63), vec![0x41]);
        assert_eq!(sleb(-64), vec![0x40]);
        assert_eq!(sleb(-65), vec![0xBF, 0x7F]);
        assert_eq!(sleb(-66), vec![0xBE, 0x7F]);
        assert_eq!(sleb(-128), vec![0x80, 0x7F]);
        assert_eq!(sleb(-129), vec![0xFF, 0x7E]);
        assert_eq!(sleb(-123456), vec![0xC0, 0xBB, 0x78]);
        assert_eq!(
            sleb(i64::MIN),
            vec![0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x7F]
        );
    }

    #[test]
    fn test_uleb() {
        assert_eq!(uleb(0), vec![0x00]);
        assert_eq!(uleb(1), vec![0x01]);
        assert_eq!(uleb(64), vec![0x40]);
        assert_eq!(uleb(127), vec![0x7F]);
        assert_eq!(uleb(128), vec![0x80, 0x01]);
        assert_eq!(uleb(624485), vec![0xE5, 0x8E, 0x26]);
        assert_eq!(
            uleb(usize::MAX),
            vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]
        );
    }
}
