//! The edition of a source from the `Cargo.toml` of its package, as `cargo fmt` passes it to rustfmt

use std::path::{Path, PathBuf};
use std::str::FromStr;

use ra_ap_syntax::Edition;
use serde::Deserialize;

use crate::Error::GenericError;

pub const CARGO_MANIFEST_NAME: &str = "Cargo.toml";

/// The edition when `[package]` does not set one, as in cargo
const CARGO_DEFAULT_EDITION: Edition = Edition::Edition2015;

#[derive(Deserialize)]
struct Manifest {
    package: Option<Package>,
    workspace: Option<Workspace>,
}

#[derive(Deserialize)]
struct Package {
    edition: Option<Inheritable>,
    /// The path to the workspace root, when it is not an ancestor directory
    workspace: Option<PathBuf>,
}

/// A value of `[package]` given directly or inherited: `edition.workspace = true`
#[derive(Deserialize)]
#[serde(untagged)]
enum Inheritable {
    Value(String),
    Inherited { workspace: bool },
}

#[derive(Deserialize)]
struct Workspace {
    package: Option<WorkspacePackage>,
}

#[derive(Deserialize)]
struct WorkspacePackage {
    edition: Option<String>,
}

/// The nearest `Cargo.toml` in `dir` or its ancestors: the manifest of the package of the sources in
/// `dir`, as cargo finds it
pub fn find_cargo_manifest(dir: &Path) -> Option<PathBuf> {
    let dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let dir = dir.canonicalize().unwrap_or(dir);
    dir.ancestors()
        .map(|dir| dir.join(CARGO_MANIFEST_NAME))
        .find(|path| path.is_file())
}

/// The edition of the package of the manifest at `path`; `None` for a virtual workspace (a manifest
/// without `[package]`)
pub fn cargo_edition(path: &Path) -> crate::Result<Option<Edition>> {
    let manifest = read_manifest(path)?;
    let Some(package) = &manifest.package else {
        return Ok(None);
    };
    let edition = match &package.edition {
        None => return Ok(Some(CARGO_DEFAULT_EDITION)),
        Some(Inheritable::Value(edition)) => edition.clone(),
        Some(Inheritable::Inherited { workspace: false }) => {
            return Err(manifest_error(path, "`edition.workspace` must be `true`"));
        }
        Some(Inheritable::Inherited { workspace: true }) => {
            let root = workspace_root(path, &manifest, package)?;
            let root_manifest = if root == path {
                manifest
            } else {
                read_manifest(&root)?
            };
            root_manifest
                .workspace
                .and_then(|workspace| workspace.package)
                .and_then(|package| package.edition)
                .ok_or_else(|| {
                    manifest_error(&root, "`[workspace.package]` has no `edition` to inherit")
                })?
        }
    };
    Edition::from_str(&edition)
        .map(Some)
        .map_err(|error| manifest_error(path, &error.to_string()))
}

fn read_manifest(path: &Path) -> crate::Result<Manifest> {
    let toml = std::fs::read_to_string(path)?;
    toml::from_str(&toml).map_err(|error| manifest_error(path, error.to_string().trim_end()))
}

fn manifest_error(path: &Path, message: &str) -> crate::Error {
    GenericError(format!("{}: {message}", path.display()))
}

fn workspace_root(path: &Path, manifest: &Manifest, package: &Package) -> crate::Result<PathBuf> {
    let package_dir = path.parent().unwrap_or(Path::new("."));
    if let Some(root) = &package.workspace {
        return Ok(package_dir.join(root).join(CARGO_MANIFEST_NAME));
    }
    if manifest.workspace.is_some() {
        return Ok(path.to_path_buf());
    }
    for dir in package_dir.ancestors().skip(1) {
        let candidate = dir.join(CARGO_MANIFEST_NAME);
        if candidate.is_file() && read_manifest(&candidate)?.workspace.is_some() {
            return Ok(candidate);
        }
    }
    Err(manifest_error(
        path,
        "`edition.workspace = true` but no workspace root found",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory with the files `(path, content)`
    fn create_dir(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("afmt-cargo-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, content) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        root
    }

    fn edition_of(root: &Path, manifest: &str) -> crate::Result<Option<Edition>> {
        cargo_edition(&root.join(manifest))
    }

    #[test]
    fn package_edition() {
        let root = create_dir(
            "package",
            &[(
                "Cargo.toml",
                "[package]\nname = \"a\"\nedition = \"2021\"\n",
            )],
        );
        assert_eq!(
            edition_of(&root, "Cargo.toml").unwrap(),
            Some(Edition::Edition2021)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_edition_is_2015() {
        let root = create_dir("default", &[("Cargo.toml", "[package]\nname = \"a\"\n")]);
        assert_eq!(
            edition_of(&root, "Cargo.toml").unwrap(),
            Some(Edition::Edition2015)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn virtual_workspace_has_no_edition() {
        let root = create_dir(
            "virtual",
            &[(
                "Cargo.toml",
                "[workspace]\nmembers = [\"a\"]\n[workspace.package]\nedition = \"2018\"\n",
            )],
        );
        assert_eq!(edition_of(&root, "Cargo.toml").unwrap(), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inherited_edition() {
        let root = create_dir(
            "inherited",
            &[
                (
                    "Cargo.toml",
                    "[workspace]\nmembers = [\"crates/*\"]\n[workspace.package]\nedition = \"2018\"\n",
                ),
                (
                    "crates/a/Cargo.toml",
                    "[package]\nname = \"a\"\nedition.workspace = true\n",
                ),
                (
                    "elsewhere/b/Cargo.toml",
                    "[package]\nname = \"b\"\nworkspace = \"../../root\"\n\
                     edition = { workspace = true }\n",
                ),
                (
                    "root/Cargo.toml",
                    "[workspace]\n[workspace.package]\nedition = \"2021\"\n",
                ),
            ],
        );
        assert_eq!(
            edition_of(&root, "crates/a/Cargo.toml").unwrap(),
            Some(Edition::Edition2018)
        );
        assert_eq!(
            edition_of(&root, "elsewhere/b/Cargo.toml").unwrap(),
            Some(Edition::Edition2021)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_and_workspace_in_one_manifest() {
        let root = create_dir(
            "root-package",
            &[(
                "Cargo.toml",
                "[package]\nname = \"a\"\nedition.workspace = true\n\
                 [workspace]\n[workspace.package]\nedition = \"2024\"\n",
            )],
        );
        assert_eq!(
            edition_of(&root, "Cargo.toml").unwrap(),
            Some(Edition::Edition2024)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn errors() {
        let root = create_dir(
            "errors",
            &[
                ("bad-edition/Cargo.toml", "[package]\nedition = \"2020\"\n"),
                ("malformed/Cargo.toml", "[package\n"),
                (
                    "no-root/Cargo.toml",
                    "[package]\nedition.workspace = true\n",
                ),
            ],
        );
        for manifest in ["bad-edition", "malformed", "no-root"] {
            let error = edition_of(&root, &format!("{manifest}/Cargo.toml")).unwrap_err();
            let GenericError(message) = error else {
                panic!("{error:?}");
            };
            assert!(message.contains(manifest), "{message}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nearest_manifest() {
        let root = create_dir(
            "nearest",
            &[
                ("Cargo.toml", "[workspace]\n"),
                ("a/Cargo.toml", "[package]\n"),
                ("a/src/bin/x.rs", ""),
            ],
        );
        let found = find_cargo_manifest(&root.join("a/src/bin")).unwrap();
        assert_eq!(found, root.join("a/Cargo.toml").canonicalize().unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }
}
