fn main() {
    f(
        /* block */
        a, b,
    );
    f(
        // line
        a, b,
    );
    f(/* same line */ a, b);
    f(/* same line */ a, b);
    f(
        // before nested call
        g(1, 2),
    );
    f(
        /* first */
        // second
        a,
    );
}
