//! The config of one source file: its config file (given or looked up) and the edition of its package

use std::path::{Path, PathBuf};

use ra_ap_syntax::Edition;

use crate::Error::GenericError;
use crate::config::Config;
use crate::config::cargo::{cargo_edition, find_cargo_manifest};
use crate::config::lookup::{config_in_dir, find_config};
use crate::config::parser::read_config;

/// Where the config of a source comes from
#[derive(Clone, Debug, Default)]
pub struct ConfigOptions {
    /// The config file to use, or a directory with one, instead of looking it up
    pub config_path: Option<PathBuf>,
    /// Read no config file, nor the edition from `Cargo.toml`
    pub no_config: bool,
    /// Ignore rustfmt's config files (`rustfmt.toml`, `.rustfmt.toml`)
    pub no_rustfmt_toml: bool,
    /// The edition, overriding the one of `Cargo.toml` and of the config file; `Cargo.toml` is
    /// then not read
    pub edition: Option<Edition>,
}

/// The config for a source in `dir`: the config file (`ConfigOptions::config_path`, or the one
/// `find_config` finds from `dir`) applied to the defaults, and the edition: the explicit one
/// (`ConfigOptions::edition`), else the one of the package in the nearest `Cargo.toml` (as
/// `cargo fmt` passes it), else the one of the config file
pub fn config_for_source(dir: &Path, options: &ConfigOptions) -> crate::Result<Config> {
    let mut config = Config::default();
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
        && let Some(edition) = cargo_edition(&manifest)
            .map_err(|error| GenericError(format!("cannot read the edition: {error}")))?
    {
        config.edition = edition;
    }
    Ok(config)
}

/// The config file for a source in `dir`; an explicitly given one that does not exist is an error
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
