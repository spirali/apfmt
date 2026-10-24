fn main() {
    // The first line of the literal overflows on the call's line
    g(
        a,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
b",
        c,
    );
    // A literal spanning lines hugs the call as in the source; rustfmt hugs it only after a callee
    // shorter than `tab_spaces`
    k("short
ttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttt");
    let s = "short
ttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttt";
    // A multi-line item is never on a single line with other arguments
    m(
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        "x
y",
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
    );
}
