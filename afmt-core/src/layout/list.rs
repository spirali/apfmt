//! Delimited, comma-separated lists

use crate::comments::{
    comments_at_closing_indentation, comments_break_line_after, is_code_kind, is_comment_kind,
    is_line_comment, next_code_token, own_line_comment_follows, prev_code_token, trivia_from,
};
use crate::doc::WeakPreference;
use crate::formatter::Edge::{BraceClose, BraceOpen, Close, Comma, Open, Word};
use crate::formatter::{Builder, Edge, Formatter, Frag};
use crate::utils::with_is_last;
use ra_ap_syntax::ast::TokenTree;
use ra_ap_syntax::{AstNode, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, T};

/// An item of a comma-separated list with the source comma following it. The item is usually
/// the formatted `Frag`, or its parts when its layout is chosen later (see `ListItem::map`).
#[derive(Clone)]
pub(crate) struct ListItem<T = Frag> {
    pub frag: T,
    /// `None` for the last item without a trailing comma
    pub comma: Option<SyntaxToken>,
    /// The item is alone on its line in a FillList (in the other layouts, it is placed as any item)
    pub own_line: bool,
    /// Whether the item may overflow as the last item of the list
    pub overflow: Overflow,
}

/// Whether and how the last item of a list may overflow: it starts on the line
/// of the opening delimiter, after the other items, and ends on the line of the closing one
/// (`f(if a {` / `...` / `})`, `f(a, |x| {` / `...` / `})`); only a combinable expression may
#[derive(Clone, Default)]
pub(crate) enum Overflow {
    #[default]
    Never,
    /// With these layouts of the item; the list layout with fewer lines wins
    With(Frag),
    /// With these layouts of the item; the list layout with fewer lines wins, and of layouts with
    /// as many lines the overflowing one (it gets a `WeakPreference`; Style Guide: a single
    /// combinable argument, `foo(bar(` / `...` / `))`, "apply this behavior recursively": the deepest
    /// one that fits)
    WeakPreference(Frag),
    /// With these layouts of the item, preferred whenever it fits: the broken layouts of the list
    /// cost a split (Style Guide: closures)
    Preferred(Frag),
    /// With these layouts of the item, the only broken layout of the list unless a comment rules
    /// the overflow out (a single closure parameter hugged by the pipes: `|(` / `    a,` / `)|`)
    Only(Frag),
}

impl Overflow {
    /// The layouts of the overflowing item, if it may overflow
    pub fn layouts(&self) -> Option<&Frag> {
        match self {
            Overflow::Never => None,
            Overflow::With(layouts)
            | Overflow::WeakPreference(layouts)
            | Overflow::Preferred(layouts)
            | Overflow::Only(layouts) => Some(layouts),
        }
    }

    /// The same overflow with the layouts of the item changed by `f`
    pub fn map(&self, f: impl FnOnce(&Frag) -> Frag) -> Overflow {
        match self {
            Overflow::Never => Overflow::Never,
            Overflow::With(layouts) => Overflow::With(f(layouts)),
            Overflow::WeakPreference(layouts) => Overflow::WeakPreference(f(layouts)),
            Overflow::Preferred(layouts) => Overflow::Preferred(f(layouts)),
            Overflow::Only(layouts) => Overflow::Only(f(layouts)),
        }
    }
}

impl<T> ListItem<T> {
    /// The item `frag` without a comma, placed as any item and not overflowing
    pub fn new(frag: T) -> Self {
        ListItem {
            frag,
            comma: None,
            own_line: false,
            overflow: Overflow::Never,
        }
    }

    /// The item formatted by `format`, with the same comma, `own_line` and `overflow`
    pub fn map(&self, format: impl FnOnce(&T) -> Frag) -> ListItem {
        ListItem {
            frag: format(&self.frag),
            comma: self.comma.clone(),
            own_line: self.own_line,
            overflow: self.overflow.clone(),
        }
    }

    /// If `own_line`, the item is alone on its line in a FillList
    pub fn own_line_if(mut self, own_line: bool) -> Self {
        self.own_line = own_line;
        self
    }

    /// How the item may overflow as the last item of the list
    pub fn with_overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }
}

/// How the last item of a list may hug it (overflow, see `Overflow`)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Hug {
    Never,
    /// When the layout with fewer lines wins
    Allowed,
    /// Also in as many lines as the broken list, see `Overflow::WeakPreference`
    WeakPreference,
    /// Whenever it fits, see `Overflow::Preferred`
    Preferred,
}

