// Simple arguments as in rustfmt: also `&x`, `a.b`, `a as T`, `a[i]`, `a?` and `[a; n]`
fn f() {
    ffffffffffffffffffff(&aaa, bbbbbbb, ccccc, ddddddd, eeeeeee, fffffff, ggggg, hhhhhhh, iiiii, jjjj, kkkkkk);
    ffffffffffffffffffff(aa.bb, bbbbbbb, ccccc, ddddddd, eeeeeee, fffffff, ggggg, hhhhhhh, iiiii, jjjj, kkkkk);
    ffffffffffffffffffff(a as u8, bbbbbbb, ccccc, ddddddd, eeeeeee, fffffff, ggggg, hhhhhhh, iiiii, jjjj, kkkk);
    ffffffffffffffffffff(a[0], bbbbbbb, ccccc, ddddddd, eeeeeee, fffffff, ggggg, hhhhhhh, iiiii, jjjj, kkkkkkk);
    ffffffffffffffffffff(a?, bbbbbbb, ccccc, ddddddd, eeeeeee, fffffff, ggggg, hhhhhhh, iiiii, jjjj, kkkkkkkk);
    ffffffffffffffffffff([a; 3], bbbbbbb, ccccc, ddddddd, eeeeeee, fffffff, ggggg, hhhhhhh, iiiii, jjjj, kkkk);
    ffffffffffffffffffff(a::b, bbbbbbb, ccccc, ddddddd, eeeeeee, fffffff, ggggg, hhhhhhh, iiiii, jjjj, kkkkkk);
    ffffffffffffffffffff((a), bbbbbbb, ccccc, ddddddd, eeeeeee, fffffff, ggggg, hhhhhhh, iiiii, jjjj, kkkkkkk);
}
