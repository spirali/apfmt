pub(crate) mod cargo;
pub(crate) mod lookup;
pub(crate) mod parser;
pub(crate) mod source;

use ra_ap_syntax::Edition;

pub struct Config {
    pub max_width: u32,
    pub tab_spaces: u32,
    pub blank_lines_upper_bound: u32,
    pub fn_call_width: u32,
    /// Maximal width of a chain of fields and method calls put on a single line: `a.b().c()`
    pub chain_width: u32,
    /// Maximal width of the elements of an array literal put on a single line
    pub array_width: u32,
    /// Maximal width of the fields of a struct literal or a struct pattern put on a single line:
    /// `S { a, b }`
    pub struct_lit_width: u32,
    /// Maximal width of the arguments of a function-like attribute put on a single line:
    /// `#[derive(A, B)]`
    pub attr_fn_like_width: u32,
    /// Maximal width of the fields of an enum struct variant put on a single line: `A { x: u32 }`
    pub struct_variant_width: u32,
    /// Maximal width of `if a { b } else { c }` put on a single line
    pub single_line_if_else_max_width: u32,
    /// Maximal width of an argument for which the arguments of a broken call are filled on lines
    /// (`a, b, c,` / `d, e,`) instead of one per line; every argument has to be simple (a literal, a name).
    /// 0 disables it.
    pub short_array_element_width_threshold: u32,
    /// The gap after which the widest list items do not align their trailing comments; 0 disables it
    pub trailing_comment_align_threshold: u32,
    /// Puts a single space after the opening sigil of a comment and before the closing one:
    /// `//a` -> `// a`, `/*a*/` -> `/* a */` (see `normalize_comment_sigil_spacing`)
    pub normalize_comment_sigil_spacing: bool,
    /// The edition of the syntax
    pub edition: Edition,
    /// The source is not formatted at all (rustfmt's `disable_all_formatting`); the caller outputs
    /// it unchanged
    pub disable_all_formatting: bool,
    pub dump_parse_tree: bool,
    pub dump_doc_tree: bool,
    pub check_result: bool,
    /// Copies the syntax that afmt cannot format yet unchanged into the output and reports it in
    /// `Formatted::unsupported`; otherwise formatting fails with `Error::Unsupported`
    pub allow_unsupported: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_width: 100,
            tab_spaces: 4,
            blank_lines_upper_bound: 1,
            fn_call_width: 60,
            chain_width: 60,
            array_width: 60,
            struct_lit_width: 18,
            attr_fn_like_width: 70,
            struct_variant_width: 35,
            single_line_if_else_max_width: 50,
            short_array_element_width_threshold: 10,
            trailing_comment_align_threshold: 40,
            normalize_comment_sigil_spacing: true,
            edition: Edition::CURRENT,
            disable_all_formatting: false,
            dump_parse_tree: false,
            dump_doc_tree: false,
            check_result: false,
            allow_unsupported: false,
        }
    }
}
