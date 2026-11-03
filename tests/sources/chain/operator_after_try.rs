// An operator stays on the last line of a broken first operand while that line ends at most at the
// next indentation: after `)` or `?`, not after an index or `.await?`
fn f() {
    let a = foo(|x| {
        let y = x + 1;
        y
    })? * 2;
    let a = foo(|x| {
        let y = x + 1;
        y
    }) * 2;
    let a = foo(|x| {
        let y = x + 1;
        y
    })?? * 2;
    let a = foo(|x| {
        let y = x + 1;
        y
    })??? * 2;
    let a = foo(|x| {
        let y = x + 1;
        y
    })[0] * 2;
    let a = foo(
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        ccc,
    )? * 2;
    let a = x.foo(|x| {
        let y = x + 1;
        y
    })? * 2;
    // Both operands broken
    let a = foo(|x| {
        let y = x + 1;
        y
    })? + bar(|x| {
        let y = x + 1;
        y
    })?;
    // More operators: only the first stays
    let a = foo(|x| {
        let y = x + 1;
        y
    })? * 2 * 3;
    let a = foo(|x| {
        let y = x + 1;
        y
    })? && aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa && bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb;
    // A single-line operand is left alone
    let a = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa? * bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb;
    let a = aa? * bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb;
    // In a head
    if foo(|x| {
        let y = x + 1;
        y
    })? == 2 {
        g();
    }
    // In an assignment and a range
    a = foo(|x| {
        let y = x + 1;
        y
    })? * 2;
    let r = foo(|x| {
        let y = x + 1;
        y
    })?..2;
    // Deeper
    if a {
        if b {
            let a = foo(|x| {
                let y = x + 1;
                y
            })? * 0x11;
        }
    }
}
