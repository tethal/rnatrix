use std::fmt::Display;

use crate::error::{err_at, NxResult};
use crate::src::Span;

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

    fn check_numeric_operands(&self, other: &Value, op: &str, op_span: Span) -> NxResult<()> {
        let l_numeric = matches!(self.get_type(), ValueType::Int | ValueType::Float);
        let r_numeric = matches!(other.get_type(), ValueType::Int | ValueType::Float);

        if l_numeric && r_numeric {
            Ok(())
        } else {
            err_at(
                op_span,
                format!(
                    "operator {} cannot be applied to {:?} and {:?}",
                    op,
                    self.get_type(),
                    other.get_type()
                ),
            )
        }
    }

    // Arithmetic operators

    pub fn add(&self, other: &Value, op_span: Span) -> NxResult<Value> {
        self.check_numeric_operands(other, "+", op_span)?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_int(l.wrapping_add(r)))
        } else {
            Ok(Value::from_float(self.to_f64() + other.to_f64()))
        }
    }

    pub fn sub(&self, other: &Value, op_span: Span) -> NxResult<Value> {
        self.check_numeric_operands(other, "-", op_span)?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_int(l.wrapping_sub(r)))
        } else {
            Ok(Value::from_float(self.to_f64() - other.to_f64()))
        }
    }

    pub fn mul(&self, other: &Value, op_span: Span) -> NxResult<Value> {
        self.check_numeric_operands(other, "*", op_span)?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_int(l.wrapping_mul(r)))
        } else {
            Ok(Value::from_float(self.to_f64() * other.to_f64()))
        }
    }

    pub fn div(&self, other: &Value, op_span: Span) -> NxResult<Value> {
        self.check_numeric_operands(other, "/", op_span)?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            if r == 0 {
                return err_at(op_span, "division by zero");
            }
            Ok(Value::from_int(l.wrapping_div(r)))
        } else {
            Ok(Value::from_float(self.to_f64() / other.to_f64()))
        }
    }

    pub fn rem(&self, other: &Value, op_span: Span) -> NxResult<Value> {
        self.check_numeric_operands(other, "%", op_span)?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            if r == 0 {
                return err_at(op_span, "division by zero");
            }
            Ok(Value::from_int(l.wrapping_rem(r)))
        } else {
            Ok(Value::from_float(self.to_f64() % other.to_f64()))
        }
    }

    // Comparison operators

    pub fn eq(&self, other: &Value, op_span: Span) -> NxResult<Value> {
        // Special case: bool equality
        if self.is_bool() && other.is_bool() {
            return Ok(Value::from_bool(self.unwrap_bool() == other.unwrap_bool()));
        }

        self.check_numeric_operands(other, "==", op_span)?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_bool(l == r))
        } else {
            Ok(Value::from_bool(self.to_f64() == other.to_f64()))
        }
    }

    pub fn ne(&self, other: &Value, op_span: Span) -> NxResult<Value> {
        // Special case: bool inequality
        if self.is_bool() && other.is_bool() {
            return Ok(Value::from_bool(self.unwrap_bool() != other.unwrap_bool()));
        }

        self.check_numeric_operands(other, "!=", op_span)?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_bool(l != r))
        } else {
            Ok(Value::from_bool(self.to_f64() != other.to_f64()))
        }
    }

    pub fn lt(&self, other: &Value, op_span: Span) -> NxResult<Value> {
        self.check_numeric_operands(other, "<", op_span)?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_bool(l < r))
        } else {
            Ok(Value::from_bool(self.to_f64() < other.to_f64()))
        }
    }

    pub fn le(&self, other: &Value, op_span: Span) -> NxResult<Value> {
        self.check_numeric_operands(other, "<=", op_span)?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_bool(l <= r))
        } else {
            Ok(Value::from_bool(self.to_f64() <= other.to_f64()))
        }
    }

    pub fn gt(&self, other: &Value, op_span: Span) -> NxResult<Value> {
        self.check_numeric_operands(other, ">", op_span)?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_bool(l > r))
        } else {
            Ok(Value::from_bool(self.to_f64() > other.to_f64()))
        }
    }

    pub fn ge(&self, other: &Value, op_span: Span) -> NxResult<Value> {
        self.check_numeric_operands(other, ">=", op_span)?;

        if let Some((l, r)) = self.as_i64_pair(other) {
            Ok(Value::from_bool(l >= r))
        } else {
            Ok(Value::from_bool(self.to_f64() >= other.to_f64()))
        }
    }

    // Unary operators

    pub fn negate(&self, op_span: Span) -> NxResult<Value> {
        match self.get_type() {
            ValueType::Int => Ok(Value::from_int(self.unwrap_int().wrapping_neg())),
            ValueType::Float => Ok(Value::from_float(-self.unwrap_float())),
            t => err_at(
                op_span,
                format!("unary negation cannot be applied to {:?}", t),
            ),
        }
    }

    pub fn not(&self, op_span: Span) -> NxResult<Value> {
        if self.is_bool() {
            Ok(Value::from_bool(!self.unwrap_bool()))
        } else {
            err_at(
                op_span,
                format!(
                    "logical negation cannot be applied to {:?}",
                    self.get_type()
                ),
            )
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            ValueImpl::Null => write!(f, "null"),
            ValueImpl::Bool(v) => write!(f, "{}", v),
            ValueImpl::Int(v) => write!(f, "{}", v),
            ValueImpl::Float(v) => write!(f, "{}", v),
        }
    }
}
