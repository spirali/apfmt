fn f() {
    let S { a, b: c, .. } = x;
    let S { aaaaa, bbbbbbbbb } = x;
    let S { aaaaa: Some(xxxxxxxxxxxxxxxxxxxxxx), bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb: yyyyyyyyyyyyyyyyyyyyyyyyyy } = x;
    let S { .. } = x;
    let S {} = x;
    let S { ref mut a, b: _ } = x;
    let [a, b, .., z] = x;
    let [first, rest @ ..] = x;
    let &mut (ref mut a, -1..=5) = x;
    let (A::B | A::C) = x;
    let E::V = x;
    let (Some(Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa::Bbbbbbbbbbbbbbbbbbbbbbbb | Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa::Ccccccccccc)) = x;
    if let A::B | A::C | A::D = x {}
    if let Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa::Bbbbbbbbbbbbbbbbbbbbbbbb | Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa::Ccccccccccc = x {}
    let (b'a'..=b'z', 'x', "s") = x;
}
