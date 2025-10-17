use crate::nx_err::{nx_err, nx_error, NxResult};
use crate::value::{CodeHandle, Value, ValueType};
use std::fmt::Write;
use std::str::FromStr;

macro_rules! define_builtins {
    ($($variant:ident => $name:literal, $arity:expr);* $(;)?) => {
        #[repr(u8)]
        #[derive(Copy, Clone, Debug)]
        pub enum Builtin {
            $($variant),*
        }

        impl Builtin {
            pub const ALL: &'static [Builtin] = &[
                $(Builtin::$variant),*
            ];

            pub const fn name(self) -> &'static str {
                match self {
                    $(Builtin::$variant => $name),*
                }
            }

            pub const fn arity(self) -> usize {
                match self {
                    $(Builtin::$variant => $arity),*
                }
            }

            pub fn as_code_handle(&self) -> CodeHandle {
                CodeHandle(usize::MAX - *self as u8 as usize)
            }

            pub fn from_code_handle(code_handle: CodeHandle) -> Option<Self> {
                let id = usize::MAX - code_handle.0;
                if id < Self::ALL.len() {
                    // SAFETY: we just checked that the value is a valid discriminant
                    Some(unsafe { std::mem::transmute(id as u8) })
                } else {
                    None
                }
            }
        }
    };
}

define_builtins! {
    Float => "float", 1;
    Int => "int", 1;
    Len => "len", 1;
    Print => "print", 1;
    Str => "str", 1;
}

pub struct Runtime {
    output: Option<String>,
}

impl Runtime {
    pub fn new() -> Self {
        Self { output: None }
    }

    pub fn with_capture() -> Self {
        Self {
            output: Some(String::new()),
        }
    }

    pub fn take_output(self) -> String {
        self.output
            .expect("Runtime was not configured to capture output")
    }

    pub fn dispatch_builtin(&mut self, builtin: Builtin, args: &[Value]) -> NxResult<Value> {
        debug_assert!(args.len() == 1);
        match builtin {
            Builtin::Float => self.float(&args[0]),
            Builtin::Int => self.int(&args[0]),
            Builtin::Len => self.len(&args[0]),
            Builtin::Print => self.print(&args[0]),
            Builtin::Str => self.str(&args[0]),
        }
    }

    fn float(&self, arg: &Value) -> NxResult<Value> {
        match arg.get_type() {
            ValueType::Int => Ok(Value::from_float(arg.unwrap_int() as f64)),
            ValueType::Float => Ok(arg.clone()),
            ValueType::String => Ok(Value::from_float(
                f64::from_str(&arg.unwrap_string()).map_err(|e| nx_error(e.to_string()))?,
            )),
            t => nx_err(format!("float cannot be applied to {:?}", t)),
        }
    }

    fn int(&self, arg: &Value) -> NxResult<Value> {
        match arg.get_type() {
            ValueType::Int => Ok(arg.clone()),
            // Truncates towards zero, saturates on overflow, NaN → 0
            ValueType::Float => Ok(Value::from_int(arg.unwrap_float() as i64)),
            ValueType::String => Ok(Value::from_int(
                i64::from_str(&arg.unwrap_string()).map_err(|e| nx_error(e.to_string()))?,
            )),
            t => nx_err(format!("int cannot be applied to {:?}", t)),
        }
    }

    fn len(&self, arg: &Value) -> NxResult<Value> {
        match arg.get_type() {
            ValueType::String => Ok(Value::from_int(arg.unwrap_string().len() as i64)),
            ValueType::List => Ok(Value::from_int(arg.unwrap_list().borrow().len() as i64)),
            t => nx_err(format!("len cannot be applied to {:?}", t)),
        }
    }

    fn print(&mut self, value: &Value) -> NxResult<Value> {
        match &mut self.output {
            Some(output) => {
                write!(output, "{}\n", value).unwrap();
            }
            None => println!("{}", value),
        }
        Ok(Value::NULL)
    }

    fn str(&self, arg: &Value) -> NxResult<Value> {
        Ok(Value::from_string(format!("{}", arg).into()))
    }
}
