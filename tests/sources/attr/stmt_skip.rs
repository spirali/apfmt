fn f() {
    #[rustfmt::skip]
    let   x = 1;
    #[rustfmt::skip]
    if   a {   b(); }
    #[rustfmt::skip]
    foo(  a,b );
    #[cfg(x)]
    #[rustfmt::skip]
    {   a  }
}
fn g() -> u8 {
    #[rustfmt::skip]
    {   1   }
}
