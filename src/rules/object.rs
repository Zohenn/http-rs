use crate::request::Request;
use crate::response::Response;
use crate::rules::callable::{wrap_callable, Call, Function};
use crate::rules::value::{FromVec, Type, Value};
use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

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

    pub fn builder() -> ObjectBuilder {
        ObjectBuilder {
            members: HashMap::new(),
        }
    }
}

pub struct ObjectBuilder {
    members: HashMap<String, Member>,
}

impl ObjectBuilder {
    pub fn add_field<Args, F>(mut self, ident: &str, callable: F) -> Self
    where
        Args: FromVec,
        F: Function<Args, Result = Type> + 'static,
    {
        self.members
            .insert(ident.to_owned(), Member::field(wrap_callable(callable)));

        self
    }

    pub fn add_method<Args, F>(mut self, ident: &str, callable: F) -> Self
    where
        Args: FromVec,
        F: Function<Args, Result = Type> + 'static,
    {
        self.members
            .insert(ident.to_owned(), Member::method(wrap_callable(callable)));

        self
    }

    pub fn get(self, instance: Rc<RefCell<dyn Any>>) -> Object {
        Object {
            members: self.members,
            instance,
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
        Object::builder()
            .add_field("method", |instance: Rc<RefCell<dyn Any>>| {
                let instance = downcast_instance_ref::<Request>(&instance);
                Type::String(instance.method.to_string())
            })
            .get(self)
    }
}

impl IntoObject for Rc<RefCell<Response>> {
    fn into_object(self) -> Object {
        Object::builder()
            .add_method(
                "set_header",
                |instance: Rc<RefCell<dyn Any>>, name: String, value: String| {
                    let mut instance = downcast_instance_mut::<Response>(&instance);
                    instance.set_header(&name, &value);
                    Type::Bool(true)
                },
            )
            .get(self)
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
    pub callable: Rc<Call>,
}

impl Member {
    pub fn field(getter: Rc<Call>) -> Self {
        Member {
            kind: MemberKind::Field,
            callable: getter,
        }
    }

    pub fn method(callable: Rc<Call>) -> Self {
        Member {
            kind: MemberKind::Method,
            callable,
        }
    }

    pub fn eval(&self, args: Vec<Value>) -> Type {
        self.callable.as_ref()(args)
    }
}
