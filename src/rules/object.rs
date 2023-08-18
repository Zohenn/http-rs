use crate::rules::callable::{wrap_callable, Call, Function};
use crate::rules::value::Value;
use std::any::Any;
use std::collections::HashMap;
use std::mem::discriminant;
use std::rc::Rc;
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

pub trait AsAny: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Gets the type name of `self`
    fn type_name(&self) -> &'static str;
}

impl<T: Any> AsAny for T {
    #[inline(always)]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline(always)]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline(always)]
    fn type_name(&self) -> &'static str {
        core::any::type_name::<T>()
    }
}

/// This is a shim around `AaAny` to avoid some boilerplate code.
/// It is a separate trait because it is also implemented
/// on runtime polymorphic traits (which are `!Sized`).
pub trait Downcast: AsAny {
    /// Returns `true` if the boxed type is the same as `T`.
    ///
    /// Forward to the method defined on the type `Any`.
    #[inline]
    fn is<T>(&self) -> bool
    where
        T: AsAny,
    {
        self.as_any().is::<T>()
    }

    /// Forward to the method defined on the type `Any`.
    #[inline]
    fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: AsAny,
    {
        self.as_any().downcast_ref()
    }

    /// Forward to the method defined on the type `Any`.
    #[inline]
    fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: AsAny,
    {
        self.as_any_mut().downcast_mut()
    }
}

impl<T: ?Sized + AsAny> Downcast for T {}

pub trait Object: AsAny {
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
