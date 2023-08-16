use crate::rules::callable::{wrap_callable, Call, Callable};
use crate::rules::value::Value;
use std::collections::HashMap;
use std::mem::discriminant;
use std::sync::Arc;

// pub struct Object {
//     members: HashMap<String, Member>,
// }
//
// impl Object {
//     pub fn get_member(&self, ident: &str) -> Option<&Member> {
//         self.members.get(ident)
//     }
//
//     pub fn get_field(&self, ident: &str) -> Option<&Member> {
//         self.get_member_kind(ident, MemberKind::Field)
//     }
//
//     pub fn get_method(&self, ident: &str) -> Option<&Member> {
//         self.get_member_kind(ident, MemberKind::Method)
//     }
//
//     fn get_member_kind(&self, ident: &str, kind: MemberKind) -> Option<&Member> {
//         match self.get_member(ident) {
//             Some(member) if discriminant(&member.kind) == discriminant(&kind) => Some(member),
//             _ => None,
//         }
//     }
// }

pub trait Object<'a> {
    fn get_member(&self, ident: &str) -> Option<Member>;

    fn get_field(&self, ident: &str) -> Option<Member> {
        self.get_member_kind(ident, MemberKind::Field(Value::Bool(true)))
    }

    fn get_method(&self, ident: &str) -> Option<Member> {
        self.get_member_kind(
            ident,
            MemberKind::Method(wrap_callable(|| Value::Bool(true))),
        )
    }

    fn get_member_kind(&self, ident: &str, kind: MemberKind) -> Option<Member> {
        match self.get_member(ident) {
            Some(member) if discriminant(&member.kind) == discriminant(&kind) => Some(member),
            _ => None,
        }
    }
}

pub enum MemberKind {
    Field(Value),
    Method(Arc<Call>),
}

pub struct Member {
    pub kind: MemberKind,
    ident: String,
}

impl Member {
    pub fn field(ident: String, value: Value) -> Self {
        Member {
            kind: MemberKind::Field(value),
            ident,
        }
    }

    pub fn method(ident: String, callable: Arc<Call>) -> Self {
        Member {
            kind: MemberKind::Method(callable),
            ident,
        }
    }
}
