// Attributes that apfmt copies unchanged with their expression; they must not be lost
// (rustfmt drops the attribute of the closure body)
fn f() {
    let y = |e|   #[allow(unused)] {
        e
    };
    match   a {
        _ => #[allow(unused)] {
            b
        }
    }
    #[cfg(x)]
    a   = 1;
    if #[a] x.y()   {}
    while #[a] !x.f() {}
}
