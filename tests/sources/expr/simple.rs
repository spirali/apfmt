fn f() {
    let a = (aaaaaaaaaaaaaaaaaaa + 1)..bbbbbbbbbbbbbbbbbbbbbbbbb;
    let b = x + 1..y;
    let c = a..;
    let r = ..=b;
    let full = ..;
    let g = &mut x as *const u8 as usize;
    let h = & raw const x;
    let i = &&x;
    let _ = x;
    _ = f();
}

fn jumps() -> u32 {
    let v = 'outer: {
        if a { break 'outer 1; }
        if b { break 'outer; }
        continue 'outer;
    };
    return;
}

fn g() {
    return aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb;
}
