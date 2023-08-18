use crate::request::Request;
use crate::response::Response;
use crate::rules::callable::{wrap_callable, Call};
use crate::rules::value::Value;
use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
pub struct Object {
    members: HashMap<String, Member>,
    pub instance: Rc<RefCell<dyn Any>>,
}

impl Object {
    pub fn get_member(&self, ident: &str) -> Option<&Member> {
        self.members.get(ident)
    }

    pub fn get_field(&self, ident: &str) -> Option<&Member> {
        match self.get_member(ident) {
            Some(member) if matches!(member.kind, MemberKind::Field) => Some(member),
            _ => None,
        }
    }

    pub fn get_method(&self, ident: &str) -> Option<&Member> {
        match self.get_member(ident) {
            Some(member) if matches!(member.kind, MemberKind::Method) => Some(member),
            _ => None,
        }
    }
}

pub trait IntoObject {
    fn into_object(self) -> Object;
}

fn downcast_instance_ref<T: 'static>(instance: &Rc<RefCell<dyn Any>>) -> Ref<T> {
    Ref::map(instance.borrow(), |v| v.downcast_ref::<T>().unwrap())
}

fn downcast_instance_mut<T: 'static>(instance: &Rc<RefCell<dyn Any>>) -> RefMut<T> {
    RefMut::map(instance.borrow_mut(), |v| v.downcast_mut::<T>().unwrap())
}

impl IntoObject for Rc<RefCell<Request>> {
    fn into_object(self) -> Object {
        Object {
            members: HashMap::from([(
                "method".to_owned(),
                Member::field(wrap_callable(|instance: Rc<RefCell<dyn Any>>| {
                    let instance = downcast_instance_ref::<Request>(&instance);
                    Value::String(instance.method.to_string())
                })),
            )]),
            instance: self,
        }
    }
}

impl IntoObject for Rc<RefCell<Response>> {
    fn into_object(self) -> Object {
        Object {
            members: HashMap::from([(
                "set_header".to_owned(),
                Member::method(wrap_callable(
                    |instance: Rc<RefCell<dyn Any>>, name: String, value: String| {
                        let mut instance = downcast_instance_mut::<Response>(&instance);
                        instance.set_header(&name, &value);
                        Value::Bool(true)
                    },
                )),
            )]),
            instance: self,
        }
    }
}

#[derive(Clone)]
pub enum MemberKind {
    Field,
    Method,
}

#[derive(Clone)]
pub struct Member {
    pub kind: MemberKind,
    pub callable: Arc<Call>,
}

impl Member {
    pub fn field(getter: Arc<Call>) -> Self {
        Member {
            kind: MemberKind::Field,
            callable: getter,
        }
    }

    pub fn method(callable: Arc<Call>) -> Self {
        Member {
            kind: MemberKind::Method,
            callable,
        }
    }

    pub fn eval(&self, args: Vec<Value>) -> Value {
        self.callable.as_ref()(args)
    }
}
