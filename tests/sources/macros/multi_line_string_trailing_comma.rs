fn f() -> String {
    format!(
        r#"<div class="stat-card">
        <div class="stat-value">{value}</div>
        </div>"#,
    )
}

fn g() -> String {
    foo(
        r#"<div class="stat-card">
        </div>"#,
    )
}

// Without a trailing comma, hugged as in the source (rustfmt hugs a literal only after a callee
// shorter than `tab_spaces`)
fn h() -> String {
    foo(r#"<div class="stat-card">
        </div>"#)
}

fn i() {
    println!("x {}", a,);
    let pair = (r#"first line
        second line"#,);
    let text = format!("{}", r#"first line
        second line"#,);
}
