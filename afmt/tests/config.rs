//! The command line: finding and reading the rustfmt config file, `--edition`

mod common;

use common::{TestDir, stderr, success};

/// A source whose indentation shows `tab_spaces`
const NESTED: &str = "fn main() { if x { y(); } }\n";

fn indented(spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    format!("fn main() {{\n{indent}if x {{\n{indent}{indent}y();\n{indent}}}\n}}\n")
}

#[test]
fn config_next_to_the_file() {
    let dir = TestDir::new("next-to-file");
    dir.write("project/rustfmt.toml", "tab_spaces = 2\n");
    let file = dir.write("project/src/main.rs", NESTED);
    assert_eq!(dir.format_file(&file, &[]), indented(2));
}

#[test]
fn nearest_config_wins() {
    let dir = TestDir::new("nearest");
    dir.write("project/rustfmt.toml", "tab_spaces = 2\n");
    dir.write("project/src/rustfmt.toml", "tab_spaces = 3\n");
    let file = dir.write("project/src/main.rs", NESTED);
    assert_eq!(dir.format_file(&file, &[]), indented(3));
}

#[test]
fn hidden_config_is_preferred() {
    let dir = TestDir::new("hidden");
    dir.write("project/rustfmt.toml", "tab_spaces = 2\n");
    dir.write("project/.rustfmt.toml", "tab_spaces = 3\n");
    let file = dir.write("project/main.rs", NESTED);
    assert_eq!(dir.format_file(&file, &[]), indented(3));
}

#[test]
fn config_of_a_file_is_not_looked_up_from_the_cwd() {
    let dir = TestDir::new("file-not-cwd");
    dir.write("cwd/rustfmt.toml", "tab_spaces = 2\n");
    let file = dir.write("project/main.rs", NESTED);
    let output = dir.afmt(&dir.path("cwd"), &[file.to_str().unwrap()], "");
    assert_eq!(success(output), indented(4));
}

#[test]
fn stdin_config_is_looked_up_from_the_cwd() {
    let dir = TestDir::new("stdin");
    dir.write("cwd/rustfmt.toml", "tab_spaces = 2\n");
    std::fs::create_dir_all(dir.path("cwd/sub")).unwrap();
    let output = dir.afmt(&dir.path("cwd/sub"), &[], NESTED);
    assert_eq!(success(output), indented(2));
}

#[test]
fn config_in_the_home_directory() {
    let dir = TestDir::new("home");
    dir.write("home/.rustfmt.toml", "tab_spaces = 2\n");
    let file = dir.write("project/main.rs", NESTED);
    assert_eq!(dir.format_file(&file, &[]), indented(2));
}

#[test]
fn config_in_the_user_configuration_directory() {
    let dir = TestDir::new("config-dir");
    dir.write("home/.config/rustfmt/rustfmt.toml", "tab_spaces = 2\n");
    let file = dir.write("project/main.rs", NESTED);
    assert_eq!(dir.format_file(&file, &[]), indented(2));
}

#[test]
fn project_config_is_preferred_to_the_home_one() {
    let dir = TestDir::new("project-over-home");
    dir.write("home/.rustfmt.toml", "tab_spaces = 2\n");
    dir.write("project/rustfmt.toml", "tab_spaces = 3\n");
    let file = dir.write("project/main.rs", NESTED);
    assert_eq!(dir.format_file(&file, &[]), indented(3));
}

#[test]
fn explicit_config_path() {
    let dir = TestDir::new("config-path");
    dir.write("project/rustfmt.toml", "tab_spaces = 2\n");
    let config = dir.write("configs/three.toml", "tab_spaces = 3\n");
    dir.write("configs/dir/rustfmt.toml", "tab_spaces = 5\n");
    let file = dir.write("project/main.rs", NESTED);
    let config_dir = dir.path("configs/dir");
    assert_eq!(
        dir.format_file(&file, &["--config-path", config.to_str().unwrap()]),
        indented(3)
    );
    assert_eq!(
        dir.format_file(&file, &["--config-path", config_dir.to_str().unwrap()]),
        indented(5)
    );
}

#[test]
fn missing_explicit_config_path_fails() {
    let dir = TestDir::new("config-path-missing");
    std::fs::create_dir_all(dir.path("empty")).unwrap();
    for path in ["missing.toml", "empty"] {
        let output = dir.afmt(&dir.root, &["--config-path", path], NESTED);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("no config file found"));
    }
}

#[test]
fn no_config() {
    let dir = TestDir::new("no-config");
    dir.write("project/rustfmt.toml", "tab_spaces = 2\n");
    dir.write("home/.rustfmt.toml", "tab_spaces = 3\n");
    let file = dir.write("project/main.rs", NESTED);
    assert_eq!(dir.format_file(&file, &["--no-config"]), indented(4));
}

#[test]
fn disable_all_formatting_keeps_the_source() {
    let dir = TestDir::new("disabled");
    dir.write("project/rustfmt.toml", "disable_all_formatting = true\n");
    let source = "fn   main( ) {x( );}";
    let file = dir.write("project/main.rs", source);
    let output = dir.afmt(&dir.root, &[file.to_str().unwrap()], "");
    assert_eq!(success(output), source);
    assert_eq!(
        dir.format_file(&file, &["--no-config"]),
        "fn main() {\n    x();\n}\n"
    );
}

/// Valid in the 2021 edition only: `gen` is a keyword since 2024
const GEN_NAME: &str = "fn main() { let gen = rng.gen(); }\n";
/// Valid since the 2018 edition: `async` is a name in 2015
const ASYNC_FN: &str = "async fn f() { g().await; }\n";

