fn f() {
    if
    // c
    a {
        x();
    } else if
    // d
    b {
        y();
    } else {
        w();
    }
    while
    // c
    a {
        x();
    }
    for x in
    // c
    a {
        x();
    }
    match
    // c
    a {
        A => 1,
    }
    if /* c */ a {
        x();
    }
    let v = if
    // c
    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa && bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb {
        1
    } else {
        2
    };
}
fn g() {
    if // c
    cond {
        x();
    }
    for x in // c
    a {
        x();
    }
    for x in a // c
    {
        x();
    }
}
