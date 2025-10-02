#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Null,
    Bool,
    Int,
    Float,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value(ValueImpl);

#[derive(Debug, Clone, PartialEq)]
enum ValueImpl {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
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

    pub fn get_type(&self) -> ValueType {
        match self.0 {
            ValueImpl::Null => ValueType::Null,
            ValueImpl::Bool(_) => ValueType::Bool,
            ValueImpl::Int(_) => ValueType::Int,
            ValueImpl::Float(_) => ValueType::Float,
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

    pub fn as_bool(&self) -> Option<bool> {
        match self.0 {
            ValueImpl::Bool(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self.0 {
            ValueImpl::Int(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self.0 {
            ValueImpl::Float(v) => Some(v),
            _ => None,
        }
    }
}
