use std::path::Path;
use std::str::FromStr;

use ra_ap_syntax::Edition;
use serde::Deserialize;
use toml::{Table, Value};

use crate::Error::GenericError;
use crate::config::Config;

/// Applies the config file at `path` to `cfg`. The format is told by the name: in a rustfmt config
/// file (a name ending with `rustfmt.toml`), the options that afmt does not have are ignored; in an
/// afmt one, they are an error.
pub(crate) fn read_config(path: &Path, cfg: &mut Config) -> crate::Result<()> {
    let toml = TomlConfig::load(path)?;
    if !is_rustfmt_config(path) {
        toml.check_unknown()?;
    }
    toml.apply(cfg);
    Ok(())
}

fn is_rustfmt_config(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with("rustfmt.toml"))
}

/// The options of a config file
#[derive(Default, Deserialize)]
#[serde(default)]
struct TomlConfig {
    max_width: Option<u32>,
    use_small_heuristics: Option<Heuristics>,
    fn_call_width: Option<u32>,
    attr_fn_like_width: Option<u32>,
    struct_lit_width: Option<u32>,
    struct_variant_width: Option<u32>,
    array_width: Option<u32>,
    chain_width: Option<u32>,
    single_line_if_else_max_width: Option<u32>,
    single_line_let_else_max_width: Option<u32>,
    tab_spaces: Option<u32>,
    blank_lines_upper_bound: Option<u32>,
    short_array_element_width_threshold: Option<u32>,
    trailing_semicolon: Option<bool>,
    edition: Option<EditionSetting>,
    disable_all_formatting: Option<bool>,
    // afmt's own options, not in rustfmt (or not in rustfmt's meaning)
    trailing_comment_align_threshold: Option<u32>,
    normalize_comment_sigil_spacing: Option<bool>,
    /// The options that afmt does not have
    #[serde(flatten)]
    unknown: Table,
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

impl TomlConfig {
    fn load(path: &Path) -> crate::Result<TomlConfig> {
        let toml = std::fs::read_to_string(path)?;
        TomlConfig::parse(&toml)
    }

    fn parse(toml: &str) -> crate::Result<TomlConfig> {
        // The caller names the file
        let cfg: TomlConfig =
            toml::from_str(toml).map_err(|e| GenericError(e.to_string().trim_end().to_string()))?;
        if cfg.max_width == Some(0) {
            return Err(GenericError(
                "`max_width` must be greater than 0".to_string(),
            ));
        }
        Ok(cfg)
    }

    /// Fails when the file has options that afmt does not have
    fn check_unknown(&self) -> crate::Result<()> {
        if self.unknown.is_empty() {
            return Ok(());
        }
        let names: Vec<_> = self
            .unknown
            .keys()
            .map(|name| format!("`{name}`"))
            .collect();
        Err(GenericError(format!(
            "unknown options: {}",
            names.join(", ")
        )))
    }

