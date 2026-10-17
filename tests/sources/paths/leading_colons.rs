// A path starting with `::` is spaced as a name in front of it
impl ::std::fmt::Debug for X {}
impl<T> ::std::fmt::Display for ::a::Y<T> {}
impl !::std::marker::Send for Z {}

fn f(a: &::a::B, b: Vec<::a::B>, c: <::a::B as ::c::D>::E) -> impl ::a::T + 'static {
    let s = S { a: ::a::f() };
    let t: [::a::B; 3] = [::a::B::new(); 3];
    !::a::f();
    (::a::f)();
    x as ::a::B
}
