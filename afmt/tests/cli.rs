//! The command line interface: input, output and errors

mod common;

use std::process::Command;

use common::output_with_stdin;

/// afmt without any rustfmt config file, which would otherwise be looked up from the current
/// directory and in the home directory of the user running the tests
fn afmt() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_afmt"));
    command.arg("--no-config");
    command
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
    let output = output_with_stdin(&mut afmt(), "fn f( {");
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
        let output = output_with_stdin(&mut afmt(), input);
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected,
            "{input:?}"
        );
    }
}
