use crate::rules::callable::Callable;
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

pub trait Object<'a> {
    fn get_member(&self, ident: &str) -> Option<Member<'a>>;

    fn get_field(&self, ident: &str) -> Option<Member<'a>> {
        self.get_member_kind(ident, MemberKind::Field(Value::Bool(true)))
    }

    fn get_method(&self, ident: &str) -> Option<Member<'a>> {
        self.get_member_kind(ident, MemberKind::Method(Box::new(|| Value::Bool(true))))
    }

    fn get_member_kind(&self, ident: &str, kind: MemberKind<'a>) -> Option<Member<'a>> {
        match self.get_member(ident) {
            Some(member) if discriminant(&member.kind) == discriminant(&kind) => Some(member),
            _ => None,
        }
    }
}

pub enum MemberKind<'a> {
    Field(Value),
    Method(Box<dyn Callable<Result = Value> + 'a>),
}

pub struct Member<'a> {
    pub kind: MemberKind<'a>,
    ident: String,
}

impl<'a> Member<'a> {
    pub fn field(ident: String, value: Value) -> Self {
        Member {
            kind: MemberKind::Field(value),
            ident,
        }
    }

    pub fn method<F>(ident: String, callable: Box<F>) -> Self
    where
        F: Callable<Result = Value> + 'a,
    {
        Member {
            kind: MemberKind::Method(callable),
            ident,
        }
    }
}
