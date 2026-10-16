// An import that imports nothing is removed; with code in front of it on its line, the line stays
// and the comment below stays on its own line (rustfmt fails: "left behind trailing whitespace")
fn f() {
    x(); use a::{};
    // c
    y();
}
fn g() {
    x(); ;
    // c
    y();
}
fn h() {
    use a::{};
    y();
}
