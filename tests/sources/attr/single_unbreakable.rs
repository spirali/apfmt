// A single argument that cannot break stays on one line within `max_width`, beyond
// `attr_fn_like_width`, as a single call argument does
#[error("Generic must have one and only bound (either <T> and 'where T: Trait', or <T: Trait>)")]
fn string() {}
#[error("Generic must have one and only bound (either <T> and 'where T: Trait', or <T: Trait>)",)]
fn string_comma() {}
#[error(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa)]
fn path() {}
#[deprecated(note = "use the other function instead, this one will be removed in the next release")]
fn key_value() {}
#[cfg(feature = "a_very_long_feature_name_that_goes_beyond_seventy_columns_xxxxxxxxxx")]
fn cfg() {}
#[cfg(not(feature = "a_very_long_feature_name_that_goes_beyond_seventy_columns_xxxxx"))]
fn nested() {}

enum E {
    #[error("Where clause predicate (eg: where T: Trait) must appear in generics list (eg: <T>)")]
    A(Span),
}
