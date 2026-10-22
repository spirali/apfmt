// Blank lines between the fields of a struct literal are removed, as between call arguments: within
// an expression only statements keep them (rustfmt keeps them in struct literals)
fn f() -> ParquetResult<Self> {
    let s = Decoder {
        prefix_lengths: delta_bitpacked::Decoder::new(values),
        suffix_lengths: delta_bitpacked::Decoder::new(values),
        values,

        offset: 0,
        last: Vec::with_capacity(32),
    };
    let s = Settings {
        searchable_attributes: Setting::Set(vec!["title".to_string()]),


        filterable_attributes: Setting::Set(vec!["genres".to_string()]),

        ..Default::default()
    };
    // A comment after the blank line stays in front of its field
    let s = Config {
        deadline: Instant::now() + Duration::from_secs(10),

        // only in tests
        stop_after: None,
    };
    let s = S {
        a: 1,

        b: 2,
    };
}
