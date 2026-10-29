//! The packages and targets of a Cargo workspace, from `cargo metadata`, as `cargo fmt` finds them

use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use afmt_core::{CARGO_MANIFEST_NAME, Edition, canonical_path};
use serde::Deserialize;

/// The part of `cargo metadata --format-version 1` that afmt needs
#[derive(Debug, Deserialize)]
pub struct Metadata {
    pub packages: Vec<Package>,
    pub workspace_root: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub manifest_path: PathBuf,
    pub targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub name: String,
    /// `lib`, `bin`, ...
    pub kind: Vec<String>,
    /// The root file of the target's module tree
    pub src_path: PathBuf,
    /// The edition of the target (`[lib] edition`, ...), which is the package's unless overridden
    pub edition: String,
}

impl Target {
    pub fn edition(&self) -> Result<Edition, String> {
        Edition::from_str(&self.edition)
            .map_err(|error| format!("{}: unsupported edition: {error}", self.src_path.display()))
    }
}

pub fn metadata(manifest_path: Option<&Path>) -> Result<Metadata, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command.args(["metadata", "--no-deps", "--format-version", "1"]);
    if let Some(manifest_path) = manifest_path {
        command.arg("--manifest-path").arg(manifest_path);
    }
    let output = command
        .output()
        .map_err(|error| format!("cannot run `cargo metadata`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "`cargo metadata` failed:\n{}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cannot read the output of `cargo metadata`: {error}"))
}

/// Which packages of the workspace to format
#[derive(Debug)]
pub enum Selection<'a> {
    /// All packages of the workspace (`--all`)
    All,
    /// The packages with these names (`-p`)
    Named(&'a [String]),
    /// As `cargo fmt` without options: all packages in the workspace root, otherwise the package of
    /// `manifest`
    Current { manifest: Option<PathBuf> },
}

pub fn select_packages<'a>(
    metadata: &'a Metadata,
    selection: &Selection,
) -> Result<Vec<&'a Package>, String> {
    match selection {
        Selection::All => Ok(metadata.packages.iter().collect()),
        Selection::Named(names) => names
            .iter()
            .map(|name| {
                metadata
                    .packages
                    .iter()
                    .find(|package| &package.name == name)
                    .ok_or_else(|| format!("package `{name}` is not in the workspace"))
            })
            .collect(),
        Selection::Current { manifest } => {
            if metadata.packages.len() == 1 {
                return Ok(metadata.packages.iter().collect());
            }
            let root_manifest = canonical_path(&metadata.workspace_root.join(CARGO_MANIFEST_NAME));
            let manifest = manifest.as_deref().map(canonical_path);
            if manifest.is_none() || manifest.as_ref() == Some(&root_manifest) {
                return Ok(metadata.packages.iter().collect());
            }
            Ok(metadata
                .packages
                .iter()
                .filter(|package| Some(canonical_path(&package.manifest_path)) == manifest)
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> Metadata {
        serde_json::from_str(
            r#"{
                "workspace_root": "/ws",
                "packages": [
                    {"name": "a", "version": "0.1.0", "manifest_path": "/ws/a/Cargo.toml", "targets": [
                        {"name": "a", "kind": ["lib"], "src_path": "/ws/a/src/lib.rs", "edition": "2021"}
                    ], "other": 1},
                    {"name": "b", "version": "0.1.0", "manifest_path": "/ws/b/Cargo.toml", "targets": [
                        {"name": "b", "kind": ["bin"], "src_path": "/ws/b/src/main.rs", "edition": "2024"}
                    ]}
                ]
            }"#,
        )
        .unwrap()
    }

    fn names(packages: &[&Package]) -> Vec<String> {
        packages.iter().map(|p| p.name.clone()).collect()
    }

    #[test]
    fn selection() {
        let metadata = metadata();
        let select = |selection| names(&select_packages(&metadata, &selection).unwrap());
        assert_eq!(select(Selection::All), ["a", "b"]);
        let b = ["b".to_string()];
        assert_eq!(select(Selection::Named(&b)), ["b"]);
        let in_root = Some(PathBuf::from("/ws/Cargo.toml"));
        assert_eq!(select(Selection::Current { manifest: in_root }), ["a", "b"]);
        let in_a = Some(PathBuf::from("/ws/a/Cargo.toml"));
        assert_eq!(select(Selection::Current { manifest: in_a }), ["a"]);
        let missing = ["c".to_string()];
        assert!(select_packages(&metadata, &Selection::Named(&missing)).is_err());
    }

    #[test]
    fn target_edition() {
        let metadata = metadata();
        let edition = metadata.packages[0].targets[0].edition().unwrap();
        assert_eq!(edition, Edition::Edition2021);
    }
}
