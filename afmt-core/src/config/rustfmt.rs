use std::path::{Path, PathBuf};
use std::str::FromStr;

use ra_ap_syntax::Edition;
use serde::Deserialize;
use toml::Value;

use crate::Error::GenericError;
use crate::config::Config;

pub const RUSTFMT_CONFIG_FILE_NAMES: [&str; 2] = [".rustfmt.toml", "rustfmt.toml"];

pub fn rustfmt_config_in_dir(dir: &Path) -> Option<PathBuf> {
    RUSTFMT_CONFIG_FILE_NAMES
        .iter()
        .map(|name| dir.join(name))
        // `is_file` is false for a directory named `rustfmt.toml` and for an unreadable path
        .find(|path| path.is_file())
}

pub fn find_rustfmt_config(dir: &Path) -> Option<PathBuf> {
    let dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let dir = dir.canonicalize().unwrap_or(dir);
    dir.ancestors()
        .find_map(rustfmt_config_in_dir)
        .or_else(|| std::env::home_dir().and_then(|home| rustfmt_config_in_dir(&home)))
        .or_else(|| user_config_dir().and_then(|dir| rustfmt_config_in_dir(&dir.join("rustfmt"))))
}

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

/// The options of a rustfmt config file that afmt has; a wrong value of one option does not make the
/// others fail (`Setting`)
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct RustfmtConfig {
    max_width: Option<u32>,
    use_small_heuristics: Option<Heuristics>,
    fn_call_width: Option<u32>,
    attr_fn_like_width: Option<u32>,
    struct_lit_width: Option<u32>,
    struct_variant_width: Option<u32>,
    array_width: Option<u32>,
    chain_width: Option<u32>,
    single_line_if_else_max_width: Option<u32>,
    tab_spaces: Option<u32>,
    blank_lines_upper_bound: Option<u32>,
    short_array_element_width_threshold: Option<u32>,
    edition: Option<EditionSetting>,
    disable_all_formatting: Option<bool>,
}

fn set_value<T: Copy>(out: &mut T, v: Option<T>) {
    if let Some(v) = v {
        *out = v;
    }
}

fn set_value_bound(out: &mut u32, v: Option<u32>, bound: u32) {
    if let Some(v) = v {
        *out = v.min(bound);
    }
}

impl RustfmtConfig {
    pub fn load(path: &Path) -> crate::Result<RustfmtConfig> {
        let toml = std::fs::read_to_string(path)?;
        RustfmtConfig::parse(&toml)
    }

    pub fn parse(toml: &str) -> crate::Result<RustfmtConfig> {
        let cfg: RustfmtConfig = toml::from_str(toml)
            .map_err(|e| GenericError(format!("Could not parse rustfmt config: {}", e)))?;
        if let Some(w) = cfg.max_width
            && w == 0
        {
            return Err(GenericError(
                "rustfmt config: Invalid max width".to_string(),
            ));
        }
        Ok(cfg)
    }

    pub fn apply(&self, config: &mut Config) {
        set_value(&mut config.tab_spaces, self.tab_spaces);
        set_value(
            &mut config.blank_lines_upper_bound,
            self.blank_lines_upper_bound,
        );
        set_value(
            &mut config.short_array_element_width_threshold,
            self.short_array_element_width_threshold,
        );
        set_value(&mut config.edition, self.edition.map(|e| e.0));
        set_value(
            &mut config.disable_all_formatting,
            self.disable_all_formatting,
        );
        set_value(&mut config.max_width, self.max_width);
        let w = config.max_width;
        if self.max_width.is_some() || self.use_small_heuristics.is_some() {
            let h = self.use_small_heuristics.unwrap_or(Heuristics::Default);
            h.apply(&mut config.fn_call_width, 60, w, true);
            h.apply(&mut config.attr_fn_like_width, 70, w, true);
            h.apply(&mut config.struct_lit_width, 18, w, false);
            h.apply(&mut config.struct_variant_width, 35, w, false);
            h.apply(&mut config.array_width, 60, w, true);
            h.apply(&mut config.chain_width, 60, w, true);
            h.apply(&mut config.single_line_if_else_max_width, 50, w, false);
        }

        set_value_bound(&mut config.fn_call_width, self.fn_call_width, w);
        set_value_bound(&mut config.attr_fn_like_width, self.attr_fn_like_width, w);
        set_value_bound(&mut config.struct_lit_width, self.struct_lit_width, w);
        set_value_bound(
            &mut config.struct_variant_width,
            self.struct_variant_width,
            w,
        );
        set_value_bound(&mut config.array_width, self.array_width, w);
        set_value_bound(&mut config.chain_width, self.chain_width, w);
        set_value_bound(
            &mut config.single_line_if_else_max_width,
            self.single_line_if_else_max_width,
            w,
        );
    }
}

/// An edition as a string (as rustfmt requires) or, leniently, as an integer
#[derive(Deserialize, Clone, Copy)]
#[serde(try_from = "Value")]
struct EditionSetting(Edition);