impl Hug {
    /// The overflow of the last item, which may overflow by itself as `overflow`, as the hug allows
    pub fn overflow(self, overflow: &Overflow) -> Overflow {
        match (self, overflow) {
            (Hug::Never, _) => Overflow::Never,
            (Hug::Preferred, Overflow::With(frag)) => Overflow::Preferred(frag.clone()),
            (Hug::WeakPreference, Overflow::With(frag)) => Overflow::WeakPreference(frag.clone()),
            (_, overflow) => overflow.clone(),
        }
    }
}

/// The last item of a list may overflow with its own layouts as `hug` allows it
pub(crate) fn hug_last_item(items: &mut [ListItem], hug: Hug) {
    if let Some(item) = items.last_mut() {
        item.overflow = hug.overflow(&Overflow::With(item.frag.clone()));
    }
}

/// The items of the comma-separated list `node` (an argument list, a parameter list, ...) with their
/// commas. `format` formats a child node, or returns `None` for children that are not items.
pub(crate) fn list_items<T>(
    node: &SyntaxNode,
    mut format: impl FnMut(SyntaxNode) -> Option<T>,
) -> Vec<ListItem<T>> {
    let mut items: Vec<ListItem<T>> = Vec::new();
    for element in node.children_with_tokens() {
        match element {
            SyntaxElement::Node(child) => {
                if let Some(frag) = format(child) {
                    items.push(ListItem::new(frag));
                }
            }
            SyntaxElement::Token(token) if token.kind() == SyntaxKind::COMMA => {
                // A comma always follows an item and is never doubled, otherwise the parser reports an error
                let last = items.last_mut().filter(|item| item.comma.is_none());
                debug_assert!(last.is_some(), "unexpected comma in {node:?}");
                if let Some(item) = last {
                    item.comma = Some(token);
                }
            }
            SyntaxElement::Token(_) => {}
        }
    }
    items
}

pub(crate) fn list_items_cast<T: AstNode, S>(
    node: &SyntaxNode,
    mut format: impl FnMut(T) -> S,
) -> Vec<ListItem<S>> {
    list_items(node, |node| T::cast(node).map(&mut format))
}

/// All items on one line, `a, b, c`, with a trailing comma only if it is `Required`; `open` is the
/// opening delimiter and the comments following it are part of the list. The caller restricts it to
/// a single line.
fn flat_list(
    fmt: &Formatter,
    items: &[ListItem],
    open: Option<SyntaxToken>,
    trailing: TrailingSeparator,
) -> Frag {
    let mut builder = Builder::with_cursor(fmt, open);
    builder.flush_trivia();
    for (item, is_last) in with_is_last(items) {
        builder.push(item.frag.clone());
        let comma = !is_last || trailing == TrailingSeparator::Required;
        push_or_skip_comma(&mut builder, item, comma);
    }
    builder.flush_trivia();
    // A line comment ends the line inside the list, so `Flat` rejects such a layout
    builder.end_line_after_line_comment();
    builder.into_frag()
}

/// The comma after `item` if `comma`, otherwise only its source comma (if any) is consumed. Unlike
/// `Builder::push_or_skip_separator`, the source comma itself is printed, so comments in front of it
/// stay there (`a /* c */, b`), as on a single line
fn push_or_skip_comma(builder: &mut Builder, item: &ListItem, comma: bool) {
    match &item.comma {
        // The source separator, so comments in front of it stay there: `a /* c */, b`
        Some(source) if comma => builder.push_token(source.clone(), Comma),
        // A required trailing comma missing in the source
        None if comma => builder.push_separator(SyntaxKind::COMMA),
        // No trailing comma, but the source one and its comments still have to be consumed
        _ => builder.skip_separator(SyntaxKind::COMMA),
    }
}