    fn apply(&self, config: &mut Config) {
        set_value(&mut config.tab_spaces, self.tab_spaces);
        set_value(
            &mut config.blank_lines_upper_bound,
            self.blank_lines_upper_bound,
        );
        set_value(
            &mut config.short_array_element_width_threshold,
            self.short_array_element_width_threshold,
        );
        set_value(&mut config.trailing_semicolon, self.trailing_semicolon);
        set_value(
            &mut config.trailing_comment_align_threshold,
            self.trailing_comment_align_threshold,
        );
        set_value(
            &mut config.normalize_comment_sigil_spacing,
            self.normalize_comment_sigil_spacing,
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
            h.apply(&mut config.single_line_let_else_max_width, 50, w, false);
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
        set_value_bound(
            &mut config.single_line_let_else_max_width,
            self.single_line_let_else_max_width,
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
    use crate::test_utils::TempDir;

    fn apply(toml: &str) -> Config {
        let mut config = Config::default();
        TomlConfig::parse(toml).unwrap().apply(&mut config);
        config
    }

    #[test]
    fn rustfmt_options() {
        let config = apply(
            "max_width = 80\nuse_small_heuristics = \"Max\"\nchain_width = 40\ntab_spaces = 2\n\
             edition = \"2021\"\n",
        );
        assert_eq!(config.max_width, 80);
        assert_eq!(config.fn_call_width, 80);
        assert_eq!(config.chain_width, 40);
        assert_eq!(config.tab_spaces, 2);
        assert_eq!(config.edition, Edition::Edition2021);
    }

    #[test]
    fn afmt_options() {
        let config = apply(
            "trailing_comment_align_threshold = 0\nnormalize_comment_sigil_spacing = false\n",
        );
        assert_eq!(config.trailing_comment_align_threshold, 0);
        assert!(!config.normalize_comment_sigil_spacing);
    }

    #[test]
    fn unknown_option_is_an_error() {
        for toml in [
            "max_widht = 80",
            "hard_tabs = true",
            "dump_parse_tree = true",
        ] {
            let config = TomlConfig::parse(toml).unwrap();
            let Err(GenericError(message)) = config.check_unknown() else {
                panic!("`{toml}` is accepted");
            };
            let name = toml.split(' ').next().unwrap();
            assert!(message.contains(&format!("`{name}`")), "{message}");
        }
        assert!(
            TomlConfig::parse("max_width = 80")
                .unwrap()
                .check_unknown()
                .is_ok()
        );
    }

    #[test]
    fn unknown_options_are_ignored_in_rustfmt_config_only() {
        let dir = TempDir::new("read-config", &[]);
        let toml = "hard_tabs = true\nmax_width = 80\n";
        for (name, ok) in [
            ("rustfmt.toml", true),
            (".rustfmt.toml", true),
            ("test.rustfmt.toml", true),
            ("afmt.toml", false),
            (".afmt.toml", false),
            ("other.toml", false),
        ] {
            let path = dir.join(name);
            std::fs::write(&path, toml).unwrap();
            let mut config = Config::default();
            assert_eq!(read_config(&path, &mut config).is_ok(), ok, "{name}");
            if ok {
                assert_eq!(config.max_width, 80);
            }
        }
    }

    #[test]
    fn invalid_values_are_errors() {
        for toml in [
            "max_width = 0",
            "max_width = -1",
            "normalize_comment_sigil_spacing = 1",
            "edition = \"2020\"",
        ] {
            assert!(TomlConfig::parse(toml).is_err(), "`{toml}` is accepted");
        }
    }

    fn widths(config: &Config) -> [u32; 8] {
        [
            config.fn_call_width,
            config.attr_fn_like_width,
            config.struct_lit_width,
            config.struct_variant_width,
            config.array_width,
            config.chain_width,
            config.single_line_if_else_max_width,
            config.single_line_let_else_max_width,
        ]
    }

    #[test]
    fn defaults_are_rustfmt_defaults() {
        let config = apply("");
        assert_eq!(config.max_width, 100);
        assert_eq!(widths(&config), [60, 70, 18, 35, 60, 60, 50, 50]);
        assert_eq!(config.edition, Config::default().edition);
        assert!(!config.disable_all_formatting);
    }

    #[test]
    fn simple_options() {
        let config = apply(
            "tab_spaces = 2\nblank_lines_upper_bound = 3\nshort_array_element_width_threshold = 0\n\
             edition = \"2018\"\ndisable_all_formatting = true\ntrailing_semicolon = false\n",
        );
        assert_eq!(config.tab_spaces, 2);
        assert_eq!(config.blank_lines_upper_bound, 3);
        assert_eq!(config.short_array_element_width_threshold, 0);
        assert_eq!(config.edition, Edition::Edition2018);
        assert!(config.disable_all_formatting);
        assert!(!config.trailing_semicolon);
    }

    #[test]
    fn edition_as_integer() {
        assert_eq!(apply("edition = 2021").edition, Edition::Edition2021);
    }

    #[test]
    fn heuristics_are_scaled_up_from_max_width() {
        // the ratio 1.55 is rounded to 1.6
        let config = apply("max_width = 155");
        assert_eq!(widths(&config), [96, 112, 29, 56, 96, 96, 80, 80]);
    }

    #[test]
    fn heuristics_are_not_scaled_down() {
        let config = apply("max_width = 50");
        assert_eq!(config.max_width, 50);
        assert_eq!(widths(&config), [60, 70, 18, 35, 60, 60, 50, 50]);
    }

    #[test]
    fn heuristics_max_and_off() {
        let config = apply("max_width = 80\nuse_small_heuristics = \"Max\"");
        assert_eq!(widths(&config), [80; 8]);
        // the case does not matter
        let config = apply("max_width = 80\nuse_small_heuristics = \"off\"");
        assert_eq!(widths(&config), [80, 80, 0, 0, 80, 80, 0, 0]);
    }

    #[test]
    fn explicit_width_overrides_heuristics() {
        let config = apply("fn_call_width = 30\nuse_small_heuristics = \"Max\"");
        assert_eq!(widths(&config), [30, 100, 100, 100, 100, 100, 100, 100]);
    }

    #[test]
    fn explicit_width_is_capped_at_max_width() {
        let config = apply("chain_width = 90\nmax_width = 80");
        assert_eq!(config.chain_width, 80);
    }

    #[test]
    fn malformed_file_changes_nothing() {
        assert!(TomlConfig::parse("max_width = 80\nthis is not toml").is_err());
    }
}
