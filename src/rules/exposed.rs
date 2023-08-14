use crate::request::Request;
use crate::rules::expr::Value;
use crate::rules::object::{Member, Object};

impl Object for Request {
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