/// Items filled on lines, as many on each line as fit: `a, b, c,` / `d, e,`; with a line break after `open`,
/// indented, and a trailing comma (unless it is `Never` or `Omitted`). Items with `own_line` are
/// alone on their line. Comments are placed like rustfmt does: a comment before the comma or ending
/// the line belongs to the item and ends the line, a comment followed by an item on the same line
/// stays in front of it, and an own-line comment gets its own line. Comments ending the line of
/// `open` stay there, as in vertical lists.
fn fill_list(
    fmt: &Formatter,
    items: &[ListItem],
    open: Option<SyntaxToken>,
    trailing: TrailingSeparator,
) -> Frag {
    let mut builder = Builder::with_cursor(fmt, open);
    let line_comments = builder.push_comments_ending_line();
    builder.align_trailing_comments(fmt.align_group());
    for (i, item) in items.iter().enumerate() {
        builder.push_leading_comments();
        builder.push(item.frag.clone());
        let Some(next) = items.get(i + 1) else {
            builder.push_or_skip_separator(SyntaxKind::COMMA, trailing.comma_when_broken());
            break;
        };
        let had_post_comment = builder.push_separator_with_trailing_comments(SyntaxKind::COMMA);
        if had_post_comment
            || item.own_line
            || next.own_line
            || builder.cursor().is_some_and(own_line_comment_follows)
        {
            builder.push_newline();
        } else {
            builder.push_soft_line_break();
        }
    }
    builder.flush_trivia();
    builder.push_newline();
    fmt.body_after_opener(line_comments, builder.into_frag())
}

/// One item per line, aligned to the column after `open` (visual indentation), without a trailing
/// comma; the closing delimiter follows the last item: `|a: A,` / ` b: B|`. Only for closure
/// parameters, an explicit exception to the Style Guide (see CLAUDE.md).
fn aligned_list(fmt: &Formatter, items: &[ListItem], open: Option<SyntaxToken>) -> Frag {
    fmt.align(aligned_items(fmt, items, open))
}

/// The items of `aligned_list`, not aligned yet
fn aligned_items(fmt: &Formatter, items: &[ListItem], open: Option<SyntaxToken>) -> Frag {
    let mut builder = Builder::with_cursor(fmt, open);
    builder.flush_trivia();
    for (item, is_last) in with_is_last(items) {
        builder.push(item.frag.clone());
        if is_last {
            builder.skip_separator(SyntaxKind::COMMA);
            break;
        }
        builder.push_separator(SyntaxKind::COMMA);
        builder.flush_trivia();
        builder.push_newline();
    }
    builder.flush_trivia();
    // A line comment would swallow the closing delimiter
    builder.end_line_after_line_comment();
    builder.into_frag()
}

/// The items aligned after the opening delimiter as in `aligned_list`, followed by `tail` on a line
/// of its own, aligned with them: `|a: A,` / ` b: B|` / ` -> T` (closure parameters and return type)
pub(crate) fn aligned_list_with_tail(
    fmt: &Formatter,
    delims: &Delimiters,
    items: &[ListItem],
    tail: Frag,
) -> Frag {
    let mut aligned = Builder::new(fmt);
    aligned.push(aligned_items(fmt, items, Some(delims.open.clone())));
    aligned.push(delims.format_close(fmt));
    aligned.push_newline();
    aligned.push(tail);
    let mut builder = Builder::new(fmt);
    builder.push(delims.format_open(fmt));
    builder.push(fmt.align(aligned.into_frag()));
    builder.into_frag()
}

/// One item per line with a trailing comma (unless it is `Never` or `Omitted`), indented; `open` is the opening
/// delimiter and the comments following it are part of the list. With `keep_blank_lines`, blank lines between items are kept
/// (up to `blank_lines_upper_bound`), e.g. between struct fields. With `lines`, the lines have these
/// numbers of items, each line on a single line (see `ListLayout::lines_if`).
fn vertical_list(
    fmt: &Formatter,
    items: &[ListItem],
    open: Option<SyntaxToken>,
    keep_blank_lines: bool,
    trailing: TrailingSeparator,
    lines: Option<&[usize]>,
) -> Frag {
    let mut lines = lines.map(|lines| lines.iter().copied());
    // The items left on the current line
    let mut left = 0;
    let mut builder = Builder::with_cursor(fmt, open);
    let line_comments = builder.push_comments_ending_line();
    builder.push_leading_comments();
    builder.align_trailing_comments(fmt.align_group());
    for (item, is_last) in with_is_last(items) {
        let ends_line = match &mut lines {
            Some(lines) => {
                if left == 0 {
                    left = lines.next().unwrap();
                }
                left -= 1;
                left == 0
            }
            None => true,
        };
        builder.push(if lines.is_some() {
            fmt.flat(item.frag.clone())
        } else {
            item.frag.clone()
        });
        builder.push_or_skip_separator(SyntaxKind::COMMA, !is_last || trailing.comma_when_broken());
        if is_last && trailing == TrailingSeparator::Never {
            // The token after the list follows the last item on its line: `T: Clone;`
            break;
        }
        if !ends_line {
            continue;
        }
        // Blank lines in front of the closing delimiter are dropped
        if keep_blank_lines && !is_last {
            builder.flush_trivia_and_end_line();
        } else {
            builder.flush_trivia();
            builder.push_newline();
        }
    }
    fmt.body_after_opener(line_comments, builder.into_frag())
}

