fn f() {
    query(r#"
SELECT * FROM table
"#);
    query(r#"
  SELECT * FROM table
  "#);
    query(r#"
        SELECT * FROM table
    "#);
    query(r#"
        SELECT
    "#);
    query(r#"
    "#);
    query(r#"
        SELECT * FROM table"#);
    query(r#"SELECT * FROM table
    "#);
    query("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
b");
    query("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa \
b");
    query("a \
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    log::warn!("You have set an idle timeout longer than 10 minutes. This can result in \
wasted allocation duration.");
    return Err(anyhow::anyhow!("Allocation queue has running jobs, so it will \
not be removed. Use `--force` if you want to remove the queue anyway"));
    let jdef = JobDef::parse(r#"
        [[task]]
        command = ["sleep", "1"]
    "#)?;
    println!(
        "first {} line
second line",
        a
    );
    let x = query(
        a,
        "first line \
             second",
    );
    // Also under an operator, and in a struct literal, a tuple or a macro
    process_message(&"first line of the message \
second line");
    let message = Message {
        text: "first line \
second line",
    };
    let pair = (
        first_value,
        "first line \
second line",
    );
    process_message(format!(
        "first line {} \
second line",
        a
    ));
}
