use crate::bc::{Bytecode, Opcode};
use crate::ctx::RuntimeContext;
use crate::error::{nx_err, NxResult};
use crate::leb128::{decode_sleb128, decode_uleb128};
use crate::value::{Builtin, Function, Value, ValueType};
use std::cell::RefCell;
use std::rc::Rc;

struct CallFrame {
    ret_addr: usize,
    prev_fp: usize,
}

pub struct Interpreter<'a> {
    rt: &'a mut RuntimeContext,
    frames: Vec<CallFrame>,
}

impl<'a> Interpreter<'a> {
    pub fn new(rt: &'a mut RuntimeContext) -> Self {
        Self {
            rt,
            frames: Vec::new(),
        }
    }

    fn prepare_builtins() -> Vec<Value> {
        Builtin::ALL
            .iter()
            .map(|b| Value::from_function(Rc::new(Function::Builtin(*b))))
            .collect()
    }

    fn prepare_stack(main: Value, mut args: Vec<Value>) -> NxResult<(Vec<Value>, usize)> {
        match main.unwrap_function().as_ref() {
            Function::UserDefined {
                max_slots,
                code_handle,
                ..
            } => {
                main.unwrap_function().check_args(args.len())?;
                let mut stack = Vec::new();
                stack.push(main.clone());
                stack.append(&mut args);
                stack.resize(stack.len() + *max_slots - args.len(), Value::NULL);
                Ok((stack, *code_handle))
            }
            _ => panic!("Bytecode main_index is not a user defined function"),
        }
    }

    pub fn run(&mut self, bc: &Bytecode, args: Vec<Value>) -> NxResult<Value> {
        let builtins = Self::prepare_builtins();
        let constants = &bc.constants;
        let mut globals = bc.globals.clone();
        let main = &globals[bc.main_index];
        let (mut stack, mut ip) = Self::prepare_stack(main.clone(), args)?;
        let code = &bc.code;
        let mut fp = 1usize;

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
                Opcode::PushConst => push!(constants[fetch_uleb!()].clone()),
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
                Opcode::MakeList => {
                    let n = fetch_uleb!();
                    let v = stack[stack.len() - n..].to_vec();
                    stack.truncate(stack.len() - n);
                    push!(Value::from_list(Rc::new(RefCell::new(v))))
                }
                Opcode::GetItem => {
                    let index = pop!();
                    let array = pop!();
                    push!(array.get_item(index)?)
                }
                Opcode::SetItem => {
                    let value = pop!();
                    let index = pop!();
                    let array = pop!();
                    array.set_item(index, value)?
                }
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
                Opcode::Call => {
                    let arg_count = fetch_uleb!();
                    let new_fp = stack.len() - arg_count;
                    let fun_obj = &stack[new_fp - 1];
                    let fun_obj = if fun_obj.is_function() {
                        fun_obj.unwrap_function()
                    } else {
                        return nx_err("expected a function");
                    };
                    fun_obj.check_args(arg_count)?;
                    match fun_obj.as_ref() {
                        Function::Builtin(builtin) => {
                            let r = builtin.eval(self.rt, &stack[new_fp..new_fp + arg_count])?;
                            stack[new_fp - 1] = r;
                            stack.truncate(new_fp);
                        }
                        Function::UserDefined {
                            max_slots,
                            code_handle,
                            ..
                        } => {
                            stack.resize(stack.len() + *max_slots - arg_count, Value::NULL);
                            self.frames.push(CallFrame {
                                ret_addr: ip,
                                prev_fp: fp,
                            });
                            fp = new_fp;
                            ip = *code_handle;
                        }
                    }
                }
                Opcode::Ret => {
                    stack[fp - 1] = stack.last().unwrap().clone();
                    stack.truncate(fp);
                    match self.frames.pop() {
                        Some(frame) => {
                            ip = frame.ret_addr;
                            fp = frame.prev_fp;
                        }
                        None => {
                            return Ok(pop!());
                        }
                    }
                }
                Opcode::Pop => {
                    pop!();
                }
            }
        }
    }
}
