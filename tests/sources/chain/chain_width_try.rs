// Chains of exactly `chain_width` (60) columns stay on one line, with or without a final `?`
// (rustfmt breaks the one ending with `?`)
fn f() -> Result<(), E> {
    let f = env.aaaa(path).bbbbbbbbbbbbb(x).cccccccccccccccccccccccc(x);
    let f = env.aaaa(path).bbbbbbbbbbbbb(x).cccccccccccccccccccccccc(x)?;
    let file = env.new_writable_file(path, EnvOptions::new()).map_err(r2e)?;
    if self.connection.handle_shutdown(&request).into_diagnostic()? {
        return Ok(());
    }
    // 61 columns: broken in both
    let f = env.aaaa(path).bbbbbbbbbbbbb(x).ccccccccccccccccccccccccc(x)?;
    Ok(())
}
