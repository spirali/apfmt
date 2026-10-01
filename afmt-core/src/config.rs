pub struct Config {
    pub max_width: u32,
    pub indent_width: u32,
    pub blank_lines_upper_bound: u32,
    pub fn_call_width: u32,
    /// Puts a single space inside single-line block comments: `/*a*/` -> `/* a */`
    pub normalize_comments: bool,
    pub dump_tree: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_width: 100,
            indent_width: 4,
            blank_lines_upper_bound: 1,
            fn_call_width: 60,
            normalize_comments: true,
            dump_tree: false,
        }
    }
}
