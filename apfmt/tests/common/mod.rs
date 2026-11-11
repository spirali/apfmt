//! What the tests of the command line share: a temporary directory isolated from the config files
//! of the user running the tests, running `apfmt` and `cargo apfmt` in it

// Each test crate uses a part of it
#![allow(dead_code)]

use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A fresh directory for one test, removed when dropped, with an empty `home/` that the commands
/// use as the home directory (and `home/.config` as the user's configuration directory), so that
/// the config files of the user running the tests are never read
pub struct TestDir {
    pub root: PathBuf,
}

impl TestDir {
    pub fn new(name: &str) -> TestDir {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let count = COUNT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("apfmt-test-{name}-{}-{count}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("home/.config")).unwrap();
        TestDir { root }
    }

    pub fn path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    /// Writes `content` into the file at `path`, creating its directories
    pub fn write(&self, path: &str, content: &str) -> PathBuf {
        let path = self.path(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
        path
    }

    pub fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.path(path)).unwrap()
    }

    /// `program` run in `cwd`, with the home and configuration directories of the test
    fn command(&self, program: &str, cwd: &Path) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(cwd)
            .env("HOME", self.path("home"))
            .env("XDG_CONFIG_HOME", self.path("home/.config"))
            .env("APPDATA", self.path("home/.config"));
        command
    }

    /// apfmt run in `cwd` with `args`, reading `stdin`
    pub fn apfmt(&self, cwd: &Path, args: &[&str], stdin: &str) -> Output {
        let mut command = self.command(env!("CARGO_BIN_EXE_apfmt"), cwd);
        command.args(args);
        output_with_stdin(&mut command, stdin)
    }

    /// The output of apfmt formatting the file at `path` (run in the test directory)
    pub fn format_file(&self, path: &Path, args: &[&str]) -> String {
        let mut all_args = args.to_vec();
        all_args.push(path.to_str().unwrap());
        success(self.apfmt(&self.root, &all_args, ""))
    }

    /// `cargo apfmt <args>` run in `dir` (relative to the root), as cargo runs it
    pub fn cargo_apfmt(&self, dir: &str, args: &[&str]) -> Output {
        self.command(env!("CARGO_BIN_EXE_cargo-apfmt"), &self.path(dir))
            .arg("apfmt")
            .args(args)
            .env("CARGO", env!("CARGO"))
            .output()
            .unwrap()
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// The output of `command` reading `stdin`
pub fn output_with_stdin(command: &mut Command, stdin: &str) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The command may exit without reading its input (on an error before it), closing the pipe
    let written = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    if let Err(error) = written {
        assert_eq!(error.kind(), ErrorKind::BrokenPipe, "{error}");
    }
    child.wait_with_output().unwrap()
}

/// The stdout of a successful command
pub fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "the command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