impl TryFrom<Value> for EditionSetting {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let edition = match value {
            Value::String(s) => Edition::from_str(&s).ok(),
            Value::Integer(n) => Edition::from_str(&n.to_string()).ok(),
            _ => None,
        };
        edition
            .map(EditionSetting)
            .ok_or(r#"expected "2015", "2018", "2021" or "2024""#)
    }
}

/// How rustfmt derives the width heuristics from `max_width` (its `use_small_heuristics`)
#[derive(Clone, Copy, Deserialize)]
#[serde(try_from = "String")]
enum Heuristics {
    /// Scaled from the defaults for `max_width = 100`
    Default,
    /// All of them are `max_width`
    Max,
    /// No single-line limits: the unlimited ones are `max_width`, the others 0
    Off,
}

impl TryFrom<String> for Heuristics {
    type Error = &'static str;

    /// Ignoring the case like rustfmt does
    fn try_from(s: String) -> Result<Self, Self::Error> {
        [
            ("Default", Heuristics::Default),
            ("Max", Heuristics::Max),
            ("Off", Heuristics::Off),
        ]
        .into_iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(&s))
        .map(|(_, heuristics)| heuristics)
        .ok_or(r#"expected "Default", "Max" or "Off""#)
    }
}

impl Heuristics {
    fn apply(&self, out: &mut u32, default: u32, max_width: u32, unlimited_when_off: bool) {
        *out = match self {
            Heuristics::Default => {
                let ratio = if max_width > 100 {
                    (max_width as f32 / 100.0 * 10.0).round() / 10.0
                } else {
                    1.0
                };
                (default as f32 * ratio).round() as u32
            }
            Heuristics::Max => max_width,
            Heuristics::Off if unlimited_when_off => max_width,
            Heuristics::Off => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(toml: &str) -> Config {
        let mut config = Config::default();
        RustfmtConfig::parse(toml).unwrap().apply(&mut config);
        config
    }

    fn widths(config: &Config) -> [u32; 7] {
        [
            config.fn_call_width,
            config.attr_fn_like_width,
            config.struct_lit_width,
            config.struct_variant_width,
            config.array_width,
            config.chain_width,
            config.single_line_if_else_max_width,
        ]
    }

    #[test]
    fn defaults_are_rustfmt_defaults() {
        let config = apply("");
        assert_eq!(config.max_width, 100);
        assert_eq!(widths(&config), [60, 70, 18, 35, 60, 60, 50]);
        assert_eq!(config.edition, Config::default().edition);
        assert!(!config.disable_all_formatting);
    }

    #[test]
    fn simple_options() {
        let config = apply(
            "tab_spaces = 2\nblank_lines_upper_bound = 3\nshort_array_element_width_threshold = 0\n\
             edition = \"2018\"\ndisable_all_formatting = true\n",
        );
        assert_eq!(config.tab_spaces, 2);
        assert_eq!(config.blank_lines_upper_bound, 3);
        assert_eq!(config.short_array_element_width_threshold, 0);
        assert_eq!(config.edition, Edition::Edition2018);
        assert!(config.disable_all_formatting);
    }

    #[test]
    fn edition_as_integer() {
        assert_eq!(apply("edition = 2021").edition, Edition::Edition2021);
    }

    #[test]
    fn heuristics_are_scaled_up_from_max_width() {
        // the ratio 1.55 is rounded to 1.6
        let config = apply("max_width = 155");
        assert_eq!(widths(&config), [96, 112, 29, 56, 96, 96, 80]);
    }

    #[test]
    fn heuristics_are_not_scaled_down() {
        let config = apply("max_width = 50");
        assert_eq!(config.max_width, 50);
        assert_eq!(widths(&config), [60, 70, 18, 35, 60, 60, 50]);
    }

    #[test]
    fn heuristics_max_and_off() {
        let config = apply("max_width = 80\nuse_small_heuristics = \"Max\"");
        assert_eq!(widths(&config), [80; 7]);
        // the case does not matter
        let config = apply("max_width = 80\nuse_small_heuristics = \"off\"");
        assert_eq!(widths(&config), [80, 80, 0, 0, 80, 80, 0]);
    }

    #[test]
    fn explicit_width_overrides_heuristics() {
        let config = apply("fn_call_width = 30\nuse_small_heuristics = \"Max\"");
        assert_eq!(widths(&config), [30, 100, 100, 100, 100, 100, 100]);
    }

    #[test]
    fn explicit_width_is_capped_at_max_width() {
        let config = apply("chain_width = 90\nmax_width = 80");
        assert_eq!(config.chain_width, 80);
    }

    #[test]
    fn unsupported_options_are_ignored() {
        let config =
            apply("hard_tabs = true\nstyle_edition = \"2024\"\nmax_width = 90\n[unknown]\nx = 1\n");
        assert_eq!(config.max_width, 90);
    }

    #[test]
    fn malformed_file_changes_nothing() {
        assert!(RustfmtConfig::parse("max_width = 80\nthis is not toml").is_err());
    }
}
