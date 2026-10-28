pub(crate) mod cargo;
pub(crate) mod lookup;
pub(crate) mod parser;
pub(crate) mod source;

use ra_ap_syntax::Edition;

/// The line endings of the output (rustfmt's `newline_style`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewlineStyle {
    /// Those of the first line of the source (rustfmt documents it so, but compares with the source
    /// after rustc has turned its `\r\n` into `\n`, so it always writes `\n`)
    Auto,
    /// `\r\n` on Windows, `\n` elsewhere
    Native,
    /// `\n`
    Unix,
    /// `\r\n`
    Windows,
}

impl NewlineStyle {
    /// Whether the output of `source` ends its lines with `\r\n`
    pub fn uses_crlf(self, source: &str) -> bool {
        match self {
            NewlineStyle::Auto => source
                .find('\n')
                .is_some_and(|i| source[..i].ends_with('\r')),
            NewlineStyle::Native => cfg!(windows),
            NewlineStyle::Unix => false,
            NewlineStyle::Windows => true,
        }
    }
}

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
    /// Maximal width of `let pat = init else { expr };` put on a single line; 0 always breaks it
    pub single_line_let_else_max_width: u32,
    /// Maximal width of an argument for which the arguments of a broken call are filled on lines
    /// (`a, b, c,` / `d, e,`) instead of one per line; every argument has to be simple (a literal, a name).
    /// 0 disables it.
    pub short_array_element_width_threshold: u32,
    /// `;` after a `return`, `break` or `continue` ending a broken block (`break;`), or none (`break`)
    pub trailing_semicolon: bool,
    /// `,` after a match arm whose body is a block (`A => {` / `...` / `},`); the Style Guide puts none
    pub match_block_trailing_comma: bool,
    /// Formats the transcribers of `macro_rules!` that parse as items, statements or an expression
    /// (rustfmt's `format_macro_bodies`); otherwise they are copied
    pub format_macro_bodies: bool,
    /// The gap after which the widest list items do not align their trailing comments; 0 disables it
    pub trailing_comment_align_threshold: u32,
    /// The cost of a line that overflows `max_width`, in columns of overflow: a layout with one more
    /// overflowing line is preferred only when it overflows this many columns less
    pub overflow_line_penalty: u32,
    /// Puts a single space after the opening sigil of a comment and before the closing one:
    /// `//a` -> `// a`, `/*a*/` -> `/* a */` (see `normalize_comment_sigil_spacing`); off by default,
    /// as rustfmt leaves the text of comments alone
    pub normalize_comment_sigil_spacing: bool,
    /// The line endings of the output
    pub newline_style: NewlineStyle,
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
            single_line_let_else_max_width: 50,
            short_array_element_width_threshold: 10,
            trailing_semicolon: true,
            match_block_trailing_comma: false,
            format_macro_bodies: true,
            trailing_comment_align_threshold: 40,
            overflow_line_penalty: 20,
            normalize_comment_sigil_spacing: false,
            newline_style: NewlineStyle::Auto,
            edition: Edition::CURRENT,
            disable_all_formatting: false,
            dump_parse_tree: false,
            dump_doc_tree: false,
            check_result: false,
            allow_unsupported: false,
        }
    }
}
