use crate::rules::value::{FromVec, Value};
use std::sync::Arc;

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

pub trait MethodFunction<T, Args = ()> {
    type Result;

    fn invoke(&self, instance: &T, args: Args) -> Self::Result;
}

pub type Call = dyn Fn(Vec<Value>) -> Value;

pub fn wrap_callable<F, Args>(func: F) -> Arc<Call>
where
    Args: FromVec,
    F: Function<Args, Result = Value> + 'static,
{
    Arc::new(move |args| func.invoke(Args::from_vec(&args)))
}
