enum E {
    // leading
    A, // trailing
    /* block */ B,

    // after a blank line
    C(u32 /* x */, String),
    D { x: u32 /* x */, y: u32 },
    F {
        // own line
        x: u32,
    },
    // last
}
enum OnlyComment {
    // nothing here
}
