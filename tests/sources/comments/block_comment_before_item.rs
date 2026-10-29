fn f() {
    let x = foooooooooooooooo(/* document = */ true, /* other_long_name = */ false, some_long_argument_name);
    let x = foooooooooooooooo(aaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbb, /* document = */ ccccccccccccccccccc());
    let x = foooooooooooooooo(aaaaaaaaaaaaaaaaaaa /* after a */, /* before b */ bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb());
    let x = foooooooooooooooo(
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, // trailing a
        /* before b */ bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb(),
        // own line
        cccccccccccccccc,
    );
    let s = S { /* a */ aaaaaaaaaaaaaaaaaaaaaaa: 1, /* b */ bbbbbbbbbbbbbbbbbbbbbbbbbbbb: 2, cccccccccccc: 3 };
    let v = [/* first */ aaaaaaaaaaaaaaaaaaaaaaaaaaaaa(), /* second */ bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb()];
}
enum E { /* a */ Aaaaaaaaaaaaaaaaaaaaaa, /* b */ Bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, Cccccccccccccccccccccc(u32) }
struct S { /* a */ aaaaaaaaaaaaa: u32, /* b */ bbbbbbbbbbbbbbbbbbbb: u32,

    /* c */ c: u32 }
fn g() {
    let x = foooooooooooooooo(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, /* x */ /* y */ bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
    let x = foooooooooooooooo(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        /* x */
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
}
