fn main() {
    // Hello
    //Comment2

    /* aa */

    /* foo */ f(); /* bar */ /* baz */ /* end */

    let a = 10; /* hello */
    test1(); /* a */ /* c */
    test2();

    a(); // a

    b(); /* x */

    let /* 0 */ x /* 1 */ = /* 2 */ "abc"; /* 3 * */

    f /* a */(/* b */ 1 /* c */, /* d */ 2 /* e */); /* f */

    let
        /* 0 */ x
        /* 1 */ =
        /* 2 */ "abc"; /* 3 * */

    f
        /* a */( /* b */
            1, /* c */
            /* d */
            2,
            /* e */
        ); /* f */

    abc();

    g(a /* a */);

    b();
    c();
}
