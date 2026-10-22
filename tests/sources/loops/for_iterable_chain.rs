fn f() {
    for (aaaaaaaaaaaa, bbbbbbbbbbbbbbb) in xxxxxxxxxxxxxxxxxxxxxx.iter().zip(yyyyyyyyyyyyyyyyyyyyyyyyy.iter()) {
        x;
    }
    for key in checks.iter().map(|c| &c.key).chain(mutations.iter().map(|m| &m.key)).chain(more_keys) {
        x;
    }
    for entry in std::fs::read_dir(&path).with_context(|| format!("Failed reading directory '{}'", path.display()))? {
        x;
    }
    for (aaaaaaaaaaaa, bbbbbbbbbbbbbbb) in xxxxxxxxxxxxxxx.iter().filter(|x| { let y = x; y.is_some() }) {
        x;
    }
}
