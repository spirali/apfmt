struct Fields {
    // leading comment
    a: u32, // trailing comment
    /* block */ b: u32,

    // after a blank line
    c: u32,
    // last
}
struct Tuple(u32 /* x */, String);
struct TupleOwnLine(
    // own line
    u32,
    String,
);
struct E {
    // only a comment
}
struct F { /* c */ }
struct G {
    // one

    // two
}
struct BeforeComma(u32 /* x */ /* y */, String /* z */);
struct BeforeCommaLong(
    first_field_type_name_aaaaaaaaaaaaaaaaaaa /* x */,
    second_field_type_name_aaaaaaaaaaaaaaaaaa,
);
struct LineCommentBeforeComma(
    first_field // x
    ,
    second_field,
);