/// The delimiters of a list
#[derive(Clone)]
pub(crate) struct Delimiters {
    open: SyntaxToken,
    close: SyntaxToken,
    kind: DelimiterKind,
}

#[derive(Clone, Copy)]
enum DelimiterKind {
    /// `( )`: an empty list is `()` with its comments in between
    Parens,
    /// `< >`: as `Parens`
    Angles,
    /// `[ ]`: as `Parens`
    Brackets,
    /// `| |` of closure parameters: as `Parens`
    Pipes,
    /// `{ }`: a single-line list has spaces inside, comments of an empty list go on their own
    /// indented lines
    Braces,
    /// `{ }` without spaces inside, as `Parens`: `a::{b, c}`
    TightBraces,
}

impl Delimiters {
    /// The `(` and `)` of the list `node` (an argument list, a parameter list, ...)
    pub fn parens_of(node: &SyntaxNode) -> Self {
        Self::of(
            node,
            SyntaxKind::L_PAREN,
            SyntaxKind::R_PAREN,
            DelimiterKind::Parens,
        )
    }

    /// The `(` and `)` among the children of `node`, if it has them (a parenthesized bound)
    pub fn try_parens_of(node: &SyntaxNode) -> Option<Self> {
        Self::try_of(
            node,
            SyntaxKind::L_PAREN,
            SyntaxKind::R_PAREN,
            DelimiterKind::Parens,
        )
    }

    /// The `<` and `>` of the list `node` (generic arguments, `<T as Trait>`)
    pub fn angles_of(node: &SyntaxNode) -> Self {
        Self::of(
            node,
            SyntaxKind::L_ANGLE,
            SyntaxKind::R_ANGLE,
            DelimiterKind::Angles,
        )
    }

    /// The `[` and `]` of the list `node` (an array, a slice pattern, an attribute)
    pub fn brackets_of(node: &SyntaxNode) -> Self {
        Self::of(
            node,
            SyntaxKind::L_BRACK,
            SyntaxKind::R_BRACK,
            DelimiterKind::Brackets,
        )
    }

    /// The `|` and `|` of the closure parameters `node`
    pub fn pipes_of(node: &SyntaxNode) -> Self {
        Self::of(
            node,
            SyntaxKind::PIPE,
            SyntaxKind::PIPE,
            DelimiterKind::Pipes,
        )
    }

    /// The `{` and `}` of the list `node` (a record field list, a variant list, ...)
    pub fn braces_of(node: &SyntaxNode) -> Self {
        Self::of(
            node,
            SyntaxKind::L_CURLY,
            SyntaxKind::R_CURLY,
            DelimiterKind::Braces,
        )
    }

    /// The `{` and `}` of the list `node` without spaces inside a single-line list (a use tree list:
    /// `a::{b, c}`)
    pub fn tight_braces_of(node: &SyntaxNode) -> Self {
        Self::of(
            node,
            SyntaxKind::L_CURLY,
            SyntaxKind::R_CURLY,
            DelimiterKind::TightBraces,
        )
    }

    /// The first `open` and the last `close` token among the children of `node`
    fn of(node: &SyntaxNode, open: SyntaxKind, close: SyntaxKind, kind: DelimiterKind) -> Self {
        Self::try_of(node, open, close, kind).unwrap()
    }

    /// The delimiters among the children of `node`, if it has them
    fn try_of(
        node: &SyntaxNode,
        open: SyntaxKind,
        close: SyntaxKind,
        kind: DelimiterKind,
    ) -> Option<Self> {
        let token = |kind: SyntaxKind| {
            node.children_with_tokens()
                .filter_map(SyntaxElement::into_token)
                .filter(move |t| t.kind() == kind)
        };
        Some(Delimiters {
            open: token(open).next()?,
            close: token(close).last()?,
            kind,
        })
    }

    /// The delimiters of a token tree: `(...)`, `[...]` or `{...}`
    pub fn of_token_tree(tree: &TokenTree) -> Self {
        let (open, close) = (tree.syntax().first_token(), tree.syntax().last_token());
        let kind = match open.as_ref().map(|t| t.kind()) {
            Some(SyntaxKind::L_CURLY) => DelimiterKind::Braces,
            Some(SyntaxKind::L_BRACK) => DelimiterKind::Brackets,
            _ => DelimiterKind::Parens,
        };
        Delimiters {
            open: open.unwrap(),
            close: close.unwrap(),
            kind,
        }
    }

