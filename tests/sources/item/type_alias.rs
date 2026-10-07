type A = u32;
pub type B<T> = Vec<T>;
type VeryLongTypeAliasName<T> = SomeVeryLongGenericType<AnotherLongTypeName<T>, YetAnotherLongTypeName>;
trait Tr {
    type C: Clone + Send;
    type D<'a> where Self: 'a;
}
type E<T> = Vec<T> where T: Clone;
