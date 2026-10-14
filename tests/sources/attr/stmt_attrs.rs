fn f() {
    #[cfg(unix)]
    if a { b(); }
    #[cfg(feature = "git")]
    { c(); }
    #[allow(unused)] // comment
    unsafe { d(); }
    #[cfg(x)]
    unsafe { d() }
    #[cfg(x)] // why
    #[allow(unused)]
    c_config.flag("-Wno-unused-parameter").flag("-Wno-unused-but-set-variable").flag("-Wno-trigraphs");
    #[cfg(debug_assertions)]
    return   3;
    #[cfg(x)]
    for a in b { c(); }
    #[cfg(x)]
    match a { _ => {} }
    #[cfg(x)]
    loop {}
    let x = {
        #[cfg(not(feature = "git"))]
        { "a" }
    };
    x.inspect_err(|e| {
        #[allow(clippy::print_stderr)]
        {
            eprintln!("{e}");
        }
    });
}
fn g() -> u8 {
    #[cfg(x)]
    { 1 }
    #[cfg(not(x))]
    { 2 }
}
fn h() -> i32 {
    #[cfg(x)]
    !a;
    #[cfg(not(unix))]
    -1
}
