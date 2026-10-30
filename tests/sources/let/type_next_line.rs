// rustfmt leaves these unformatted: a `let` type that does not fit on the line of `:` goes on the
// next line, as for `const`
fn f() {
    let name: aaaa::bbbbbbbbbb::cccccccccccccc::Dddddddddddddddddddddddddddddddddddddddddddddddddddddd = Ddddddd::new(1);
    let name: aaaa::bbbbbbbbbb::cccccccccccccc::Dddddddddddddddddddddddddddddddddddddddddddddddddddddd;
    let Foo { aaaaaaa, bbbbbbb }: aaaa::bbbbbbbbbb::cccccccccccccc::Ddddddddddddddddddddddddddddddd = x;
}
