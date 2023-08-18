use crate::request::Request;
use crate::response::Response;
use crate::rules::callable::wrap_callable;
use crate::rules::object::{Member, Object};
use crate::rules::value::Value;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

// impl Object for Request {
//     fn get_member(&self, ident: &str) -> Option<Member> {
//         match ident {
//             "method" => Some(Member::field(
//                 "method".to_owned(),
//                 Value::String(self.method.to_string()),
//             )),
//             _ => None,
//         }
//     }
// }
//
// impl Object for Response {
//     fn get_member(&self, ident: &str) -> Option<Member> {
//         match ident {
//             "set_header" => Some(Member::method(
//                 "set_header".to_owned(),
//                 wrap_callable(|response: Rc<RefCell<Response>>, name: String, value: String| {
//                     response.borrow_mut().set_header(&name, &value);
//                     Value::Bool(false)
//                 }),
//             )),
//             _ => None,
//         }
//     }
// }
