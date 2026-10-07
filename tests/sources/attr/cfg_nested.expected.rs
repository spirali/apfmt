#[cfg(all(
    target_os = "linux",
    not(any(feature = "aaaaaaaaaaa", feature = "bbbbbbbbbbbbbbbbbbbbbbbbb")),
))]
fn f() {}

#[cfg(any(
    feature = "aaaaaaaaaaaaaaaaaaaaaaaa",
    feature = "bbbbbbbbbbbbbbbbbbbbbbb",
    feature = "c",
))]
fn g() {}

#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash),
)]
struct S;
