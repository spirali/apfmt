fn main() {
    f(
        // before a
        a,
        b, // after b
        // dangling
    );
    f(/* x */);
    f(// x
    );
    g(h(
        1, // one
        2,
    ));
    f(a /* c */);
}
