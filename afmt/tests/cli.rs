//! The command line interface: input, output and errors

use std::process::{Command, Stdio};

fn afmt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_afmt"))
}

#[test]
fn missing_file_is_an_error_not_a_panic() {
    let output = afmt().arg("does/not/exist.rs").output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.starts_with("afmt: cannot read does/not/exist.rs: "),
        "{stderr}"
    );
    assert!(!stderr.contains("panicked"), "{stderr}");
}

#[test]
fn syntax_error_exits_with_code_1() {
    let mut child = afmt()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(&mut child.stdin.take().unwrap(), b"fn f( {").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn output_ends_with_one_newline_unless_empty() {
    // An empty file stays empty (rustfmt writes a single newline)
    for (input, expected) in [
        ("", ""),
        ("\n\n", ""),
        ("fn f() {}", "fn f() {}\n"),
        ("fn f() {}\n\n", "fn f() {}\n"),
    ] {
        let mut child = afmt()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        std::io::Write::write_all(&mut child.stdin.take().unwrap(), input.as_bytes()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected,
            "{input:?}"
        );
    }
}
