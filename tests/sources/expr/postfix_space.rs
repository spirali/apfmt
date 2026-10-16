fn f() -> Result<(), E> {
    let a = x? == y;
    let b = x? + 1;
    x? = 3;
    let c = x? as u8;
    let d = x??.y?[0]?(1)?;
    let e = (x?, &y?, x? - y?..z?);
    if x? {
        g()?
    }
    let r = &a[1..];
    match 0 {
        2.. => {}
        _ => {}
    }
    Ok(())
}
