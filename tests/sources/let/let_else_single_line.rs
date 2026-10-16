// `let pat = init else { expr };` on a single line when the whole statement is at most
// `single_line_let_else_max_width` (50) wide and the `else` block is just an expression
fn f() {
    let Some(x) = y else { return };
    let Some(x) = y else {
        return
    };
    let Some(x) = y else { continue };
    let Some(x) = y else { panic!() };
    let Some(x) = y else { return Err(e) };
    let Some(x) = y else { unsafe { f() } };
    let Some(x) = y else { if a { b } else { c } };
    let Some(xxxxxxx): Option<u8> = y else { return };
    let Some(xxxxxxx) = y.f(|x| { x }) else { return };
    let Some(x) = y else { return }; // c
    // 50 and 51 wide
    let Some(xxxxxxx) = yyyyyyyyyyyyy else { fail() };
    let Some(xxxxxxx) = yyyyyyyyyyyyyy else { fail() };
    // A statement, a comment, an attribute or a macro call with braces: broken
    let Some(x) = y else { return; };
    let Some(x) = y else { panic!(); };
    let Some(x) = y else { f(); fail() };
    let Some(x) = y else {
        // c
        fail()
    };
    let Some(x) = y else { #[allow(x)] fail() };
    let Some(x) = y else { m! {} };
    let Some(x) = y else {};
}
