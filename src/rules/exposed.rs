use crate::request::Request;
use crate::rules::object::{Member, Object};
use crate::rules::value::Value;

impl<'a> Object<'a> for Request {
    fn get_member(&self, ident: &str) -> Option<Member> {
        match ident {
            "method" => Some(Member::field(
                "method".to_owned(),
                Value::String(self.method.to_string()),
            )),
            _ => None,
        }
    }
}

// impl Object for Response {
//     fn get_member(&self, ident: &str) -> Option<Member> {
//         match ident {
//             "set_header" => Some(Member::method(
//                 "set_header".to_owned(),
//                 vec![Value::String("".to_owned()), Value::String("".to_owned())],
//             )),
//             _ => None,
//         }
//     }
// }
//
