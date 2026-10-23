// A struct variant with an attribute or a doc comment follows the other struct variants
enum WithAttr {
    #[error("x")]
    A {
        x: u32,
    },
    B {
        msg: String,
        help: String,
        aaaaaaaaaaaaaaaaaaaaaaaaaaa: u32,
        bbbbbbbbbbbbbbbbbbbbbbbb: u32,
    },
}

enum WithDoc {
    /// doc
    A {
        x: u32,
    },
    B {
        msg: String,
        help: String,
        aaaaaaaaaaaaaaaaaaaaaaaaaaa: u32,
        bbbbbbbbbbbbbbbbbbbbbbbb: u32,
    },
}
