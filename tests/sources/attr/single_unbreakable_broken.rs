// A single argument that does not fit within `max_width` is broken, with a trailing comma (rustfmt
// writes none); more arguments are limited by `attr_fn_like_width`
enum E {
    #[error("Generic must have one and only bound (either <T> and 'where T: Trait', or <T: Trait>)")]
    A(Span),
    #[error("could not find command '{command}' for terminal {terminal_id} at all", foo)]
    B,
    // A string spanning lines on its own line stays there, as a call argument
    #[error(
        "Module evaluation is still pending after multiple event loop iterations, \
     but no stalled top-level await was found. This is a bug in Deno."
    )]
    C,
}
