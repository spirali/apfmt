fn generics_only<
    TTTTTTTTTTTTTTTTTTTTTTTTTT: Clone,
    UUUUUUUUUUUUUUUUUUUUUUUUUUUUUUU: Default,
    VVVVVVVVVVV,
>() where
    T: Clone,
{
    body();
}
fn comment<T>(
    a: T, // c
) where
    T: Clone,
{
    body();
}
fn comment_after<T>(a: T) // after
where
    T: Clone,
{
    body();
}
