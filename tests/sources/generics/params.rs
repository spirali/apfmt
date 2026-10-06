fn k<T>(a: T) {}
fn l<'a, T: Clone + 'a, const N: usize>(a: &'a T) {}
fn m<T: ?Sized, U = u32, const N: usize = 3>(a: &T) {}
fn x<T: A<B>, F: Fn(u32) -> u32, 'a: 'b + 'c>() {}
fn q<FirstTypeParameterName: FirstTraitBound, SecondTypeParameterName: SecondTraitBound>(a: u32) {
    a;
}
// The generics stay on one line, the parameters break (Style Guide)
fn q2<FirstTypeParameterName: FirstTraitBound, SecondTypeParameterName: SecondTraitBound>(a: u32, b: u32) {
    a;
}
fn s<T: FirstVeryLongTraitBoundName + SecondVeryLongTraitBoundName + ThirdTraitBoundName + Fourth>(a: T) {
    a;
}
struct S<T>(T);
struct U<T: Clone> { a: T }
enum E<T> { A(T) }
