use crate::bc::{Bytecode, Opcode};
use crate::ctx::RuntimeContext;
use crate::error::{nx_err, NxResult};
use crate::leb128::{decode_sleb128, decode_uleb128};
use crate::value::{Builtin, Function, Value, ValueType};
use std::rc::Rc;

pub struct Interpreter<'rt> {
    rt: &'rt mut RuntimeContext,
}

impl<'rt> Interpreter<'rt> {
    pub fn new(rt: &'rt mut RuntimeContext) -> Self {
        Self { rt }
    }

    fn prepare_builtins() -> Vec<Value> {
        Builtin::ALL
            .iter()
            .map(|b| Value::from_function(Rc::new(Function::Builtin(*b))))
            .collect()
    }

    pub fn run(&mut self, bc: &Bytecode, mut args: Vec<Value>) -> NxResult<Value> {
        let builtins = Self::prepare_builtins();
        let mut globals = bc.globals.clone();
        let main = &globals[0].unwrap_function();
        let max_slots = if let Function::UserDefined { max_slots, .. } = main.as_ref() {
            max_slots
        } else {
            todo!()
        };
        let code = &bc.code;
        let mut ip = 0; // TODO code_handle from Function
        let mut stack = Vec::new();
        let arg_cnt = args.len();
        let fp = 1;
        stack.push(Value::NULL); // TODO push main function object
        stack.append(&mut args);
        assert_eq!(arg_cnt, main.param_count());
        stack.resize(stack.len() + *max_slots - arg_cnt, Value::NULL);
        // TODO do what "call arg_cnt" does - check args count, push frame, setup FP

        macro_rules! fetch_u8 {
            () => {{
                let r = code[ip];
                ip += 1;
                r
            }};
        }

        macro_rules! fetch_sleb {
            () => {
                decode_sleb128(|| fetch_u8!())
            };
        }

        macro_rules! fetch_uleb {
            () => {
                decode_uleb128(|| fetch_u8!())
            };
        }

        macro_rules! fetch_jump_target {
            () => {{
                let from = ip - 1;
                (from as i64 + decode_sleb128(|| fetch_u8!())) as usize
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

        macro_rules! pop_bool {
            () => {{
                let value = pop!();
                if value.get_type() != ValueType::Bool {
                    nx_err("expected a boolean value")
                } else {
                    Ok(value.unwrap_bool())
                }
            }};
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
                Opcode::Load0 => push!(stack[fp].clone()),
                Opcode::LoadLocal => push!(stack[fp + fetch_uleb!()].clone()),
                Opcode::StoreLocal => stack[fp + fetch_uleb!()] = pop!(),
                Opcode::LoadGlobal => push!(globals[fetch_uleb!()].clone()),
                Opcode::StoreGlobal => globals[fetch_uleb!()] = pop!(),
                Opcode::LoadBuiltin => push!(builtins[fetch_uleb!()].clone()),
                // MakeList => "make_list";        // 1A // N
                // GetItem => "get_item";          // 1B
                // SetItem => "set_item";          // 1C
                Opcode::Jmp => ip = fetch_jump_target!(),
                Opcode::JFalse => {
                    let target = fetch_jump_target!();
                    if !pop_bool!()? {
                        ip = target;
                    }
                }
                Opcode::JTrue => {
                    let target = fetch_jump_target!();
                    if pop_bool!()? {
                        ip = target;
                    }
                }
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
