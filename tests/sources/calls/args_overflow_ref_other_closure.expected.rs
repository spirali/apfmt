// Another closure, also after `&` / `&mut`, stops the last one from overflowing (Style Guide);
// rustfmt counts only closures without `&`

fn f() {
    two_closures(
        &|a| a + 1,
        &mut |b| {
            f(b);
            g(b);
        },
    );
    closure_first(
        |a| a + 1,
        &mut |b| {
            f(b);
            g(b);
        },
    );
    ref_closure_first(
        &|a| a + 1,
        |b| {
            f(b);
            g(b);
        },
    );
}
