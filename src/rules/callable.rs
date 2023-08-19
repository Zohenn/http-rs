use crate::rules::value::{FromVec, Type, Value};
use std::rc::Rc;

pub trait Function<Args = ()> {
    type Result;

    fn invoke(&self, args: Args) -> Self::Result;
}

impl<F, R> Function<()> for F
where
    F: Fn() -> R,
{
    type Result = R;

    fn invoke(&self, _args: ()) -> Self::Result {
        self()
    }
}

impl<F, A, R> Function<(A,)> for F
where
    F: Fn(A) -> R,
{
    type Result = R;

    fn invoke(&self, args: (A,)) -> Self::Result {
        self(args.0)
    }
}

impl<F, A, B, C, R> Function<(A, B, C)> for F
where
    F: Fn(A, B, C) -> R,
{
    type Result = R;

    fn invoke(&self, args: (A, B, C)) -> Self::Result {
        self(args.0, args.1, args.2)
    }
}

pub type Call = dyn Fn(Vec<Value>) -> Type;

pub fn wrap_callable<F, Args>(func: F) -> Rc<Call>
where
    Args: FromVec,
    F: Function<Args, Result = Type> + 'static,
{
    Rc::new(move |args| func.invoke(Args::from_vec(&args)))
}
