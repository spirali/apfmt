use ra_ap_syntax::Edition;

pub struct Config {
    pub max_width: u32,
    pub indent_width: u32,
    pub blank_lines_upper_bound: u32,
    pub fn_call_width: u32,
    /// Maximal width of the fields of an enum struct variant put on a single line: `A { x: u32 }`
    pub struct_variant_width: u32,
    /// Maximal width of `if a { b } else { c }` put on a single line
    pub single_line_if_else_max_width: u32,
    /// Maximal width of an argument for which the arguments of a broken call are filled on lines
    /// (`a, b, c,` / `d, e,`) instead of one per line; every argument has to be simple (a literal, a name).
    /// 0 disables it.
    pub short_array_element_width_threshold: u32,
    /// Puts a single space inside single-line block comments: `/*a*/` -> `/* a */`
    pub normalize_comments: bool,
    pub edition: Edition,
    pub dump_tree: bool,
    pub check_result: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_width: 100,
            indent_width: 4,
            blank_lines_upper_bound: 1,
            fn_call_width: 60,
            struct_variant_width: 35,
            single_line_if_else_max_width: 50,
            short_array_element_width_threshold: 10,
            normalize_comments: true,
            edition: Edition::CURRENT,
            dump_tree: false,
            check_result: false,
        }
    }
}
