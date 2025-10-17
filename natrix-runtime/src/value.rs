use crate::nx_err::{nx_err, nx_error, NxResult};
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use std::str::FromStr;

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

#[derive(Debug, Clone)]
pub struct Value(ValueImpl);

#[derive(Debug, Clone)]
enum ValueImpl {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Rc<str>),
    List(Rc<RefCell<Vec<Value>>>),
    Function(Rc<FunctionObject>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeHandle(pub usize);

pub const BUILTIN_FLOAT: CodeHandle = CodeHandle(usize::MAX - 1);
pub const BUILTIN_INT: CodeHandle = CodeHandle(usize::MAX - 2);
pub const BUILTIN_LEN: CodeHandle = CodeHandle(usize::MAX - 3);
pub const BUILTIN_PRINT: CodeHandle = CodeHandle(usize::MAX - 4);
pub const BUILTIN_STR: CodeHandle = CodeHandle(usize::MAX - 5);

#[derive(Debug)]
pub struct FunctionObject {
    pub name: Box<str>,
    pub arity: usize,
    pub code_handle: CodeHandle,
}

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

    pub fn from_function(v: Rc<FunctionObject>) -> Self {
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

    pub fn is_numeric(&self) -> bool {
        matches!(self.get_type(), ValueType::Int | ValueType::Float)
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

    pub fn unwrap_function(&self) -> Rc<FunctionObject> {
        match &self.0 {
            ValueImpl::Function(v) => v.clone(),
            _ => panic!("expected function, got {:?}", self.get_type()),
        }
    }

    // Internal helpers - return reference to Rc (no refcount bump)
    fn string_ref(&self) -> &Rc<str> {
        match &self.0 {
            ValueImpl::String(s) => s,
            _ => panic!("expected string, got {:?}", self.get_type()),
        }
    }

    fn list_ref(&self) -> &Rc<RefCell<Vec<Value>>> {
        match &self.0 {
            ValueImpl::List(v) => v,
            _ => panic!("expected list, got {:?}", self.get_type()),
        }
    }

    fn function_ref(&self) -> &Rc<FunctionObject> {
        match &self.0 {
            ValueImpl::Function(v) => v,
            _ => panic!("expected function, got {:?}", self.get_type()),
        }
    }

    // Helper methods for operators

    fn to_f64(&self) -> f64 {
        match self.0 {
            ValueImpl::Int(v) => v as f64,
            ValueImpl::Float(v) => v,
            _ => unreachable!("to_f64 called on non-numeric type"),
        }
    }

    fn as_i64_pair(&self, other: &Value) -> Option<(i64, i64)> {
        if self.is_int() && other.is_int() {
            Some((self.unwrap_int(), other.unwrap_int()))
        } else {
            None
        }
    }

    fn check_numeric_operands(&self, other: &Value, op: &str) -> NxResult<()> {
        if self.is_numeric() && other.is_numeric() {
            Ok(())
        } else {
            nx_err(format!(
                "operator {} cannot be applied to {:?} and {:?}",
                op,
                self.get_type(),
                other.get_type()
            ))
        }
    }

    // Arithmetic operators

    pub fn add(&self, other: &Value) -> NxResult<Value> {
        // String concatenation
        if self.is_string() && other.is_string() {
            let concatenated = format!("{}{}", self.string_ref(), other.string_ref());
            return Ok(Value::from_string(concatenated.into()));
        }

        // List concatenation
        if self.is_list() && other.is_list() {
            let v1 = self.list_ref().borrow();
            let v2 = other.list_ref().borrow();
            let mut result = Vec::with_capacity(v1.len() + v2.len());
            result.extend(v1.iter().cloned());
            result.extend(v2.iter().cloned());
            return Ok(Value::from_list(Rc::new(RefCell::new(result))));
        }

        self.check_numeric_operands(other, "+")?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_int(l.wrapping_add(r)))
        } else {
            Ok(Value::from_float(self.to_f64() + other.to_f64()))
        }
    }

    pub fn sub(&self, other: &Value) -> NxResult<Value> {
        self.check_numeric_operands(other, "-")?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_int(l.wrapping_sub(r)))
        } else {
            Ok(Value::from_float(self.to_f64() - other.to_f64()))
        }
    }

    pub fn mul(&self, other: &Value) -> NxResult<Value> {
        // String repetition
        if self.is_string() && other.is_int() {
            let s = self.string_ref();
            let cnt = other.unwrap_int();
            if cnt < 0 {
                return nx_err("string repetition count cannot be negative");
            }
            let cnt = cnt as usize;

            // Check for overflow before allocating
            let new_len = s
                .len()
                .checked_mul(cnt)
                .ok_or_else(|| nx_error("string repetition result too large"))?;

            let mut result = String::with_capacity(new_len);
            for _ in 0..cnt {
                result.push_str(s);
            }
            return Ok(Value::from_string(result.into()));
        }

        // List repetition
        if self.is_list() && other.is_int() {
            let l = self.list_ref().borrow();
            let cnt = other.unwrap_int();
            if cnt < 0 {
                return nx_err("list repetition count cannot be negative");
            }
            let cnt = cnt as usize;

            // Check for overflow before allocating
            let new_len = l
                .len()
                .checked_mul(cnt)
                .ok_or_else(|| nx_error("list repetition result too large"))?;

            let mut result = Vec::with_capacity(new_len);
            for _ in 0..cnt {
                result.extend(l.iter().cloned());
            }
            return Ok(Value::from_list(Rc::new(RefCell::new(result))));
        }

        if self.is_int() && (other.is_string() || other.is_list()) {
            return other.mul(self);
        }

        self.check_numeric_operands(other, "*")?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_int(l.wrapping_mul(r)))
        } else {
            Ok(Value::from_float(self.to_f64() * other.to_f64()))
        }
    }

    pub fn div(&self, other: &Value) -> NxResult<Value> {
        self.check_numeric_operands(other, "/")?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            if r == 0 {
                return nx_err("division by zero");
            }
            Ok(Value::from_int(l.wrapping_div(r)))
        } else {
            Ok(Value::from_float(self.to_f64() / other.to_f64()))
        }
    }

    pub fn rem(&self, other: &Value) -> NxResult<Value> {
        self.check_numeric_operands(other, "%")?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            if r == 0 {
                return nx_err("division by zero");
            }
            Ok(Value::from_int(l.wrapping_rem(r)))
        } else {
            Ok(Value::from_float(self.to_f64() % other.to_f64()))
        }
    }

    // Comparison operators

    pub fn eq(&self, other: &Value) -> NxResult<Value> {
        // Strings
        if self.is_string() && other.is_string() {
            return Ok(Value::from_bool(self.string_ref() == other.string_ref()));
        }

        // Lists - element-wise comparison
        if self.is_list() && other.is_list() {
            let v1 = self.list_ref().borrow();
            let v2 = other.list_ref().borrow();

            if v1.len() != v2.len() {
                return Ok(Value::FALSE);
            }

            for (e1, e2) in v1.iter().zip(v2.iter()) {
                if !e1.eq(e2)?.unwrap_bool() {
                    return Ok(Value::FALSE);
                }
            }
            return Ok(Value::TRUE);
        }

        // Functions
        if self.is_function() && other.is_function() {
            return Ok(Value::from_bool(Rc::ptr_eq(
                self.function_ref(),
                other.function_ref(),
            )));
        }

        // Bools
        if self.is_bool() && other.is_bool() {
            return Ok(Value::from_bool(self.unwrap_bool() == other.unwrap_bool()));
        }

        // Numbers
        if self.is_numeric() && other.is_numeric() {
            return if let Some((l, r)) = self.as_i64_pair(other) {
                Ok(Value::from_bool(l == r))
            } else {
                Ok(Value::from_bool(self.to_f64() == other.to_f64()))
            };
        }

        // Incompatible types are never equal
        Ok(Value::from_bool(false))
    }

    pub fn ne(&self, other: &Value) -> NxResult<Value> {
        self.eq(other).map(|v| Value::from_bool(!v.unwrap_bool()))
    }

    pub fn lt(&self, other: &Value) -> NxResult<Value> {
        if self.is_string() && other.is_string() {
            return Ok(Value::from_bool(self.string_ref() < other.string_ref()));
        }

        self.check_numeric_operands(other, "<")?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_bool(l < r))
        } else {
            Ok(Value::from_bool(self.to_f64() < other.to_f64()))
        }
    }

    pub fn le(&self, other: &Value) -> NxResult<Value> {
        if self.is_string() && other.is_string() {
            return Ok(Value::from_bool(self.string_ref() <= other.string_ref()));
        }

        self.check_numeric_operands(other, "<=")?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_bool(l <= r))
        } else {
            Ok(Value::from_bool(self.to_f64() <= other.to_f64()))
        }
    }

    pub fn gt(&self, other: &Value) -> NxResult<Value> {
        if self.is_string() && other.is_string() {
            return Ok(Value::from_bool(self.string_ref() > other.string_ref()));
        }

        self.check_numeric_operands(other, ">")?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_bool(l > r))
        } else {
            Ok(Value::from_bool(self.to_f64() > other.to_f64()))
        }
    }

    pub fn ge(&self, other: &Value) -> NxResult<Value> {
        if self.is_string() && other.is_string() {
            return Ok(Value::from_bool(self.string_ref() >= other.string_ref()));
        }

        self.check_numeric_operands(other, ">=")?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_bool(l >= r))
        } else {
            Ok(Value::from_bool(self.to_f64() >= other.to_f64()))
        }
    }

    // Unary operators

    pub fn negate(&self) -> NxResult<Value> {
        match self.get_type() {
            ValueType::Int => Ok(Value::from_int(self.unwrap_int().wrapping_neg())),
            ValueType::Float => Ok(Value::from_float(-self.unwrap_float())),
            t => nx_err(format!("unary negation cannot be applied to {:?}", t)),
        }
    }

    pub fn not(&self) -> NxResult<Value> {
        if self.is_bool() {
            Ok(Value::from_bool(!self.unwrap_bool()))
        } else {
            nx_err(format!(
                "logical negation cannot be applied to {:?}",
                self.get_type()
            ))
        }
    }

    // Other operations
    pub fn str(&self) -> Value {
        Value::from_string(format!("{}", self).into())
    }

    pub fn len(&self) -> NxResult<Value> {
        match &self.0 {
            ValueImpl::String(s) => Ok(Value::from_int(s.len() as i64)),
            ValueImpl::List(l) => Ok(Value::from_int(l.borrow().len() as i64)),
            _ => nx_err(format!("len cannot be applied to {:?}", self.get_type())),
        }
    }

    pub fn get_item(&self, index: Value) -> NxResult<Value> {
        if !index.is_int() {
            return nx_err("index must be an integer");
        }

        let idx = index.unwrap_int();
        if idx < 0 {
            return nx_err("index cannot be negative");
        }
        let idx = idx as usize;

        if self.is_list() {
            let list = self.list_ref().borrow();
            return match list.get(idx) {
                Some(v) => Ok(v.clone()),
                None => nx_err("list index out of bounds"),
            };
        }

        if self.is_string() {
            let string = self.string_ref();
            return match string.as_bytes().get(idx) {
                Some(&byte) => Ok(Value::from_int(byte as i64)),
                None => nx_err("string index out of bounds"),
            };
        }

        nx_err("only lists and strings support indexing")
    }

    pub fn set_item(&self, index: Value, value: Value) -> NxResult<()> {
        if !index.is_int() {
            return nx_err("index must be an integer");
        }

        let idx = index.unwrap_int();
        if idx < 0 {
            return nx_err("index cannot be negative");
        }
        let idx = idx as usize;

        if self.is_list() {
            let mut list = self.list_ref().borrow_mut();
            return match list.get_mut(idx) {
                Some(v) => {
                    *v = value;
                    Ok(())
                }
                None => nx_err("list index out of bounds"),
            };
        }

        nx_err("only lists support indexing in assignments")
    }

    pub fn int(&self) -> NxResult<Value> {
        match &self.0 {
            ValueImpl::Int(i) => Ok(Value::from_int(*i)),
            // Truncates towards zero, saturates on overflow, NaN → 0
            ValueImpl::Float(f) => Ok(Value::from_int(*f as i64)),
            ValueImpl::String(s) => Ok(Value::from_int(
                i64::from_str(s).map_err(|e| nx_error(e.to_string()))?,
            )),
            _ => nx_err(format!("int cannot be applied to {:?}", self.get_type())),
        }
    }

    pub fn float(&self) -> NxResult<Value> {
        match &self.0 {
            ValueImpl::Int(i) => Ok(Value::from_float(*i as f64)),
            ValueImpl::Float(f) => Ok(Value::from_float(*f)),
            ValueImpl::String(s) => Ok(Value::from_float(
                f64::from_str(s).map_err(|e| nx_error(e.to_string()))?,
            )),
            _ => nx_err(format!("float cannot be applied to {:?}", self.get_type())),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            ValueImpl::Null => write!(f, "null"),
            ValueImpl::Bool(v) => write!(f, "{}", v),
            ValueImpl::Int(v) => write!(f, "{}", v),
            ValueImpl::Float(v) => write!(f, "{:?}", v),
            ValueImpl::String(v) => write!(f, "{}", v),
            ValueImpl::List(v) => {
                write!(f, "[")?;
                for (i, e) in v.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    match &e.0 {
                        ValueImpl::String(s) => write!(f, "{:?}", s)?,
                        _ => write!(f, "{}", e)?,
                    }
                }
                write!(f, "]")
            }
            ValueImpl::Function(fun) => {
                write!(f, "<function {} at {:#x}>", fun.name, fun.code_handle.0)
            }
        }
    }
}
