// A `return`, `break` or `continue` ending a broken block gets `;` (`trailing_semicolon`, as rustfmt);
// a single-line block or body keeps it without
fn f() {
    loop { g(); break }
    loop { g(); break 'a }
    let x = loop { g(); break 5 };
    for x in y { g(); continue }
    if a { return } else { b }
    if a { g(); return } else { b }
    if a { g(); return }
    let c = || { g(); return };
    let c = || { return };
    match x { A => { return } B => { g(); return } }
    unsafe { g(); return }
    'a: { g(); break 'a 1 }
    { return // c
    }
    { g(); return /* c */ }
    { #[allow(x)] return }
    { g(); m!() }
    { g(); return; }
    { g(); return;; }
    async { g(); return };
    { g(); (return) }
}
fn h() -> u8 { return 1 }
fn k() -> u8 { g(); return 1 }
