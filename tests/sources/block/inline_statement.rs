// A block other than `unsafe { ... }` is never on one line in statement position (Style Guide
// "Blocks"); rustfmt puts `async { 1 };` on one line
fn f() {
    { f() }
    { g() };
    async { 1 };
    unsafe { f() }
    h();
}
