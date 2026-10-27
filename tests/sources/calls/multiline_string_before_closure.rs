// A closure does not hug the call after a string spanning lines: the arguments are one per line
fn f() {
    ParseTest::parse(r#"
name myapp
port 8080
"#, |t, c: Config| {
        t.assert_is(&c.name, "myapp", "myapp", None, None);
        t.assert_is(&c.port, 8080u16, "8080", None, None);
    });
    check("first line
second line", 42, |x| {
        x.run();
    });
}
