use crate::request::Request;
use crate::response::Response;
use crate::rules::callable::{Call, Function};
use crate::rules::error::{RuleError, SemanticErrorKind};
use crate::rules::lexer::Position;
use crate::rules::object::{Downcast, Object};
use crate::rules::object2::Object2;
use crate::rules::RuleEvaluationResult;
use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
pub enum Value {
    String(String),
    Int(u32),
    Bool(bool),
    Ident(String),
    // Object(Rc<dyn Any>),
    Object2(Object2),
    // ObjectMut(Rc<dyn Any>),
    Callable(Arc<Call>),
    CallableMethod(Object2, Arc<Call>),
    // CallableMethod(Rc<dyn Any>, Arc<Call>),
    // CallableMethodMut(Rc<dyn Any>, Arc<Call>),
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

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

// impl FromValue for Rc<RefCell<Response>> {
//     fn from_value(val: &Value) -> Result<Self, RuleError> {
//         if let Value::ObjectMut(obj) = val {
//             print_type_of(&obj);
//
//
//             // let obj = obj.as_any();
//             // let o: Rc<dyn Any> = Rc::new(obj.clone());
//             let o = obj.downcast_ref::<Rc<RefCell<Response>>>().unwrap();
//             // let o2 = obj.downcast_ref::<Rc<RefCell<Response>>>().unwrap();
//
//             println!("{:?}", o.downcast_ref::<Rc<Rc<RefCell<Response>>>>());
//             println!("{:?}", o.downcast_ref::<Rc<RefCell<Response>>>());
//             println!("{:?}", o.downcast_ref::<RefCell<Response>>());
//             println!("{:?}", o.downcast_ref::<&dyn Any>());
//             // print_type_of(&obj);
//             //
//             println!("{:?}", o.type_id());
//             println!("{:?}", obj.type_id());
//             println!("{:?}", TypeId::of::<Rc<dyn Any>>());
//             println!("{:?}", TypeId::of::<Rc<Rc<RefCell<Response>>>>());
//             println!("{:?}", TypeId::of::<Rc<RefCell<Response>>>());
//             println!("{:?}", TypeId::of::<RefCell<Response>>());
//             println!("{:?}", TypeId::of::<Rc<RefCell<dyn Object>>>());
//             // obj.downcast_ref::<RefCell<Response>>().unwrap();
//             // obj.downcast_ref::<Rc<RefCell<Response>>>().unwrap();
//             Err(RuleError::semantic(
//                 SemanticErrorKind::IncorrectType,
//                 Position::zero(),
//             ))
//
//             // let o = obj
//             //     .downcast_ref::<RefCell<Response>>()
//             //     .map_err(|_| {
//             //         RuleError::semantic(SemanticErrorKind::IncorrectType, Position::zero())
//             //     })?
//             //     .clone();
//             //
//             // Ok(o)
//         } else {
//             Err(RuleError::semantic(
//                 SemanticErrorKind::IncorrectType,
//                 Position::zero(),
//             ))
//         }
//     }
// }

impl FromValue for Rc<RefCell<Request>> {
    fn from_value(val: &Value) -> Result<Self, RuleError> {
        if let Value::Object2(obj) = val {
            // let instance = obj.instance;
            // println!("{:?}", instance.type_id());
            // Ok(instance)
            Err(RuleError::semantic(
                SemanticErrorKind::IncorrectType,
                Position::zero(),
            ))
        } else {
            Err(RuleError::semantic(
                SemanticErrorKind::IncorrectType,
                Position::zero(),
            ))
        }
    }
}

impl FromValue for Rc<RefCell<dyn Any>> {
    fn from_value(val: &Value) -> Result<Self, RuleError> {
        if let Value::Object2(obj) = val {
            Ok(obj.instance.clone())
            // Err(RuleError::semantic(
            //     SemanticErrorKind::IncorrectType,
            //     Position::zero(),
            // ))
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
