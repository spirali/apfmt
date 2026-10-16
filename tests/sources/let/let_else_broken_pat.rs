// After a broken pattern, `else {` goes on its own line unless the initializer ends with closing
// delimiters at the indentation of `let` (Style Guide "else blocks (let-else statements)")
fn f() {
    let Foo { aaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccc, dddd } = x else { return; };
    let Foo { aaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccc, dddd } = f(x) else { return; };
    let Foo { aaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccc, dddd } = x? else { return; };
    let Some(Foo { aaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccc, dd }) = x else { return; };
    let Foo { aaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccc, dddd }: T = x else { return; };
    let Foo { aaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccc, dddd } = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb() else { return; };
    // The initializer ends with `)` at the indentation of `let`: `) else {` (rustfmt breaks before `else`)
    let Foo { aaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccccc, dddd } = some_function(aaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccccc) else { return; };
}
