use crate::rules::value::{FromVec, Value};
use std::sync::Arc;

pub trait Callable<Args = ()> {
    type Result;

    fn invoke(&self, args: Args) -> Self::Result;
}

impl<F, R> Callable<()> for F
where
    F: Fn() -> R,
{
    type Result = R;

    fn invoke(&self, _args: ()) -> Self::Result {
        self()
    }
}

impl<F, A, R> Callable<(A,)> for F
where
    F: Fn(A) -> R,
{
    type Result = R;

    fn invoke(&self, args: (A,)) -> Self::Result {
        self(args.0)
    }
}

pub type Call = dyn Fn(Vec<Value>) -> Value;
pub fn wrap_callable<F, Args>(func: F) -> Arc<Call>
where
    Args: FromVec,
    F: Callable<Args, Result = Value> + 'static,
{
    Arc::new(move |args| func.invoke(Args::from_vec(&args)))
}