#[test]
fn edition_flag() {
    let dir = TestDir::new("edition");
    let output = dir.afmt(&dir.root, &["--edition", "2021"], GEN_NAME);
    assert_eq!(
        success(output),
        "fn main() {\n    let gen = rng.gen();\n}\n"
    );
    for edition in [None, Some("2024")] {
        let args: Vec<_> = edition.iter().flat_map(|e| ["--edition", e]).collect();
        let output = dir.afmt(&dir.root, &args, GEN_NAME);
        assert!(
            !output.status.success(),
            "`gen` is a keyword in edition {edition:?}"
        );
    }
    let output = dir.afmt(&dir.root, &["--edition", "2015"], ASYNC_FN);
    assert!(!output.status.success(), "`async fn` in edition 2015");
    let output = dir.afmt(&dir.root, &["--edition", "2018"], ASYNC_FN);
    assert_eq!(success(output), "async fn f() {\n    g().await;\n}\n");
}

#[test]
fn edition_flag_overrides_the_config() {
    let dir = TestDir::new("edition-override");
    dir.write("rustfmt.toml", "edition = \"2015\"\n");
    let output = dir.afmt(&dir.root, &[], ASYNC_FN);
    assert!(!output.status.success(), "the config sets edition 2015");
    let output = dir.afmt(&dir.root, &["--edition", "2021"], ASYNC_FN);
    assert_eq!(success(output), "async fn f() {\n    g().await;\n}\n");
}

#[test]
fn edition_from_the_config() {
    let dir = TestDir::new("edition-config");
    dir.write("rustfmt.toml", "edition = \"2021\"\n");
    let output = dir.afmt(&dir.root, &[], GEN_NAME);
    assert_eq!(
        success(output),
        "fn main() {\n    let gen = rng.gen();\n}\n"
    );
}

#[test]
fn invalid_edition_flag() {
    let dir = TestDir::new("edition-invalid");
    let output = dir.afmt(&dir.root, &["--edition", "2020"], NESTED);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("invalid edition"),
        "{}",
        stderr(&output)
    );
}

const CARGO_2021: &str = "[package]\nname = \"a\"\nedition = \"2021\"\n";

#[test]
fn edition_from_cargo_toml() {
    let dir = TestDir::new("cargo-edition");
    dir.write("project/Cargo.toml", CARGO_2021);
    let file = dir.write("project/src/bin/main.rs", GEN_NAME);
    assert_eq!(
        dir.format_file(&file, &[]),
        "fn main() {\n    let gen = rng.gen();\n}\n"
    );
    // the flag overrides it
    let output = dir.afmt(
        &dir.root,
        &["--edition", "2024", file.to_str().unwrap()],
        "",
    );
    assert!(
        !output.status.success(),
        "`gen` is a keyword in edition 2024"
    );
}

#[test]
fn edition_from_cargo_toml_for_stdin() {
    let dir = TestDir::new("cargo-edition-stdin");
    dir.write("project/Cargo.toml", CARGO_2021);
    std::fs::create_dir_all(dir.path("project/src")).unwrap();
    let output = dir.afmt(&dir.path("project/src"), &[], GEN_NAME);
    assert_eq!(
        success(output),
        "fn main() {\n    let gen = rng.gen();\n}\n"
    );
}

#[test]
fn cargo_toml_overrides_rustfmt_toml() {
    let dir = TestDir::new("cargo-over-rustfmt");
    dir.write("project/Cargo.toml", CARGO_2021);
    dir.write("project/rustfmt.toml", "edition = \"2024\"\n");
    let file = dir.write("project/src/main.rs", GEN_NAME);
    assert_eq!(
        dir.format_file(&file, &[]),
        "fn main() {\n    let gen = rng.gen();\n}\n"
    );
}

#[test]
fn cargo_toml_without_edition_is_2015() {
    let dir = TestDir::new("cargo-2015");
    dir.write("project/Cargo.toml", "[package]\nname = \"a\"\n");
    let file = dir.write("project/src/main.rs", ASYNC_FN);
    let output = dir.afmt(&dir.root, &[file.to_str().unwrap()], "");
    assert!(!output.status.success(), "`async fn` in edition 2015");
}

#[test]
fn edition_inherited_from_the_workspace() {
    let dir = TestDir::new("cargo-workspace");
    dir.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\n[workspace.package]\nedition = \"2021\"\n",
    );
    dir.write(
        "crates/a/Cargo.toml",
        "[package]\nname = \"a\"\nedition.workspace = true\n",
    );
    let file = dir.write("crates/a/src/lib.rs", GEN_NAME);
    assert_eq!(
        dir.format_file(&file, &[]),
        "fn main() {\n    let gen = rng.gen();\n}\n"
    );
}

#[test]
fn virtual_workspace_keeps_the_rustfmt_edition() {
    let dir = TestDir::new("cargo-virtual");
    dir.write("Cargo.toml", "[workspace]\nmembers = []\n");
    dir.write("rustfmt.toml", "edition = \"2021\"\n");
    let file = dir.write("src/main.rs", GEN_NAME);
    assert_eq!(
        dir.format_file(&file, &[]),
        "fn main() {\n    let gen = rng.gen();\n}\n"
    );
}

#[test]
fn broken_cargo_toml_fails() {
    let dir = TestDir::new("cargo-broken");
    dir.write("project/Cargo.toml", "[package]\nedition = \"2020\"\n");
    let file = dir.write("project/main.rs", NESTED);
    let output = dir.afmt(&dir.root, &[file.to_str().unwrap()], "");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("Cargo.toml"),
        "{}",
        stderr(&output)
    );
    // --no-config does not read it
    assert_eq!(dir.format_file(&file, &["--no-config"]), indented(4));
    // nor does --edition
    assert_eq!(dir.format_file(&file, &["--edition", "2021"]), indented(4));
}
