// A return type without parameters to break moves to the next line when ` {` does not fit; rustfmt
// keeps it on the line up to 101 columns, with `{` on its own line, and moves it from 102
fn aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa() -> Result<(), E> {
    h();
}
// 99, 100 and 101 columns
fn aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa()
-> Result<(), E> {
    h();
}
fn aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa()
-> Result<(), E> {
    h();
}
fn aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa()
-> Result<(), E> {
    h();
}
// Also moved by rustfmt
fn aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa()
-> Result<(), E> {
    h();
}
// Without generic arguments
fn aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa()
-> Rrrrrrrrrr {
    h();
}
impl A {
    pub fn aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa() -> Result<(), E> {
        h();
    }
}
