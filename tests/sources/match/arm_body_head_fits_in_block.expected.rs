// A head that fits on one line only in the block goes there. rustfmt does the same when `match
// head` is wider, but when only ` {` does not fit it puts `{` on its own line
fn head_fits_in_block() {
    match x {
        // rustfmt: `{` on its own line
        Some(Axxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx) => {
            match protobuf_plugin_command.payload {
                _ => 1,
            }
        }
        // rustfmt: `{` on its own line, after a head that could break
        Some(Axxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx) => {
            match compute(aaaaaaaaaaaaa, bbbbbbbbbbbbb)? {
                _ => 1,
            }
        }
        // `match head` does not fit either: the same in rustfmt
        Some(Axxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx) => {
            match compute(aaaaaaaaaaaaa, bbbbbbbbbbbbb)? {
                _ => 1,
            }
        }
        // It fits on the line of `=>`
        Some(Axxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx) => match compute(aaaaaaaaaaaaa, bbbbbbbbbbbbb)? {
            _ => 1,
        },
    }
}
