trait T { type A: Clone; const B: u32; fn f(&self); fn g() {} }
pub unsafe trait U<T>: Clone + Send where T: Copy {}
pub trait VeryLongTraitNameForTesting<T>: SomeSuperTraitWithLongName + AnotherSuperTrait + Send + Sync {
    fn f();
}
pub trait Short: A + B {}
auto trait Marker {}
