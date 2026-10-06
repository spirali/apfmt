fn n<T>(a: T) where T: Clone {}
fn o<T, U>(a: T, b: U) -> T where T: Clone + Copy, U: Default, 'a: 'b {
    a
}
fn r<T>(a: T) where T: FirstVeryLongTraitBoundName + SecondVeryLongTraitBoundName + ThirdTraitBoundName + Fourth {
    a;
}
fn t<T>(first_parameter: u32, second_parameter: u32, third_parameter: u32, fourth: T) -> u32 where T: Clone {
    first_parameter
}
fn p<T>() where T: Clone;
struct S<T>(T) where T: Clone;
struct U<T> where T: Clone;
struct W<T> where T: Clone { a: T }
enum E<T> where T: Clone { A(T) }
