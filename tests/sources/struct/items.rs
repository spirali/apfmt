struct A;
fn f(a: A) -> B {
    a
}

pub(crate) struct B { a: A }


struct C(pub(super) A, pub(in crate::x) B);
