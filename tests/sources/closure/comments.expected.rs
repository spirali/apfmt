fn f() {
    let f = |x| {
        // c
        x
    };
    let f = |x| x; // after
    let f = |x| {
        x /* end */
    };
}
