const A: u8 = 1; /// doc f
fn f() {}

struct S {
    a: u8, /// doc b
    b: u8,
}

enum E {
    A, /** doc B */ B,
}

impl S {
    fn g() {} /// doc h
    fn h() {}
}

fn k() {
    let x = 1; /// doc y
    let y = 2;
}

struct S2 {
    a: u8,     // a
    bbbb: u8, /// doc c
    c: u8, // c
}

struct T(u8, /// d
    u16);

trait Tr {
    type A; /// doc f
    fn f(); /** doc g */ fn g();
}

fn k2() {
    g(1, /// x
        2);
    let s = S { a: 1, /// d
        b: 2 };
    match x {
        1 => {} /// doc
        _ => {}
    }
}

mod m2 { /// doc
    fn f() {}
}

fn k3() {
    g(1, /// x
        2);
    g(1, // x
        2);
    g(1, /** x */
        2);
    let a = [1, /// x
        2];
}
