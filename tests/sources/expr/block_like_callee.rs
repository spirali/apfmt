fn f() {
    let a = || -> bool { true }();
    let b = (|| 1)();
    let c = { 1 }.max(2);
    let d = unsafe { g }(1);
    let e = || -> bool { true }().then(|| 1);
    let f = if x { g } else { h }(1);
    let g = match x {
        _ => h,
    }(1);
    let h = async { 1 }.await;
    let i = unsafe { a }[0];
    let j = if x { a } else { b }[0];
}
