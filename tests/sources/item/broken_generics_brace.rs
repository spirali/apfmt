// `{` on its own line after a broken generics clause of an impl or a trait
impl<TAllowDescriptorLongName: AllowDescriptor, TDenyDescriptorLongName: DenyDescriptor + Clone + Debug> Foo<TAllowDescriptorLongName> {
    fn f() {}
}

impl<TAllowDescriptorLongName: AllowDescriptor, TDenyDescriptorLongName: DenyDescriptor + Clone + Debug> Tr for Foo<T> {
    fn f() {}
}

impl<'a, T: 'a + AsRef<[u8]>, I: std::iter::DoubleEndedIterator<Item = &'a T> + std::iter::ExactSizeIterator<Item = &'a T> + Clone> fmt::Display for KeysInfoFormatter<'a, T, I> {
    fn fmt(&self) {}
}

trait Foo<TAllowDescriptorLongName: AllowDescriptor, TDenyDescriptorLongName: DenyDescriptor + Clone> {
    fn f();
}

trait Foo<TAllowDescriptorLongName: AllowDescriptor, TDenyDescriptorLongName: DenyDescriptor + Clone>: Bar {
    fn f();
}

// a struct keeps `> {`
struct Foo<TAllowDescriptorLongName: AllowDescriptor, TDenyDescriptorLongName: DenyDescriptor + Clone> {
    a: u8,
}
