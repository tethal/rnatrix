use crate::bc::{Bytecode, Opcode};
use crate::nx_err::NxResult;
use crate::runtime::{Builtin, Runtime};
use crate::value::Value;
use std::rc::Rc;

pub struct Interpreter<'rt> {
    rt: &'rt mut Runtime,
}

impl<'rt> Interpreter<'rt> {
    pub fn new(rt: &'rt mut Runtime) -> Self {
        Self { rt }
    }

    fn prepare_builtins() -> Vec<Value> {
        Builtin::ALL
            .iter()
            .map(|b| Value::from_function(Rc::new(b.as_function_object())))
            .collect()
    }

    pub fn run(&mut self, bc: &Bytecode, mut args: Vec<Value>) -> NxResult<Value> {
        let builtins = Self::prepare_builtins();
        let mut globals = bc.globals.clone();
        let main = &globals[0].unwrap_function();

        let code = &bc.code;
        let mut ip = main.code_handle.0;
        let mut stack = Vec::new();
        let arg_cnt = args.len();
        let fp = 0;
        stack.push(Value::NULL); // TODO push main function object
        stack.append(&mut args);
        assert_eq!(arg_cnt, main.param_count);
        stack.resize(stack.len() + main.max_slots - arg_cnt, Value::NULL);
        // TODO do what "call arg_cnt" does - check args count, push frame, setup FP

        macro_rules! fetch_u8 {
            () => {{
                let r = code[ip];
                ip += 1;
                r
            }};
        }

        macro_rules! fetch_sleb {
            () => {{
                let mut result = 0i64;
                let mut shift = 0;
                let mut byte;
                loop {
                    byte = fetch_u8!();
                    result |= ((byte & 0x7f) as i64) << shift;
                    shift += 7;
                    if (byte & 0x80) == 0 {
                        break;
                    }
                }
                if shift < 64 && (byte & 0x40) != 0 {
                    result |= (!0i64 << shift);
                }
                result
            }};
        }

        macro_rules! fetch_uleb {
            () => {{
                let mut result = 0usize;
                let mut shift = 0;
                let mut byte;
                loop {
                    byte = fetch_u8!();
                    result |= ((byte & 0x7f) as usize) << shift;
                    shift += 7;
                    if (byte & 0x80) == 0 {
                        break;
                    }
                }
                result
            }};
        }
        macro_rules! pop {
            () => {
                stack.pop().unwrap()
            };
        }

        macro_rules! push {
            ($val:expr) => {
                stack.push($val)
            };
        }

        macro_rules! unary {
            ($op:ident) => {{
                let v: Value = pop!();
                push!(v.$op()?)
            }};
        }

        macro_rules! binary {
            ($op:ident) => {{
                let r: Value = pop!();
                let l: Value = pop!();
                push!(l.$op(&r)?)
            }};
        }

        loop {
            match Opcode::from_u8(fetch_u8!()).unwrap() {
                Opcode::Push0 => push!(Value::from_int(0)),
                Opcode::Push1 => push!(Value::from_int(1)),
                Opcode::PushNull => push!(Value::NULL),
                Opcode::PushFalse => push!(Value::FALSE),
                Opcode::PushTrue => push!(Value::TRUE),
                Opcode::PushInt => push!(Value::from_int(fetch_sleb!())),
                // PushConst => "push_const";      // 06 // N
                Opcode::Add => binary!(add),
                Opcode::Sub => binary!(sub),
                Opcode::Mul => binary!(mul),
                Opcode::Div => binary!(div),
                Opcode::Mod => binary!(rem),
                Opcode::Eq => binary!(eq),
                Opcode::Ne => binary!(ne),
                Opcode::Lt => binary!(lt),
                Opcode::Le => binary!(le),
                Opcode::Gt => binary!(gt),
                Opcode::Ge => binary!(ge),
                Opcode::Neg => unary!(negate),
                Opcode::Not => unary!(not),
                Opcode::Load1 => push!(stack[fp + 1].clone()),
                Opcode::LoadLocal => push!(stack[fp + fetch_uleb!()].clone()),
                Opcode::StoreLocal => stack[fp + fetch_uleb!()] = pop!(),
                Opcode::LoadGlobal => push!(globals[fetch_uleb!()].clone()),
                Opcode::StoreGlobal => globals[fetch_uleb!()] = pop!(),
                Opcode::LoadBuiltin => push!(builtins[fetch_uleb!()].clone()),
                // MakeList => "make_list";        // 1A // N
                // GetItem => "get_item";          // 1B
                // SetItem => "set_item";          // 1C
                // Jmp => "jmp";                   // 1D // offset
                // JFalse => "jfalse";             // 1E // offset
                // JTrue => "jtrue";               // 1F // offset
                // Call => "call";                 // 20 // N
                Opcode::Ret => {
                    let ret_val = pop!();
                    // TODO pop frame and continue if not last, otherwise:
                    return Ok(ret_val);
                }
                Opcode::Pop => {
                    pop!();
                }
                op => todo!("opcode {:?}", op),
            }
        }
    }
}
