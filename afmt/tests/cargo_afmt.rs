//! `cargo afmt`: the files of Cargo packages, their editions, `--check`

mod common;

use common::{TestDir, stderr, stdout};

fn package(name: &str, edition: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"{edition}\"\n")
}

const UNFORMATTED: &str = "fn  f( ) {g( );}\n";
const FORMATTED: &str = "fn f() {\n    g();\n}\n";

#[test]
fn formats_the_module_tree() {
    let ws = TestDir::new("tree");
    ws.write("Cargo.toml", &package("a", "2024"));
    ws.write("src/lib.rs", &format!("mod b;\nmod c;\n{UNFORMATTED}"));
    ws.write("src/b.rs", &format!("mod d;\n{UNFORMATTED}"));
    ws.write("src/b/d.rs", UNFORMATTED);
    ws.write("src/c/mod.rs", FORMATTED);

    // `--check` lists the files that would change, writes nothing and fails
    let output = ws.cargo_afmt("", &["--check"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "src/b/d.rs\nsrc/b.rs\nsrc/lib.rs\n");
    assert_eq!(ws.read("src/b/d.rs"), UNFORMATTED);

    let output = ws.cargo_afmt("", &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        ws.read("src/lib.rs"),
        format!("mod b;\nmod c;\n{FORMATTED}")
    );
    assert_eq!(ws.read("src/b.rs"), format!("mod d;\n{FORMATTED}"));
    assert_eq!(ws.read("src/b/d.rs"), FORMATTED);

    let output = ws.cargo_afmt("", &["--check"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn all_targets_are_formatted() {
    let ws = TestDir::new("targets");
    ws.write("Cargo.toml", &package("a", "2024"));
    for file in [
        "src/lib.rs",
        "src/main.rs",
        "src/bin/other.rs",
        "tests/t.rs",
        "examples/e.rs",
        "build.rs",
    ] {
        ws.write(file, UNFORMATTED);
    }
    let output = ws.cargo_afmt("", &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    for file in [
        "src/lib.rs",
        "src/main.rs",
        "src/bin/other.rs",
        "tests/t.rs",
        "examples/e.rs",
        "build.rs",
    ] {
        assert_eq!(ws.read(file), FORMATTED, "{file}");
    }
}

/// Valid in the 2021 edition only: `gen` is a keyword since 2024
const GEN_NAME: &str = "fn main() { let gen = rng.gen(); }\n";

#[test]
fn edition_of_each_target() {
    let ws = TestDir::new("editions");
    ws.write(
        "Cargo.toml",
        &format!(
            "{}[[bin]]\nname = \"old\"\npath = \"src/old.rs\"\nedition = \"2021\"\n",
            package("a", "2024")
        ),
    );
    ws.write("src/lib.rs", UNFORMATTED);
    ws.write("src/old.rs", GEN_NAME);
    let output = ws.cargo_afmt("", &["-v"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("[bin (2021)] src/old.rs"),
        "{}",
        stdout(&output)
    );
    assert_eq!(
        ws.read("src/old.rs"),
        "fn main() {\n    let gen = rng.gen();\n}\n"
    );
    assert_eq!(ws.read("src/lib.rs"), FORMATTED);
}

/// A workspace with the members `a` (edition 2021) and `b`, both unformatted
fn two_members(name: &str) -> TestDir {
    let ws = TestDir::new(name);
    ws.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"3\"\n",
    );
    ws.write("a/Cargo.toml", &package("a", "2021"));
    ws.write("a/src/lib.rs", &format!("{GEN_NAME}{UNFORMATTED}"));
    ws.write("b/Cargo.toml", &package("b", "2024"));
    ws.write("b/src/lib.rs", UNFORMATTED);
    ws
}

#[test]
fn packages_of_the_workspace_root() {
    let ws = two_members("root");
    let output = ws.cargo_afmt("", &["--check"]);
    assert_eq!(
        stdout(&output),
        "a/src/lib.rs\nb/src/lib.rs\n",
        "{}",
        stderr(&output)
    );
}

#[test]
fn package_of_the_current_directory() {
    let ws = two_members("member");
    // also from a subdirectory of the package
    for dir in ["a", "a/src"] {
        let output = ws.cargo_afmt(dir, &["--check", "-q"]);
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    }
    ws.cargo_afmt("a", &[]);
    assert_eq!(ws.read("b/src/lib.rs"), UNFORMATTED);
    assert!(ws.cargo_afmt("a", &["--check"]).status.success());
}

#[test]
fn selected_and_all_packages() {
    let ws = two_members("selected");
    let output = ws.cargo_afmt("a", &["-p", "b"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(ws.read("b/src/lib.rs"), FORMATTED);
    assert_eq!(ws.read("a/src/lib.rs"), format!("{GEN_NAME}{UNFORMATTED}"));
    let output = ws.cargo_afmt("a", &["--all"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(ws.read("a/src/lib.rs").ends_with(FORMATTED));
    let output = ws.cargo_afmt("", &["-p", "nope"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("`nope`"), "{}", stderr(&output));
}

#[test]
fn manifest_path() {
    let ws = two_members("manifest");
    let manifest = ws.path("b/Cargo.toml");
    let output = ws.cargo_afmt(
        "",
        &["--check", "--manifest-path", manifest.to_str().unwrap()],
    );
    assert_eq!(stdout(&output), "b/src/lib.rs\n", "{}", stderr(&output));
}

#[test]
fn config_files_are_found_per_file() {
    let ws = TestDir::new("config");
    ws.write("Cargo.toml", &package("a", "2024"));
    ws.write("rustfmt.toml", "tab_spaces = 2\n");
    ws.write("src/lib.rs", "mod off;\nfn f() { g(); }\n");
    ws.write("src/off/afmt.toml", "disable_all_formatting = true\n");
    ws.write("src/off/mod.rs", UNFORMATTED);
    let output = ws.cargo_afmt("", &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(ws.read("src/lib.rs"), "mod off;\nfn f() {\n  g();\n}\n");
    assert_eq!(ws.read("src/off/mod.rs"), UNFORMATTED);
    // `--no-config` reads neither
    ws.cargo_afmt("", &["--no-config"]);
    assert_eq!(ws.read("src/off/mod.rs"), FORMATTED);
}

#[test]
fn a_broken_config_fails_every_file_of_its_directory() {
    let ws = TestDir::new("broken-config");
    ws.write("Cargo.toml", &package("a", "2024"));
    ws.write("src/lib.rs", "mod bad;\nfn f() { g(); }\n");
    ws.write("src/bad/afmt.toml", "max_width = 0\n");
    ws.write("src/bad/mod.rs", "mod a;\n");
    ws.write("src/bad/a.rs", UNFORMATTED);
    let output = ws.cargo_afmt("", &[]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr(&output);
    for file in ["src/bad/mod.rs: ", "src/bad/a.rs: "] {
        assert!(stderr.contains(file), "{stderr}");
    }
    assert_eq!(ws.read("src/bad/a.rs"), UNFORMATTED);
    assert_eq!(ws.read("src/lib.rs"), "mod bad;\nfn f() {\n    g();\n}\n");
}

#[test]
fn an_error_in_one_file_does_not_stop_the_others() {
    let ws = TestDir::new("errors");
    ws.write("Cargo.toml", &package("a", "2024"));
    ws.write(
        "src/lib.rs",
        "mod bad;\nmod good;\n#[cfg(windows)]\nmod windows;\n",
    );
    ws.write("src/bad.rs", "fn f( {\n");
    ws.write("src/good.rs", UNFORMATTED);
    let output = ws.cargo_afmt("", &[]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr(&output);
    assert!(stderr.contains("src/bad.rs: "), "{stderr}");
    // a module without a file (e.g. for another platform) is a warning
    assert!(
        stderr.contains("warning: ") && stderr.contains("`windows`"),
        "{stderr}"
    );
    assert_eq!(ws.read("src/good.rs"), FORMATTED);
    assert_eq!(ws.read("src/bad.rs"), "fn f( {\n");
}
