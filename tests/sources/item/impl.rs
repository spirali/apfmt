impl S {}
impl<T> Trait for S<T> {
    const A: u32 = 1;


    type B = u32;
    fn f(&self) {}
    // trailing comment
}
pub(crate) unsafe impl<'a> !Send for X<'a> {
    // only a comment
}
impl<T: Clone> SomeVeryLongTraitNameForTesting<WithGenerics> for AnotherVeryLongTypeName<T, U, V> {}
impl<T> Trait for S<T> where T: Clone + Send {
    fn f() {}
}
default impl<T> Trait for T {}
