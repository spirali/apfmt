// An item much wider than the others (a gap of more than `trailing_comment_align_threshold`, 40)
// keeps a single space, narrower ones join the column (rustfmt aligns them all). A line that the
// column would make exceed `max_width` keeps a single space (rustfmt makes it, or gives up the whole
// list).
enum E {
    A = 1,  // a
    Bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb = 2, // b
    Cc = 3, // c
    D = 4,  // d
}
fn f() {
    // Narrow items join the column
    let candidates = [
        Candidate::new_utf_8(),                                      // 0
        Candidate::new_iso_2022_jp(allow_iso_2022_jp),               // 1
        Candidate::new_visual(&SINGLE_BYTE_DATA[ISO_8859_8_INDEX]),  // 2
        Candidate::new_gbk(),                                        // 3
        Candidate::new_latin(&SINGLE_BYTE_DATA[WINDOWS_1252_INDEX]), // 4
    ];
    call(
        a,  // short comment
        bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, // b
        cc, // third
    );
    call(
        aaaaaaaa, // a comment that is long enough to exceed the width when it is aligned xxxxxxxx
        bbbbbbbbbbbbbbbbbb, // b
    );
}
