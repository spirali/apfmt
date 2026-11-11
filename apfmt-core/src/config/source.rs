//! The config of one source file: its config file (given or looked up) and the edition of its package

use std::path::{Path, PathBuf};

use ra_ap_syntax::Edition;

use crate::Error::GenericError;
use crate::config::Config;
use crate::config::cargo::{cargo_edition, find_cargo_manifest};
use crate::config::lookup::{config_in_dir, find_config};
use crate::config::parser::read_config;

#[derive(Clone, Debug, Default)]
pub struct ConfigOptions {
    /// The config file to use, or a directory with one, instead of looking it up
    pub config_path: Option<PathBuf>,
    /// Read no config file, nor the edition from `Cargo.toml`
    pub no_config: bool,
    /// Ignore rustfmt's config files (`rustfmt.toml`, `.rustfmt.toml`)
    pub no_rustfmt_toml: bool,
    /// Overrides `Cargo.toml` and the config file; `Cargo.toml` is then not read
    pub edition: Option<Edition>,
}

/// The config file applied to the defaults, and the edition: explicit, else from `Cargo.toml`, else
/// from the config file. An unreadable `Cargo.toml` is a returned warning.
pub fn config_for_source(
    dir: &Path,
    options: &ConfigOptions,
) -> crate::Result<(Config, Vec<String>)> {
    let mut config = Config::default();
    let mut warnings = Vec::new();
    if !options.no_config
        && let Some(file) = config_file(dir, options)?
    {
        read_config(&file, &mut config).map_err(|error| {
            GenericError(format!(
                "cannot read the config file {}: {error}",
                file.display()
            ))
        })?;
    }
    if let Some(edition) = options.edition {
        config.edition = edition;
    } else if !options.no_config
        && let Some(manifest) = find_cargo_manifest(dir)
    {
        match cargo_edition(&manifest) {
            Ok(Some(edition)) => config.edition = edition,
            Ok(None) => {}
            Err(error) => warnings.push(format!(
                "using edition {}, cannot read the edition of the package: {error}",
                config.edition
            )),
        }
    }
    Ok((config, warnings))
}

/// An explicitly given one that does not exist is an error
fn config_file(dir: &Path, options: &ConfigOptions) -> crate::Result<Option<PathBuf>> {
    let include_rustfmt = !options.no_rustfmt_toml;
    let Some(path) = &options.config_path else {
        return Ok(find_config(dir, include_rustfmt));
    };
    let file = if path.is_dir() {
        config_in_dir(path, true, include_rustfmt)
    } else {
        path.exists().then(|| path.clone())
    };
    match file {
        Some(file) => Ok(Some(file)),
        None => Err(GenericError(format!(
            "no config file found at {}",
            path.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TempDir;

    /// In a package directory with `apfmt.toml`, so no config file outside it is found, and
    /// `Cargo.toml`
    fn config_in_package(
        name: &str,
        apfmt_toml: &str,
        cargo_toml: &str,
        edition: Option<Edition>,
    ) -> (Config, Vec<String>) {
        let root = TempDir::new(
            name,
            &[("apfmt.toml", apfmt_toml), ("Cargo.toml", cargo_toml)],
        );
        let options = ConfigOptions {
            edition,
            ..ConfigOptions::default()
        };
        config_for_source(&root, &options).unwrap()
    }

    #[test]
    fn edition_of_package() {
        let (config, warnings) = config_in_package(
            "package-edition",
            "edition = \"2018\"\n",
            "[package]\nname = \"a\"\nedition = \"2021\"\n",
            None,
        );
        assert_eq!(config.edition, Edition::Edition2021);
        assert!(warnings.is_empty());
    }

    #[test]
    fn unreadable_manifest_warns_and_uses_config_edition() {
        for (name, cargo_toml) in [
            (
                "invalid-edition",
                "[package]\nname = \"a\"\nedition = \"2023\"\n",
            ),
            (
                "toml-error",
                "[package]\nname = \"a\"\ninclude = [\"a\" \"b\"]\n",
            ),
        ] {
            let (config, warnings) =
                config_in_package(name, "edition = \"2018\"\n", cargo_toml, None);
            assert_eq!(config.edition, Edition::Edition2018, "{name}");
            assert_eq!(warnings.len(), 1, "{name}");
            assert!(
                warnings[0]
                    .starts_with("using edition 2018, cannot read the edition of the package: "),
                "{}",
                warnings[0]
            );
        }
    }

    #[test]
    fn explicit_edition_does_not_read_manifest() {
        let (config, warnings) = config_in_package(
            "explicit-edition",
            "",
            "[package]\nname = \"a\"\nedition = \"2023\"\n",
            Some(Edition::Edition2015),
        );
        assert_eq!(config.edition, Edition::Edition2015);
        assert!(warnings.is_empty());
    }

    #[test]
    fn invalid_config_file_is_an_error() {
        let root = TempDir::new(
            "invalid-config",
            &[("apfmt.toml", "max_width = \"wide\"\n")],
        );
        assert!(config_for_source(&root, &ConfigOptions::default()).is_err());
    }
}
