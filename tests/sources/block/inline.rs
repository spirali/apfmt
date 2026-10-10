// A block holding just an expression is on one line in expression position (Style Guide "Blocks")
fn a() { unsafe { g() } }
fn b() {
    let y = { 2 };
    let x = unsafe { f() };
    let z = async { 1 };
    let w = async move { 1 };
    let c = const { 1 };
    let l = 'a: { 1 };
    x = unsafe { f() };
    f(unsafe { g() }, 1);
    let s = S { a: unsafe { f() }, b: { 2 } };
    let t = [unsafe { f() }, { 2 }];
    let v = match x { A => unsafe { f() }, B => { g() } };
    let u = a + unsafe { b };
    if unsafe { f() } { 1 } else { 2 };
    // An `unsafe` block may be on one line in statement position too
    unsafe { f() };
    // Not on one line: statements, comments
    let y = { f(); 2 };
    let y = {
        // c
        2
    };
    // Too long: on the next line if it fits there
    let x = unsafe { some_very_long_function_name(aaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbb, ccccc) };
    let x = unsafe { some_very_long_function_name(aaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccc) };
    // The tail expression is in expression position
    { 1 }
}
