// Format-string macros: with simple arguments, those in front of the format string and those after it
// each on one line (Style Guide "Format string macros")
fn f() {
    println!("a very long format string with {} and {} and {} placeholders xxxx", first, second, third);
    println!("a very long format string with {} and {} and {} placeholders xxxx", aa.b(), c + d, e);
    println!("a very long format string with {} and {} and {} placeholders xxxx", aa, c, eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee, ffffffffffffffffffff, ggggg);
    assert_eq!(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, "message {}", ccccc);
    assert_eq!(aa.b(), c, "a very long format string with {} and {} and {} placeholders xxxx", e);
    assert!(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa == bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, "message {}", ccccccc);
    write!(f, "a very long format string with {} and {} and {} placeholders xxxxxxxxxxxx", e, g)?;
    eprintln!("a very long format string with {} and {} and {} placeholders xxxxxxxxxxxxxxxx", e, g,);
    println!("a very long format string with {} and {} and {} placeholders xxxxxxxxxxxxxxxxxx", "lit", 1);
    std::println!("a very long format string with {} and {} and {} placeholders xxxxxxxxxxxx", e, g);
    debug!("a very long format string with {} and {} and {} placeholders xxxxxxxxxxxxxxxxxx", e, g);
    trace!("a very long format string with {} and {} and {} placeholders xxxxxxxxxxxxxxxxxx", e, g);
    println!("a very long format string with {} and {} and {} placeholders xxxxxxxxxxxxx", &e, !g);
    println!("a very long format string with {} and {} and {} placeholders xxxxxxxxxxxxx", e.f, g[0]);
    println!("{}", aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
    panic!(concat!("a", "b"), xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx, yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy);
}
