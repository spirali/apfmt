// A blank line in front of an own-line comment between items is kept, in every list (rustfmt
// removes it between call arguments and array elements); other blank lines between them are removed
fn f() {
    g(
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,

        // c
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,

        cccccccccccccccccccccccccccc,
    );
    let v = [
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,

        // c
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
    ];
}
