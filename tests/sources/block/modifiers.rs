fn f() {
    unsafe { g(); h(); }
    'a: {
        1
    }
    let y = async move { a(); 2 };
    let z = const { let x = 3; x };
    let g = gen { f(); };
    unsafe {
        // comment
        g();
    }
    let l = 'b: { if c { break 'b 1; } 2 };
}
