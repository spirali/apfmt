// Nested plain blocks of a closure body, `{ { x } }`: the outer braces are removed, the innermost
// ones as those of a single block (rustfmt keeps the outer ones of a long body and after broken
// parameters)
fn f() {
    let c = |x| { { x } };
    let c = |x| { { { x } } };
    let c = |x| { { g(); x } };
    let c = |x| { { aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa(bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccccccccccc) } };
    let c = |x| -> u8 { { x } };
    let c = |x| { unsafe { x } };
    let c = |x| { { m! {} } };
    let c = |x| { { x } // c
    };
    let c = |aaaaaaaaaaaaaaaaaaaaaaaaaaa: Aaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb: Bbbbbbbbbbbbbb| { { x } };
    let c = |aaaaaaaaaaaaaaaaaaaaaaaaaaa: Aaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb: Bbbbbbbbbbbbbb| { { g(); x } };
    let c = |aaaaaaaaaaaaaaaaaaaaaaaaaaa: Aaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb: Bbbbbbbbbbbbbb| { x };
}
