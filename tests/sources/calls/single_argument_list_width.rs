// A single argument or element that is an array, a tuple or a struct literal is limited only by its
// own width rule and `max_width`, not by the `fn_call_width` / `array_width` of the list around it
fn f() {
    let out = df.agg([col("aaaaaaaaaaaaaaaaaaaaa").sort_by([col("bbbbbbbbbbbbbb")], options)]);
    let out = df.agg([aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa]);
    let out = df.agg(&[aaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb(cccccccccccc)]);
    let out = df.agg([aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbb]);
    let out = df.agg((aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbb));
    assert_eq!(result, vec![(aaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb)]);
    let x = [[aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbb]];
    let x = [(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbb)];
    let x = ((aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbb),);
    // still broken: the list itself is too wide, or the argument is a call
    let out = df.agg([aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbb]);
    let out = df.agg(S { a: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa });
    let out = df.agg(vec![aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa]);
}
