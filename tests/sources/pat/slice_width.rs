// A slice pattern is on a single line when its patterns are at most `array_width` (60) wide, as
// an array; otherwise one pattern per line, also simple ones (not filled)
fn f() {
    let [aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, cccccccccccccccccc, ddddddddd] = x;
    let [aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, cccccccccccccccccc, dddddddddd] = x;
    let [aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, cccccccccccccccccc, ddddddddddd] = x;
    let [a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z, aa, bb] = x;
    let [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23] = x;
    let [aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, cccccccccccccccccc, Some(dddddddd), ..] = x;
    let [Foo { aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, cccccccccccccccccc, ddddddddddddddddd }] = x;
    match x {
        [aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, cccccccccccccccccc, ddddddddddd, eeeeeeeee] => {}
        Some([aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, cccccccccccccccccc, ddddddddddd, eee]) => {}
    }
    let [aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, cccccccccccccccccc, ddddddddddd]: [u8; 4] = x;
    let [aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, cccccccccccccccccc, ddddd] = x;
    let [aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, cccccccccccccccccc, dddddd] = x;
    let [aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, cccccccccccccccccc, 11111] = x;
    let [aaaaaaaaaaaa, bbbbbbbbbbbbbbbb, cccccccccccccccccc, 111111] = x;
}
