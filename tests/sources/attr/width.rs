// `attr_fn_like_width` limits the items of an attribute list on a single line; a single item that is
// a list itself is not measured, its own items are: `cfg(all(...))` is on a single line whenever
// `all(...)` is
#[cfg(all(feature = "coin_cbc", not(feature = "microlp"), not(feature = "highs")))]
pub(crate) mod coin_cbc;

#[cfg(all(feature = "coin_cbc", not(feature = "microlp"), not(feature = "highs"), unix))]
pub(crate) mod coin_cbc_unix;

#[cfg(any(feature = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaa", feature = "bbbbbbbbbbbbbbbbbbbbbbbbbbb"))]
fn any_long() {}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, Debug, Clone, PartialEq))]
struct CfgAttr;

#[clap(about, version(crate::HQ_VERSION), disable_help_subcommand(true), help_expected(true))]
struct Clap;

#[serde(rename_all = "kebab-case", tag = "type", content = "content", deny_unknown_fields)]
struct Serde;

#[serde(rename_all = "kebab-case", tag = "type", content = "content")]
struct SerdeShort;

// Derives are on a single line whenever it fits within `max_width`
#[derive(Default, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskId;

fn nested() {
    {
        {
            #[cfg(all(feature = "coin_cbc", not(feature = "microlp"), not(feature = "highs")))]
            let x = 1;
        }
    }
}

struct Fields {
    #[arg(long, value_delimiter(','), value_enum, default_value("finished,failed"))]
    pub filter: Vec<Status>,
    #[arg(long, value_enum)]
    pub kind: Kind,
}
