// `else {` follows the initializer only when its last line is closing delimiters at the indentation of
// `let`; otherwise it goes on its own line (Style Guide "else blocks (let-else statements)")
fn f() {
    let Some(xxxxxxxxxxxxxxxxxxxxxxxx) = some_function_name(aaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccc) else {
        return;
    };
    // Broken chain
    let Some(xxxxxxxxxxxxxxxxxxxxxxxx) = aaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb().cccccccccccccccccccccccc() else {
        return;
    };
    let Some(xxxxxxxxxxxxxxxxxxxxxxxx) = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb else {
        return;
    };
    // The pattern and the initializer on one line: break before `else`, not inside the pattern
    let Some(xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx) = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa else {
        return;
    };
    // The initializer moved to the line after `=`
    let Some(xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx) = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.b() else {
        return;
    };
    let Some(xxxxxxxxxxxxxxxxxxxxxxxx) = aaaaaaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbb(|x| { let y = x; y }) else {
        return;
    };
}
