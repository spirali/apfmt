// A single string literal spanning lines hugs the list if it starts on the line of the opening
// delimiter in the source, as Prettier and Ruff do; rustfmt hugs it only after a callee shorter
// than `tab_spaces`

fn hugged_in_source() {
    let expected = str![[r#"
Usage: example <FIRST>... -- <SECOND>...
"#]];
    query(r#"
SELECT * FROM t
"#);
    conn.execute(r#"
CREATE TABLE t (id INTEGER)
"#)?;
    let m = indoc!(r#"
        text
    "#);
    let v = Some(&"first line
second line");
    let x = vec![r#"
a
"#];
    cmd.arg("--x").arg(r#"
a
"#);
    let t = toml::from_str::<Config>(r#"
        name = "x"
    "#).unwrap();
}

fn broken_in_source() {
    f(
        r#"
SELECT * FROM t
"#,
    );
    let expected = str![[
        r#"
Usage: example
"#,
    ]];
    query(
        // comment
        r#"
SELECT * FROM t
"#,
    );
    query(
        /* comment */ r#"
SELECT * FROM t
"#,
    );
}

fn not_hugged() {
    // The first line does not fit
    let s = some_function_with_a_long_name_that_fills_the_line_almost_completely_aaaaaaaaaaaaaaaa(
        r#"
x
"#,
    );
    // More items, or a tuple, whose comma would follow the string
    assert_eq!(
        out,
        r#"
expected
"#
    );
    let pair = (
        r#"
a
"#,
    );
    let x = [
        r#"
a
"#,
        r#"
b
"#,
    ];
}
