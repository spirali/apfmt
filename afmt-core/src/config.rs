pub struct Config {
    pub max_width: u32,
    pub indent_width: u32,
    pub blank_lines_upper_bound: u32,
    pub dump_tree: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_width: 100,
            indent_width: 4,
            blank_lines_upper_bound: 1,
            dump_tree: false,
        }
    }
}
