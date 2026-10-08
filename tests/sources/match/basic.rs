fn f() {
    match x { A => 1, B => { 2 } C => { let y = 1; y } D => {} E => if a { b } else { c }, F => match y { G => 1, H => 2 }, }
    match x {
        Aaaaaaaaaaaaaaaaaaaaaa(xxxxxxxxx) => some_function_name(argument_one, argument_two, argument_three),
        Bbbbbbbbbbbbbbbbbbbbbb(xxxxxxxxx) => aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        Cccccccccccccccccccccc(xxxxxxxxx) => a_function_call(another_call(argument1, argument2, argument3x)),
        Dddddddddddddddddddddd(xxxxxxxxx) => Foo { field_one: aaaaaaaaaaaaaaaa, field_two: bbbbbbbbbbbbbbbbb },
        | X | Y => 4,
        Z => xs.iter().map(|x| x.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa()).filter(|x| x.bbbbbbbbbbbbbbbbbbb()).count(),
        // comment
        W => 5, // trailing
        V => { 6 } // after block
        U => |x| x + 1,
        T => |x| { let y = x; y },
        S => unsafe { foo() },
        Q => (),
        P => return,
    }
    let x = match foo.bar.baz() { _ => {} };
    match aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb(cccccccccccccccccccc, ddddddddddddddd) { _ => {} }
    match x {
        #[cfg(a)]
        A => 1,

        B => 2,
    }
    match x {}
    let v = foo(match x { A => 1, B => 2 });
}
