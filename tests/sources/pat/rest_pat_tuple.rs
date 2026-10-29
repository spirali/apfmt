fn f(x: (u8, u8)) {
    let (..) = x;
    let (a,) = (x.0,);
    match x {
        (..) => {}
    }
    match y {
        S(..) => 1,
        (a, ..) => 2,
        (.., b) => 3,
    };
    let (
        aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb,
        ..,
    ) = z;
}
