use crate::rules::callable::{Call, Callable};
use crate::rules::object::Object;
use std::sync::Arc;

pub enum Value {
    String(String),
    Int(u32),
    Bool(bool),
    Ident(String),
    Object(Arc<dyn for<'a> Object<'a>>),
    Callable(Arc<Call>),
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

pub trait FromValue: Clone {
    fn from_value(val: &Value) -> Self;
}

impl FromValue for String {
    fn from_value(val: &Value) -> Self {
        if let Value::String(s) = val {
            s.clone()
        } else {
            String::new()
        }
    }
}

pub trait FromVec {
    fn from_vec(values: &[Value]) -> Self
    where
        Self: Sized;
}

impl FromVec for () {
    fn from_vec(values: &[Value]) -> Self
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
        (A::from_value(iter.next().unwrap()),)
    }
}
