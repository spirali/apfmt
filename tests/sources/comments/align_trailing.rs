// Trailing comments of the items of a broken list on consecutive lines are aligned in a column
fn f() {
    let v = vec![
        a, // first
        bbbbbb, // second
        cc,       // third
    ];
    call(
        x, // x
        yyyyyy, // y
    );
    match x {
        A => 1, // one
        Bbbbbb => 2,    // two
    }
    let s = S { a: 1, // a
        bbbbbb: 2, // b
    };
    // Not after statements
    let a = 1; // a
    let bbbbbb = 2; // b
}
struct S {
    a: u32, // a
    bbbbbb: String, // b
    e: u8 /* before comma */,
    f: Vec<u8>, /* block */
}
enum E {
    A, // a
    Bbbbbb, // b
}
// A blank line, an own-line comment, an attribute or an item without a comment ends a column
struct Blank {
    a: u32, // a
    bbbbbb: String, // b

    c: u8, // c
    ddddddddddd: u8, // d
}
struct OwnLine {
    a: u32, // a
    // section
    bbbbbb: String, // b
}
struct NoComment {
    a: u32, // a
    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb: u8,
    cccccc: String, // c
}
struct Attr {
    a: u32, // a
    #[serde(default)]
    bbbbbb: String, // b
}
