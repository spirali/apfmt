// A blank line between the elements of a broken chain is kept, also without a comment after it;
// rustfmt removes it
fn f() {
    let opts = WriteOptions::new()
        .set_disable_wal(opt.disable_wal())

        .set_sync(true);
    let opts = WriteOptions::new()
        .set_disable_wal(opt.disable_wal())

        // A comment
        .set_sync(true);
    // After the root; at most one blank line
    let opts = WriteOptions::new

        .set_disable_wal(opt.disable_wal())

        .set_sync(true);
    // On one line, there is nothing to keep it in
    let z = Foo::new().a(1).b(2).c(3);
}
