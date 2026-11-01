fn f() {
    match x {
        Foo("a
b", c) => 1,
        Bar(Baz("a
b")) => 2,
        (r"a
b", d) => 3,
        _ => 4,
    }
}
