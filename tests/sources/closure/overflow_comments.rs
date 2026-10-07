fn f() {
    foo(|x| some_long_function_name(aaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbccccccccc, dddddddddddd), // c
    );
    foo(a, |x| some_long_function_name(aaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbccccccccc, ddddddddddd) /* c */);
    let v = a.b(|x| x.c(|y| y.d(|z| z.e(|w| w.fffffffffffffffffffffffffffffffff().gggggggggggggggggggggggggggggggg()))));
    (|x| x)(1).foo();
    x?;
    a[0][1].b()?[2];
}
