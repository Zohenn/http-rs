pub trait Callable<Args = ()> {
    type Result;

    fn invoke(&self, args: Args) -> Self::Result;
}

impl<F, R> Callable<()> for F
where
    F: Fn() -> R,
{
    type Result = R;

    fn invoke(&self, args: ()) -> Self::Result {
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
