// A comment ending the line of the operand or the operator ends the operand, so the operator gets
// its own line
fn f() {
    let a = foo(|x| {
        let y = x + 1;
        y
    })? // c
        * 2;
    let a = foo(|x| {
        let y = x + 1;
        y
    }) // c
        * 2;
    let a = foo(|x| {
        let y = x + 1;
        y
    })? // c
        * 2;
    let a = foo(|x| {
        let y = x + 1;
        y
    })?
        // c
        * 2;
}
