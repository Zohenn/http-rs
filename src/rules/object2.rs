use crate::request::Request;
use crate::response::Response;
use crate::rules::callable::{wrap_callable, Call};
use crate::rules::value::Value;
use std::any::Any;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::mem::discriminant;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
pub struct Object2 {
    members: HashMap<String, Member2>,
    pub instance: Rc<RefCell<dyn Any>>,
}

impl Object2 {
    pub fn get_member(&self, ident: &str) -> Option<&Member2> {
        self.members.get(ident)
    }

    pub fn get_field(&self, ident: &str) -> Option<&Member2> {
        match self.get_member(ident) {
            Some(member) if matches!(member.kind, MemberKind2::Field) => Some(member),
            _ => None,
        }
    }

    pub fn get_method(&self, ident: &str) -> Option<&Member2> {
        match self.get_member(ident) {
            Some(member) if matches!(member.kind, MemberKind2::Method) => Some(member),
            _ => None,
        }
    }
}

pub trait IntoObject {
    fn into_object(self) -> Object2;
}

impl IntoObject for Rc<RefCell<Request>> {
    fn into_object(self) -> Object2 {
        Object2 {
            members: HashMap::from([(
                "method".to_owned(),
                Member2::field(wrap_callable(|instance: Rc<RefCell<dyn Any>>| {
                    let instance =
                        Ref::map(instance.borrow(), |v| v.downcast_ref::<Request>().unwrap());
                    Value::String(instance.method.to_string())
                })),
            )]),
            instance: self,
        }
    }
}

impl IntoObject for Rc<RefCell<Response>> {
    fn into_object(self) -> Object2 {
        Object2 {
            members: HashMap::from([(
                "set_header".to_owned(),
                Member2::method(wrap_callable(
                    |instance: Rc<RefCell<dyn Any>>, name: String, value: String| {
                        let mut instance = RefMut::map(instance.borrow_mut(), |v| {
                            v.downcast_mut::<Response>().unwrap()
                        });
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
pub enum MemberKind2 {
    Field,
    Method,
}

#[derive(Clone)]
pub struct Member2 {
    pub kind: MemberKind2,
    pub callable: Arc<Call>,
}

impl Member2 {
    pub fn field(getter: Arc<Call>) -> Self {
        Member2 {
            kind: MemberKind2::Field,
            callable: getter,
        }
    }

    pub fn method(callable: Arc<Call>) -> Self {
        Member2 {
            kind: MemberKind2::Method,
            callable,
        }
    }

    pub fn eval(&self, args: Vec<Value>) -> Value {
        self.callable.as_ref()(args)
    }
}

#[cfg(test)]
mod test {
    use crate::header::Headers;
    use crate::http_version::HttpVersion;
    use crate::request::Request;
    use crate::request_method::RequestMethod;
    use crate::response::Response;
    use crate::response_status_code::ResponseStatusCode;
    use crate::rules::callable::wrap_callable;
    use crate::rules::object::Downcast;
    use crate::rules::object2::{Member2, Object2};
    use crate::rules::value::Value;
    use std::any::{Any, TypeId};
    use std::cell::{Ref, RefCell, RefMut};
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    fn test() {
        let request = Request {
            method: RequestMethod::Get,
            url: "/".to_string(),
            version: HttpVersion::Http1_1,
            headers: Headers::new(),
            body: vec![],
        };

        let request = Rc::new(RefCell::new(request));

        let obj = Object2 {
            members: HashMap::from([(
                "method".to_owned(),
                Member2::field(wrap_callable(|instance: Rc<RefCell<dyn Any>>| {
                    let instance =
                        Ref::map(instance.borrow(), |v| v.downcast_ref::<Request>().unwrap());
                    Value::String(instance.method.to_string())
                })),
            )]),
            instance: request,
        };

        let method = obj.get_field("method").unwrap();
        let callable = method.callable.clone();

        match callable(vec![Value::Object2(obj)]) {
            Value::String(s) => println!("{s}"),
            _ => {}
        }

        let response = Response::builder().get();

        let response = Rc::new(RefCell::new(response));

        let obj2 = Object2 {
            members: HashMap::from([(
                "set_header".to_owned(),
                Member2::method(wrap_callable(
                    |instance: Rc<RefCell<dyn Any>>, name: String, value: String| {
                        let mut instance = RefMut::map(instance.borrow_mut(), |v| {
                            v.downcast_mut::<Response>().unwrap()
                        });
                        instance.set_header(&name, &value);
                        Value::Bool(true)
                    },
                )),
            )]),
            instance: response.clone(),
        };

        let method = obj2.get_method("set_header").unwrap();
        let callable = method.callable.clone();

        callable(vec![
            Value::Object2(obj2),
            Value::String("server".to_owned()),
            Value::String("http-rs".to_owned()),
        ]);

        println!("{:?}", response.borrow());

        // println!("{:?}", TypeId::of::<Rc<RefCell<dyn Any>>>());
        // println!("{:?}", obj.instance.type_id());
        // println!("{:?}", TypeId::of::<Request>());
        // println!("{:?}", (*obj.instance.borrow()).type_id());
    }
}
