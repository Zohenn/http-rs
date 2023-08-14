use crate::rules::expr::Value;
use std::collections::HashMap;
use std::mem::discriminant;

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

pub trait Object {
    fn get_member(&self, ident: &str) -> Option<Member>;

    fn get_field(&self, ident: &str) -> Option<Member> {
        self.get_member_kind(ident, MemberKind::Field(Value::Bool(true)))
    }

    fn get_method(&self, ident: &str) -> Option<Member> {
        self.get_member_kind(ident, MemberKind::Method)
    }

    fn get_member_kind(&self, ident: &str, kind: MemberKind) -> Option<Member> {
        match self.get_member(ident) {
            Some(member) if discriminant(&member.kind) == discriminant(&kind) => Some(member),
            _ => None,
        }
    }
}

pub enum MemberKind<'a> {
    Field(Value<'a>),
    Method,
}

pub struct Member<'a> {
    pub kind: MemberKind<'a>,
    ident: String,
}

impl<'a> Member<'a> {
    pub fn field(ident: String, value: Value<'a>) -> Self {
        Member {
            kind: MemberKind::Field(value),
            ident,
        }
    }
}