    /// The source from the opening to the closing delimiter, reindented (see
    /// `Formatter::reindented_range`), spaced by the delimiters: `{ ... }` as a block, `(...)` and
    /// `[...]` as a list
    pub fn format_reindented(&self, fmt: &Formatter) -> Frag {
        let (start, end) = self.edges();
        fmt.reindented_range(&self.open, &self.close, start, end)
    }

    fn edges(&self) -> (Edge, Edge) {
        match self.kind {
            DelimiterKind::Parens
            | DelimiterKind::Angles
            | DelimiterKind::Brackets
            | DelimiterKind::Pipes
            | DelimiterKind::TightBraces => (Open, Close),
            DelimiterKind::Braces => (BraceOpen, BraceClose),
        }
    }

    /// `inner` between the delimiters; `inner` places the comments following the opening delimiter, as
    /// the list layouts do (they start with the opening delimiter as their cursor)
    pub fn wrap_body(&self, fmt: &Formatter, inner: Frag) -> Frag {
        self.surround(fmt, self.format_open(fmt), inner)
    }

    /// `inner` between the delimiters, e.g. `(a + b)`, `<T as Trait>`, `{ x }`; the comments
    /// following the opening delimiter are placed in front of `inner`. When comments put `inner` on
    /// a line after the opening delimiter (see `comments_break_line_after`) or an own-line comment
    /// follows it, the delimiters are broken as those of a block: `(` / `    // c` / `    a && b` /
    /// `)`, with a comment ending the line of `(` kept there (see `body_after_opener`).
    pub fn enclose(&self, fmt: &Formatter, inner: Frag) -> Frag {
        let (open_edge, _) = self.edges();
        let comment_before_close =
            prev_code_token(&self.close).is_some_and(|t| own_line_comment_follows(&t));
        if !(comments_break_line_after(&self.open) || comment_before_close) {
            return self.surround(fmt, fmt.token(self.open.clone(), open_edge), inner);
        }
        let mut body = Builder::with_cursor(fmt, Some(self.open.clone()));
        let line_comments = body.push_comments_ending_line();
        body.push_comments();
        body.push_on_new_line(inner);
        body.flush_trivia();
        body.push_newline();
        self.wrap_body(fmt, fmt.body_after_opener(line_comments, body.into_frag()))
    }

    /// The opening delimiter, whose following comments are placed by the fragment after it
    fn format_open(&self, fmt: &Formatter) -> Frag {
        fmt.token_without_cursor(self.open.clone(), self.edges().0)
    }

    /// The closing delimiter
    fn format_close(&self, fmt: &Formatter) -> Frag {
        fmt.token(self.close.clone(), self.edges().1)
    }

    fn surround(&self, fmt: &Formatter, open: Frag, inner: Frag) -> Frag {
        let mut builder = Builder::new(fmt);
        builder.push(open);
        builder.push(inner);
        builder.push(self.format_close(fmt));
        builder.into_frag()
    }

    /// The container without items or statements: `()`, `{}`, or only comments,
    pub fn format_empty(&self, fmt: &Formatter) -> Frag {
        // The container is empty, so only trivia is between the delimiters
        let inside = || trivia_from(self.open.next_token());
        if !inside().any(|t| is_comment_kind(t.kind())) {
            return self.wrap_body(fmt, Frag::empty());
        }
        if !inside().any(|t| t.text().contains('\n')) {
            let mut comments = Builder::with_cursor(fmt, Some(self.open.clone()));
            comments.flush_trivia();
            let comments = comments.into_frag();
            let mut own_line = Builder::new(fmt);
            own_line.push(comments.clone());
            own_line.push_newline();
            // On their own line only if they do not fit on the delimiters' line
            let inline = fmt.flat(self.wrap_body(fmt, comments));
            let own_line = self.wrap_body(fmt, fmt.body_after_opener(false, own_line.into_frag()));
            return fmt.alt_frag(inline, own_line);
        }
        let mut body = Builder::with_cursor(fmt, Some(self.open.clone()));
        let line_comments = body.push_comments_ending_line();
        body.flush_trivia();
        body.push_newline();
        self.wrap_body(fmt, fmt.body_after_opener(line_comments, body.into_frag()))
    }

