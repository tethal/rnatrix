mod builtin;
mod ops;

use crate::error::{nx_err, NxResult};
pub use builtin::Builtin;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Null,
    Bool,
    Int,
    Float,
    String,
    List,
    Function,
}

#[derive(Debug)]
pub enum Function {
    Builtin(Builtin),
    UserDefined {
        name: Box<str>,
        param_count: usize,
        max_slots: usize, // includes parameters
        code_handle: usize,
    },
}

impl Function {
    pub fn name(&self) -> &str {
        match self {
            Function::Builtin(builtin) => builtin.name(),
            Function::UserDefined { name, .. } => &*name,
        }
    }

    pub fn param_count(&self) -> usize {
        match self {
            Function::Builtin(builtin) => builtin.param_count(),
            Function::UserDefined { param_count, .. } => *param_count,
        }
    }

    pub fn check_args(&self, args_count: usize) -> NxResult<()> {
        let param_count = self.param_count();
        if args_count != param_count {
            nx_err(format!(
                "function {} expects {} argument{}, but {} were provided",
                self.name(),
                param_count,
                if param_count == 1 { "" } else { "s" },
                args_count
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum ValueImpl {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Rc<str>),
    List(Rc<RefCell<Vec<Value>>>),
    Function(Rc<Function>),
}

#[derive(Debug, Clone)]
pub struct Value(pub(super) ValueImpl);

impl Value {
    pub const NULL: Value = Value(ValueImpl::Null);
    pub const TRUE: Value = Value(ValueImpl::Bool(true));
    pub const FALSE: Value = Value(ValueImpl::Bool(false));

    pub fn from_bool(v: bool) -> Self {
        Value(ValueImpl::Bool(v))
    }

    pub fn from_int(v: i64) -> Self {
        Value(ValueImpl::Int(v))
    }

    pub fn from_float(v: f64) -> Self {
        Value(ValueImpl::Float(v))
    }

    pub fn from_string(v: Rc<str>) -> Self {
        Value(ValueImpl::String(v))
    }

    pub fn from_list(v: Rc<RefCell<Vec<Value>>>) -> Self {
        Value(ValueImpl::List(v))
    }

    pub fn from_function(v: Rc<Function>) -> Self {
        Value(ValueImpl::Function(v))
    }

    pub fn get_type(&self) -> ValueType {
        match self.0 {
            ValueImpl::Null => ValueType::Null,
            ValueImpl::Bool(_) => ValueType::Bool,
            ValueImpl::Int(_) => ValueType::Int,
            ValueImpl::Float(_) => ValueType::Float,
            ValueImpl::String(_) => ValueType::String,
            ValueImpl::List(_) => ValueType::List,
            ValueImpl::Function(_) => ValueType::Function,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self.0, ValueImpl::Null)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self.0, ValueImpl::Bool(_))
    }

    pub fn is_int(&self) -> bool {
        matches!(self.0, ValueImpl::Int(_))
    }

    pub fn is_float(&self) -> bool {
        matches!(self.0, ValueImpl::Float(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self.0, ValueImpl::String(_))
    }

    pub fn is_list(&self) -> bool {
        matches!(self.0, ValueImpl::List(_))
    }

    pub fn is_function(&self) -> bool {
        matches!(self.0, ValueImpl::Function(_))
    }

    pub fn unwrap_bool(&self) -> bool {
        match self.0 {
            ValueImpl::Bool(v) => v,
            _ => panic!("expected bool, got {:?}", self.get_type()),
        }
    }

    pub fn unwrap_int(&self) -> i64 {
        match self.0 {
            ValueImpl::Int(v) => v,
            _ => panic!("expected int, got {:?}", self.get_type()),
        }
    }

    pub fn unwrap_float(&self) -> f64 {
        match self.0 {
            ValueImpl::Float(v) => v,
            _ => panic!("expected float, got {:?}", self.get_type()),
        }
    }

    pub fn unwrap_string(&self) -> Rc<str> {
        match &self.0 {
            ValueImpl::String(v) => v.clone(),
            _ => panic!("expected string, got {:?}", self.get_type()),
        }
    }

    pub fn unwrap_list(&self) -> Rc<RefCell<Vec<Value>>> {
        match &self.0 {
            ValueImpl::List(v) => v.clone(),
            _ => panic!("expected list, got {:?}", self.get_type()),
        }
    }

    pub fn unwrap_function(&self) -> Rc<Function> {
        match &self.0 {
            ValueImpl::Function(v) => v.clone(),
            _ => panic!("expected function, got {:?}", self.get_type()),
        }
    }
}
