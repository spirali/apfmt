// An own-line comment after `=` and the right side are indented like code; rustfmt leaves the
// statement as written
fn f() {
    let isolate =
        // SAFETY: the pointer is valid
        unsafe { g() };
    x =
        // A comment on an assignment
        y;
    let a =
        // Not indented
        b;
    const A: u32 =
        // A comment on a const
        1;
}