    /// `{`, then `lines`, each on its own line and block indented, then `}`;
    /// the blank lines of the source between the lines are kept, those after `{` and
    /// in front of `}` dropped. Without lines, the empty container (see `format_empty`).
    ///
    /// In front of `} else`, own-line comments written at the indentation of `}` stay there, as they
    /// introduce the `else` branch (see `comments_at_closing_indentation`); rustfmt keeps the
    /// author's choice the same way. In front of another `}` they are indented with the lines.
    pub fn format_block(&self, fmt: &Formatter, lines: impl IntoIterator<Item = Frag>) -> Frag {
        let mut lines = lines.into_iter().peekable();
        if lines.peek().is_none() {
            return self.format_empty(fmt);
        }
        // Tokens following `{` belong to the body, so they are nested with it
        let mut builder = Builder::with_cursor(fmt, Some(self.open.clone()));
        let line_comments = builder.push_comments_ending_line();
        builder.push_comments();
        for line in lines {
            builder.push_on_new_line(line);
        }
        let before_else = next_code_token(&self.close).is_some_and(|t| t.kind() == T![else]);
        if !(before_else && comments_at_closing_indentation(&self.close)) {
            builder.flush_trivia();
            builder.push_newline();
            return self.wrap_body(
                fmt,
                fmt.body_after_opener(line_comments, builder.into_frag()),
            );
        }
        // The comments ending the last line stay on it; the others follow the indented lines
        builder.push_comments_ending_line();
        let mut body = Builder::new(fmt);
        body.push(fmt.body_after_opener(line_comments, builder.into_frag()));
        body.flush_trivia();
        body.push_newline();
        self.wrap_body(fmt, body.into_frag())
    }

    /// An own-line comment after the opening delimiter allows only broken layouts
    fn has_own_line_comment(&self) -> bool {
        own_line_comment_follows(&self.open)
    }

    /// Whether a line comment is between the last item and the closing delimiter: `a, // c` / `)`
    fn has_line_comment_before_close(&self) -> bool {
        std::iter::successors(self.close.prev_token(), |t| t.prev_token())
            .take_while(|t| !is_code_kind(t.kind()) || t.kind() == SyntaxKind::COMMA)
            .any(|t| is_line_comment(&t))
    }
}

/// The layouts a delimited list may take, see `delimited_list`. Start with `tall` or `fixed_vertical`
/// and refine it with `fill_if`, `broken_if` and `trailing_separator`. The last item may overflow if
/// it allows it, see `Overflow`.
pub(crate) struct ListLayout {
    /// The items on a single line; `None` allows only the broken layout
    flat: Option<FlatLimit>,
    broken: BrokenList,
    trailing: TrailingSeparator,
    /// The numbers of items on the lines of another broken layout, see `lines_if`
    lines: Option<Vec<usize>>,
}

impl ListLayout {
    /// On a single line if it fits, otherwise one item per line
    pub fn tall(flat: FlatLimit) -> Self {
        ListLayout {
            flat: Some(flat),
            broken: BrokenList::Vertical {
                keep_blank_lines: false,
            },
            trailing: TrailingSeparator::IfVertical,
            lines: None,
        }
    }

    /// Always one item per line, blank lines between items are kept (struct fields, enum variants)
    pub fn fixed_vertical() -> Self {
        ListLayout {
            flat: None,
            broken: BrokenList::Vertical {
                keep_blank_lines: true,
            },
            trailing: TrailingSeparator::IfVertical,
            lines: None,
        }
    }

    /// The broken layout has one item per line aligned to the opening delimiter, see `aligned_list`
    pub fn aligned(mut self) -> Self {
        self.broken = BrokenList::Aligned;
        self
    }

    /// If `fill`, the broken layout has as many items per line as fit instead of one per line
    pub fn fill_if(mut self, fill: bool) -> Self {
        if fill {
            self.broken = BrokenList::Fill;
        }
        self
    }

    /// If `broken`, only the broken layout is allowed, even when the items fit on a single line
    pub fn broken_if(mut self, broken: bool) -> Self {
        if broken {
            self.flat = None;
        }
        self
    }

    pub fn trailing_separator(mut self, trailing: TrailingSeparator) -> Self {
        self.trailing = trailing;
        self
    }

    /// With `lines`, the list may also be broken with these numbers of items on its lines, each line
    /// on a single line, e.g. the arguments of a format-string macro: `x, y,` / `"{} {}",` / `a, b`;
    /// the items must have no comments between them
    pub fn lines_if(mut self, lines: Option<Vec<usize>>) -> Self {
        debug_assert!(
            lines
                .as_ref()
                .is_none_or(|lines| lines.iter().all(|&n| n > 0))
        );
        self.lines = lines;
        self
    }
}

