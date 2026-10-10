// A right-hand side that fits on the next line moves there instead of breaking on the line of `=`
// (Style Guide `let`: "try splitting after the `=`, if the declaration fits on two lines")
fn f() {
    let xxxxxxxxxxxxxxxxxxxxx: SomeLongType = aaaaaaaaaaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb;
    xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx = aaaaaaaaaaaaaaaaaaaaaaaa && bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb;
    xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx += aaaaaaaaaaaaaaaaaaaaaaaa * bbbbbbbbbbbbbbbbbbbbbbbbbbbb;
    let xxxxxxxxxxxxxxxxxxxxxxxxxx = aaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbb(ccccccccccccccccccc, ddddddd);
    let xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx = aaaaaaaaaaaaaaa(bbbbbbbbbbbbbbbbbbbb) + cccccccccccccc(ddddddddddddddddd);
    let xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx = !aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.is_empty() && bbbbbbbbbbbbbbbbb;
    // Too long for the next line too: broken on the line of `=`
    let xxxxxxxxxxxxxxxxxxxxxxxxxxxx = aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb + ccccccccccccccccccccccc;
    // Wider than `chain_width` on the next line: broken on the line of `=`
    let xxxxxxxxxxxxxxxxxxxxxxxxxxxx = aaaaaaaaaaaaaaaaaaaa.bbbbbbbbbbbbbbbbbbbbbb().cccccccccccccccccccc();
}

const XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX: usize = AAAAAAAAAAAAAAAAAAAAAAAAAAA + BBBBBBBBBBBBBBBBBBBBBBBBB;
