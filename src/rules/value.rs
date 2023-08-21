use crate::rules::callable::Call;
use crate::rules::error::{RuleError, RuntimeErrorKind, SemanticErrorKind};
use crate::rules::lexer::Position;
use crate::rules::object::Object;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub enum Type {
    String(String),
    Int(u32),
    Bool(bool),
    Ident(String),
    Object(Object),
    Function(Rc<Call>),
    Method(Object, Rc<Call>),
    List(Vec<Value>),
}

impl Type {
    pub fn type_string(&self) -> String {
        let s = match self {
            Type::String(_) => "string",
            Type::Int(_) => "int",
            Type::Bool(_) => "bool",
            Type::Ident(_) => "identifier",
            Type::Object(_) => "object",
            Type::Function(_) | Type::Method(_, _) => "callable",
            Type::List(_) => "list",
        };

        s.to_owned()
    }
}

#[derive(Clone)]
pub struct Value {
    t: Type,
    position: Position,
}

impl Value {
    pub fn new(t: Type, position: Position) -> Self {
        Value { t, position }
    }

    pub fn t(&self) -> &Type {
        &self.t
    }

    pub fn take_t(self) -> Type {
        self.t
    }

    pub fn position(&self) -> &Position {
        &self.position
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (&self.t, &other.t) {
            (Type::String(s1), Type::String(s2)) => s1.eq(s2),
            (Type::Int(i1), Type::Int(i2)) => i1.eq(i2),
            (Type::Bool(b1), Type::Bool(b2)) => b1.eq(b2),
            _ => false,
        }
    }
}

pub trait FromValue: Sized {
    fn from_value(val: &Value) -> Result<Self, RuleError>;
}

impl FromValue for String {
    fn from_value(val: &Value) -> Result<Self, RuleError> {
        if let Type::String(s) = val.t() {
            Ok(s.clone())
        } else {
            Err(RuleError::runtime(
                RuntimeErrorKind::IncorrectType("string".to_owned(), val.t().type_string()),
                *val.position(),
            ))
        }
    }
}

impl FromValue for Rc<RefCell<dyn Any>> {
    fn from_value(val: &Value) -> Result<Self, RuleError> {
        if let Type::Object(obj) = val.t() {
            Ok(obj.instance.clone())
        } else {
            Err(RuleError::runtime(
                RuntimeErrorKind::IncorrectType("object".to_owned(), val.t().type_string()),
                Position::zero(),
            ))
        }
    }
}

pub trait FromVec {
    fn from_vec(values: &[Value]) -> Result<Self, RuleError>
    where
        Self: Sized;
}

impl FromVec for () {
    fn from_vec(_values: &[Value]) -> Result<Self, RuleError>
    where
        Self: Sized,
    {
        Ok(())
    }
}

fn next_vec_value<'a>(
    iter: &'a mut dyn Iterator<Item = &Value>,
    expected_count: usize,
    got_count: usize,
) -> Result<&'a Value, RuleError> {
    iter.next().ok_or_else(|| {
        RuleError::runtime(
            RuntimeErrorKind::TooFewArguments(expected_count, got_count),
            Position::zero(),
        )
    })
}

impl<A: FromValue> FromVec for (A,) {
    fn from_vec(values: &[Value]) -> Result<Self, RuleError>
    where
        Self: Sized,
    {
        let mut iter = values.iter();
        Ok((A::from_value(next_vec_value(&mut iter, 1, 0)?)?,))
    }
}

impl<A: FromValue, B: FromValue, C: FromValue> FromVec for (A, B, C) {
    fn from_vec(values: &[Value]) -> Result<Self, RuleError>
    where
        Self: Sized,
    {
        let mut iter = values.iter();
        Ok((
            A::from_value(next_vec_value(&mut iter, 3, 0)?)?,
            B::from_value(next_vec_value(&mut iter, 3, 1)?)?,
            C::from_value(next_vec_value(&mut iter, 3, 2)?)?,
        ))
    }
}
