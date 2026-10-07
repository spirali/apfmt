fn main() {
    // The first line of the literal overflows on the call's line
    g(a, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
b", c);
    // Only the last line counts after the literal
    k("short
ttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttt");
    let s = "short
ttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttttt";
    // A multi-line item is never on a single line with other arguments
    m(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, "x
y", bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
}
