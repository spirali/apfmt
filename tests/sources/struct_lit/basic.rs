fn f() {
    let a = Foo { x, y: 1 };
    let a = Foo { xxxxxxxx, yyyyyyyyy: 1 };
    let a = Foo { field1, ..an_expr };
    let a = Foo { field1, ..Default::default() };
    let a = Foo { field1: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb), };
    let a = Foo { field1: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb };
    let a = Foo {};
    let a = Foo { /* c */ };
    let a = a::b::Foo::<T> { x: 1 };
    let a = Foo { 0: 1, 1: 2 };
    foo(Bar { field: whatever, other_field: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, third: b });
    let a = Foo {
        // c
        x: 1, // d
        #[cfg(a)]
        y,
    };
    let f = || Foo { field1, field2: 0, field3: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa };
    let a = Foo { ..Default::default() };
    let a = Foo { x, .. };
}
