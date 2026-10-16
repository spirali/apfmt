fn f() {
    let c = || {
        m! {}
    };
    let d = |x: u8| {
        m! { x }
    };
    let p = || e!();
    let q = || e![];
    let x = unsafe {
        m! {}
    };
    let y = if a {
        m! {}
    } else {
        n! {}
    };
    let z = if a { m!() } else { n!() };
    match 1 {
        0 => {
            m! {}
        }
        1 => {
            m! {}
        }
        2 => e!(),
        3 => e![x],
        4 => {
            m! { aaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, ccccccccccccccccccccccccccc }
        }
        _ => {
            m! {}
        }
    }
}
