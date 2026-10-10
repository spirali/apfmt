// The pattern and the initializer exactly `max_width` wide still fit on one line, so `else` goes on
// its own line after them; rustfmt reserves a column for `;` and moves the initializer
fn f() {
    let Some(xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx) = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
    else {
        return;
    };
    // The last line of the initializer is not just closing delimiters: `else` on its own line
    let Some(xxxxxxxxxxxxxxxxxxxxxxxx) = some_function_name(
        aaaaaaaaaaaaaaaaaaa,
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        ccccccccccccccc,
    ).unwrap_or_default()
    else {
        return;
    };
}
