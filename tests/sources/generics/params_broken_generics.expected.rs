// Broken generics are followed by broken parameters
fn generic_helper<
    T: Clone + Debug,
    U: From<T> + Into<Value>,
    F: Fn(T) -> U,
    G: FnMut(U) -> Option<T>,
>(
    first: T,
    second: F,
) -> Option<(T, U)>
where
    T: Send,
{
    x
}
fn no_params<T: Clone + Debug, U: From<T> + Into<Value>, F: Fn(T) -> U, G: FnMut(U) -> Option<T>>()
-> Option<(T, U)> {
    x
}
// The generics fit on one line, so the parameters break instead (Style Guide; rustfmt breaks both)
fn with_self<T: Clone + Debug, U: From<T> + Into<Value>, F: Fn(T) -> U, G: FnMut(U) -> Option<T>>(
    &self,
) {
    x
}
fn short<T>(a: T) {}
