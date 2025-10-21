use crate::value::Value;
pub use interpreter::Interpreter;

mod interpreter;

#[derive(Debug)]
pub struct Bytecode {
    pub code: Vec<u8>,
    // pub constants: Vec<Value>,
    // line table
    pub globals: Vec<Value>,
    // entry point - just index to globals, or ensure main is first
}

macro_rules! define_opcodes {
    ($($variant:ident => $name:literal);* $(;)?) => {
        #[repr(u8)]
        #[derive(Copy, Clone, Debug)]
        pub enum Opcode {
            $($variant),*
        }

        impl Opcode {
            pub const ALL: &'static [Opcode] = &[
                $(Opcode::$variant),*
            ];

            pub const fn name(self) -> &'static str {
                match self {
                    $(Opcode::$variant => $name),*
                }
            }

            pub fn as_u8(&self) -> u8 {
                *self as u8
            }

            pub fn from_u8(id: u8) -> Option<Self> {
                if (id as usize) < Self::ALL.len() {
                    // SAFETY: we just checked that the value is a valid discriminant
                    Some(unsafe { std::mem::transmute(id) })
                } else {
                    None
                }
            }
        }
    };
}

define_opcodes! {
    Push0 => "push_0";              // 00
    Push1 => "push_1";              // 01
    PushNull => "push_null";        // 02
    PushFalse => "push_false";      // 03
    PushTrue => "push_true";        // 04
    PushInt => "push_int";          // 05
    PushConst => "push_const";      // 06 // N
    Add => "add";                   // 07
    Sub => "sub";                   // 08
    Mul => "mul";                   // 09
    Div => "div";                   // 0A
    Mod => "mod";                   // 0B
    Eq => "eq";                     // 0C
    Ne => "ne";                     // 0D
    Lt => "lt";                     // 0E
    Le => "le";                     // 0F
    Gt => "gt";                     // 10
    Ge => "ge";                     // 11
    Neg => "neg";                   // 12
    Not => "not";                   // 13
    Load0 => "load_0";              // 14
    LoadLocal => "load_local";      // 15 // N
    StoreLocal => "store_local";    // 16 // N
    LoadGlobal => "load_global";    // 17 // N
    StoreGlobal => "store_global";  // 18 // N
    LoadBuiltin => "load_builtin";  // 19 // N
    MakeList => "make_list";        // 1A // N
    GetItem => "get_item";          // 1B
    SetItem => "set_item";          // 1C
    Jmp => "jmp";                   // 1D // offset
    JFalse => "jfalse";             // 1E // offset
    JTrue => "jtrue";               // 1F // offset
    Call => "call";                 // 20 // N
    Ret => "ret";                   // 21
    Pop => "pop";                   // 22
}
