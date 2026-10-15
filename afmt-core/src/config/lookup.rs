//! Finding the config file of a source: `afmt.toml` or `rustfmt.toml`, the way rustfmt finds its own

use std::path::{Path, PathBuf};

pub fn config_in_dir(dir: &Path, afmt: bool, rustfmt: bool) -> Option<PathBuf> {
    let mut r = if afmt {
        file_in_dir(dir, &[".afmt.toml", "afmt.toml"])
    } else {
        None
    };

    if r.is_none() && rustfmt {
        r = file_in_dir(dir, &[".rustfmt.toml", "rustfmt.toml"])
    }
    r
}

fn file_in_dir(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

/// The config file for a source in `dir`: the nearest one in `dir` or its ancestors, otherwise the one
/// in the home directory, otherwise the one in the user's configuration directory (`afmt/`, then
/// `rustfmt/`, in `$XDG_CONFIG_HOME` or `~/.config` on Linux). In one directory, an afmt config
/// file is preferred to a rustfmt one.
pub fn find_config(dir: &Path, include_rustfmt: bool) -> Option<PathBuf> {
    let dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let dir = dir.canonicalize().unwrap_or(dir);
    dir.ancestors()
        .find_map(|dir| config_in_dir(dir, true, include_rustfmt))
        .or_else(|| std::env::home_dir().and_then(|home| config_in_dir(&home, true, include_rustfmt)))
        .or_else(|| {
            let config_dir = user_config_dir()?;
            config_in_dir(&config_dir.join("afmt"), true, false).or_else(|| if include_rustfmt {
                config_in_dir(&config_dir.join("rustfmt"), false, true)
            } else { None })
        })
}

/// The user's configuration directory, as the `dirs` crate used by rustfmt defines it
fn user_config_dir() -> Option<PathBuf> {
    let env_dir = |name: &str| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    if cfg!(windows) {
        env_dir("APPDATA")
    } else if cfg!(target_os = "macos") {
        std::env::home_dir().map(|home| home.join("Library/Application Support"))
    } else {
        env_dir("XDG_CONFIG_HOME").or_else(|| std::env::home_dir().map(|home| home.join(".config")))
    }
}