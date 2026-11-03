// A blank line in front of a comment between operands is kept, one after it removed; rustfmt removes
// both
fn f() {
    let x = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa

        // Before
        + bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
        // After

        + cccccccccccccccccccccccccccccc;
    let x = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa

        // First

        // Second
        + bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb;
    // On the first line of a chain, a single blank line stays single
    let a = foo

        // A comment
        .bar()
        .baz()
        .qux();
    foo(
        a,

        b,

        // A comment
        c,
    );
}