/// When a list has a comma after its last item
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrailingSeparator {
    /// Only when the list is broken (vertical or filled), never on a single line
    IfVertical,
    /// Always, also on a single line, because the comma changes the meaning: the tuple pattern `(a,)`
    /// is not the parenthesized pattern `(a)`
    Required,
    /// Never; in a vertical layout, the token after the list follows the last item on its line, e.g.
    /// the `;` after a `where` clause: `    T: Clone;`
    Never,
    /// Never, because no item may follow the last one; in a vertical layout, the closing delimiter is
    /// on its own line, e.g. after `..` in a struct pattern: `    ..` / `}`
    Omitted,
}

impl TrailingSeparator {
    /// Whether a broken list (vertical or filled) has a comma after its last item
    fn comma_when_broken(self) -> bool {
        matches!(
            self,
            TrailingSeparator::IfVertical | TrailingSeparator::Required
        )
    }
}

/// When the items may be put on a single line
#[derive(Clone, Copy)]
pub(crate) enum FlatLimit {
    /// Whenever the line fits within `max_width`
    MaxWidth,
    /// Only when the items between the delimiters are at most this wide (`fn_call_width`, ...)
    Within(u32),
}

impl FlatLimit {
    /// The single-line limit of the arguments `items` of a call or an attribute: `width`
    /// (`fn_call_width`, `attr_fn_like_width`), but only `max_width` for a single item that cannot
    /// hug the list as `hug` allows, or has no layout spanning more lines (a string literal,
    /// `f(a::B::new())`, a macro call copied unchanged): broken, it would only move to a line of its
    /// own, as rustfmt does not (`#[error("...")]`, `f("...")`)
    pub fn of_arguments(fmt: &Formatter, items: &[ListItem], hug: Hug, width: u32) -> Self {
        match items {
            [item] if hug == Hug::Never || !fmt.has_line_break(&item.frag) => FlatLimit::MaxWidth,
            _ => FlatLimit::Within(width),
        }
    }

    /// `frag` restricted to a single line, if it has a single-line layout and its width allows it
    pub fn apply(self, fmt: &Formatter, frag: Frag) -> Option<Frag> {
        match self {
            FlatLimit::MaxWidth => fmt.flat_if_possible(frag),
            FlatLimit::Within(width) => fmt.flat_within(frag, width),
        }
    }
}

/// The layout of a list that does not fit on a single line
#[derive(Clone, Copy)]
enum BrokenList {
    /// One item per line, see `vertical_list`
    Vertical { keep_blank_lines: bool },
    /// As many items per line as fit, except items with `own_line`, see `fill_list`
    Fill,
    /// One item per line, visually aligned, see `aligned_list`
    Aligned,
}

/// The items on a single line, without the delimiters, if `limit` allows it and no own-line comment
/// follows the opening delimiter; wrap it with `Delimiters::wrap_body`
pub(crate) fn flat_layout(
    fmt: &Formatter,
    delims: &Delimiters,
    items: &[ListItem],
    limit: FlatLimit,
    trailing: TrailingSeparator,
) -> Option<Frag> {
    if delims.has_own_line_comment() {
        return None;
    }
    let flat = flat_list(fmt, items, Some(delims.open.clone()), trailing);
    limit.apply(fmt, flat)
}

/// Items following a keyword, always one per line and indented: `where` / `    T: Clone,`.
/// The list ends with a line break, so the next token starts a new line (`{`), unless
/// `trailing` is `Never`: then it follows the last item, which has no comma (`    T: Clone;`).
pub(crate) fn keyword_list(
    fmt: &Formatter,
    keyword: SyntaxToken,
    items: &[ListItem],
    trailing: TrailingSeparator,
) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(fmt.token_without_cursor(keyword.clone(), Word));
    builder.push(vertical_list(
        fmt,
        items,
        Some(keyword),
        false,
        trailing,
        None,
    ));
    builder.into_frag()
}

