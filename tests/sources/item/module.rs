mod a;
pub mod b {
    use x;


    fn f() {}
}
mod c {}
mod d {
    // only a comment
}
pub(crate) mod e { mod f { fn g() {} } }
