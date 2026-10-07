impl<T: Clone> SomeVeryLongTraitNameForTesting<WithGenerics> for AnotherVeryLongTypeName<T, U, V>
where
    T: Copy,
{}
impl<T: Clone> SomeVeryLongTraitNameForTesting<WithGenerics> for AnotherVeryLongTypeName<T, U, V>
where
    T: Copy,
{
    fn f() {}
}