/// The last item overflows with its layouts `last_layouts` (see `Overflow`); the items in front of it
/// and the first line of the last one are on the line of the opening delimiter, within the width of
/// `limit` (as rustfmt does: `f(a, b, |x| {` is a single-line call as wide as its first line). The
/// last item has a comma only if it is `Required` (`(match a {` / `...` / `},)`).
fn overflow_layout(
    fmt: &Formatter,
    delims: &Delimiters,
    items: &[ListItem],
    last_layouts: &Frag,
    limit: FlatLimit,
    trailing: TrailingSeparator,
) -> Option<Frag> {
    let rest = &items[..items.len() - 1];
    let (mut overflow, last_layouts) = if rest.is_empty() {
        let overflow = Builder::with_cursor(fmt, Some(delims.open.clone()));
        (overflow, last_layouts.clone())
    } else {
        let line = flat_layout(fmt, delims, rest, limit, TrailingSeparator::Required)?;
        let last_layouts = match limit {
            FlatLimit::MaxWidth => last_layouts.clone(),
            FlatLimit::Within(width) => {
                // The items in front, their trailing comma and the space after it
                let rest_width = fmt.min_flat_width(&line).unwrap_or(0) + 1;
                let first_line_width = width.checked_sub(rest_width).filter(|&w| w > 0)?;
                // Not squeezed into the width by a layout that costs a split (`|x| match x` / `{`,
                // braces added around a match): the list is broken instead, as rustfmt does
                if fmt
                    .natural_first_line_width(last_layouts)
                    .is_some_and(|w| w > first_line_width)
                {
                    return None;
                }
                fmt.first_line_within(last_layouts.clone(), first_line_width)
            }
        };
        let mut overflow = Builder::new(fmt);
        overflow.push(line);
        (overflow, last_layouts)
    };
    overflow.flush_trivia();
    overflow.push(last_layouts);
    let last = items.last().unwrap();
    push_or_skip_comma(&mut overflow, last, trailing == TrailingSeparator::Required);
    overflow.flush_trivia();
    // Only an item that is broken overflows; on a single line, it is the flat layout
    Some(fmt.multi_line(overflow.into_frag()))
}

/// A delimited list, `(a, b)`, `<A, B>` or `{ a, b }`, in the layouts `layout` allows
pub(crate) fn delimited_list(
    fmt: &Formatter,
    delims: &Delimiters,
    items: &[ListItem],
    layout: ListLayout,
) -> Frag {
    if items.is_empty() {
        return delims.format_empty(fmt);
    }
    let open = Some(delims.open.clone());
    let last_overflow = &items.last().unwrap().overflow;
    // A line comment after the last item would end the line in front of the closing delimiter
    let overflow = (last_overflow.layouts())
        .filter(|_| !delims.has_own_line_comment() && !delims.has_line_comment_before_close())
        .and_then(|last_layouts| {
            let limit = layout.flat.unwrap_or(FlatLimit::MaxWidth);
            let overflow =
                overflow_layout(fmt, delims, items, last_layouts, limit, layout.trailing)?;
            Some(match last_overflow {
                Overflow::WeakPreference(_) => {
                    fmt.weak_preference(overflow, WeakPreference::PREFERRED)
                }
                _ => overflow,
            })
        });
    let overflow_only = matches!(last_overflow, Overflow::Only(_));
    let broken = (overflow.is_none() || !overflow_only).then(|| {
        let broken = match layout.broken {
            BrokenList::Vertical { keep_blank_lines } => vertical_list(
                fmt,
                items,
                open.clone(),
                keep_blank_lines,
                layout.trailing,
                None,
            ),
            BrokenList::Fill => fill_list(fmt, items, open.clone(), layout.trailing),
            BrokenList::Aligned => aligned_list(fmt, items, open.clone()),
        };
        // With a preferred overflow, the broken layouts are used only when it does not fit; an item
        // that cannot span more lines has no overflow (a macro call copied on one line)
        match (&overflow, last_overflow) {
            (Some(_), Overflow::Preferred(last) | Overflow::Only(last))
                if fmt.has_line_break(last) =>
            {
                fmt.dispreferred(broken)
            }
            _ => broken,
        }
    });
    let mut layouts = fmt
        .alt_some(broken, overflow)
        .expect("a broken or an overflow layout");
    if let Some(limit) = layout.flat {
        layouts = fmt.alt_opt(
            layouts,
            flat_layout(fmt, delims, items, limit, layout.trailing),
        );
    }
    let layouts = delims.wrap_body(fmt, layouts);
    match &layout.lines {
        Some(lines) => {
            debug_assert_eq!(lines.iter().sum::<usize>(), items.len());
            let grouped = vertical_list(fmt, items, open, false, layout.trailing, Some(lines));
            fmt.alt_frag(layouts, delims.wrap_body(fmt, grouped))
        }
        None => layouts,
    }
}
