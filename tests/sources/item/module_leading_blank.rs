// Blank lines at the start and the end of a module or extern block body are removed, as in a block
// (rustfmt keeps the one at the start of a module, and both in an extern block)
mod m {

    use x;

    fn f() {}

}

extern "C" {

    fn g();

}
