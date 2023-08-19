use crate::rules::callable::Call;
use crate::rules::error::{RuleError, SemanticErrorKind};
use crate::rules::lexer::Position;
use crate::rules::object::Object;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub enum Value {
    String(String),
    Int(u32),
    Bool(bool),
    Ident(String),
    Object(Object),
    Function(Rc<Call>),
    Method(Object, Rc<Call>),
    Many(Vec<Value>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(s1), Value::String(s2)) => s1.eq(s2),
            (Value::Int(i1), Value::Int(i2)) => i1.eq(i2),
            _ => todo!(),
            // Value::Int(_) => {}
            // Value::Bool(_) => {}
            // Value::Object => {}
        }
    }
}

pub trait FromValue: Sized {
    fn from_value(val: &Value) -> Result<Self, RuleError>;
}

impl FromValue for String {
    fn from_value(val: &Value) -> Result<Self, RuleError> {
        if let Value::String(s) = val {
            Ok(s.clone())
        } else {
            todo!("Error");
            Ok(String::new())
        }
    }
}

impl FromValue for Rc<RefCell<dyn Any>> {
    fn from_value(val: &Value) -> Result<Self, RuleError> {
        if let Value::Object(obj) = val {
            Ok(obj.instance.clone())
        } else {
            Err(RuleError::semantic(
                SemanticErrorKind::IncorrectType,
                Position::zero(),
            ))
        }
    }
}

pub trait FromVec {
    fn from_vec(values: &[Value]) -> Self
    where
        Self: Sized;
}

impl FromVec for () {
    fn from_vec(_values: &[Value]) -> Self
    where
        Self: Sized,
    {
    }
}

impl<A: FromValue> FromVec for (A,) {
    fn from_vec(values: &[Value]) -> Self
    where
        Self: Sized,
    {
        let mut iter = values.iter();
        (A::from_value(iter.next().unwrap()).unwrap(),)
    }
}

impl<A: FromValue, B: FromValue, C: FromValue> FromVec for (A, B, C) {
    fn from_vec(values: &[Value]) -> Self
    where
        Self: Sized,
    {
        let mut iter = values.iter();
        (
            A::from_value(iter.next().unwrap()).unwrap(),
            B::from_value(iter.next().unwrap()).unwrap(),
            C::from_value(iter.next().unwrap()).unwrap(),
        )
    }
}
