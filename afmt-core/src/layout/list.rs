//! Delimited, comma-separated lists

use crate::comments::{is_line_comment, own_line_comment_follows, trivia_from};
use crate::doc::WeakPreference;
use crate::formatter::Edge::{BraceClose, BraceOpen, Close, Comma, Open, Word};
use crate::formatter::{Builder, Edge, Formatter, Frag};
use crate::utils::with_is_last;
use ra_ap_syntax::ast::TokenTree;
use ra_ap_syntax::{AstNode, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

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

/// The only item of a list may overflow with its own layouts (HugList), e.g. the pattern of
/// `Some(Foo {` / `...` / `})`; a list with more items is left as it is
pub(crate) fn hug_single_item(items: &mut [ListItem]) {
    if let [item] = items {
        item.overflow = Overflow::With(item.frag.clone());
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

/// The comma after `item` if `comma`, otherwise only its source comma (if any) is consumed
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
/// indented, and a trailing comma. Items with `own_line` are alone on their line. Comments are placed
/// like rustfmt does: a comment before the comma or ending the line belongs to the item and ends the
/// line, a comment followed by an item on the same line stays in front of it, and an own-line comment
/// gets its own line. Comments ending the line of `open` stay there, as in vertical lists.
fn fill_list(fmt: &Formatter, items: &[ListItem], open: Option<SyntaxToken>) -> Frag {
    let mut builder = Builder::with_cursor(fmt, open);
    let line_comments = builder.push_comments_ending_line();
    builder.align_trailing_comments(fmt.align_group());
    for (i, item) in items.iter().enumerate() {
        builder.push_leading_comments();
        builder.push(item.frag.clone());
        let had_post_comment = builder.push_separator_with_trailing_comments(SyntaxKind::COMMA);
        let Some(next) = items.get(i + 1) else {
            break;
        };
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
    fmt.align(builder.into_frag())
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
        let skip = matches!(
            trailing,
            TrailingSeparator::Never | TrailingSeparator::Omitted
        );
        if is_last && skip {
            builder.skip_separator(SyntaxKind::COMMA);
        } else {
            builder.push_separator(SyntaxKind::COMMA);
        }
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
        let (open_edge, _) = self.edges();
        self.surround(
            fmt,
            fmt.token_without_cursor(self.open.clone(), open_edge),
            inner,
        )
    }

    /// `inner` between the delimiters, e.g. `(a + b)`, `<T as Trait>`, `{ x }`;
    /// the comments following the opening delimiter are placed in front of `inner`
    pub fn enclose(&self, fmt: &Formatter, inner: Frag) -> Frag {
        let (open_edge, _) = self.edges();
        self.surround(fmt, fmt.token(self.open.clone(), open_edge), inner)
    }

    fn surround(&self, fmt: &Formatter, open: Frag, inner: Frag) -> Frag {
        let (_, close_edge) = self.edges();
        let mut builder = Builder::new(fmt);
        builder.push(open);
        builder.push(inner);
        builder.push_token(self.close.clone(), close_edge);
        builder.into_frag()
    }

    /// The container without items or statements: `()`, `{}`, or only comments,
    pub fn format_empty(&self, fmt: &Formatter) -> Frag {
        // The container is empty, so only trivia is between the delimiters
        let inside = || trivia_from(self.open.next_token());
        if !inside().any(|t| t.kind() == SyntaxKind::COMMENT) {
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
        builder.flush_trivia();
        builder.push_newline();
        self.wrap_body(
            fmt,
            fmt.body_after_opener(line_comments, builder.into_frag()),
        )
    }

    /// An own-line comment after the opening delimiter allows only broken layouts
    fn has_own_line_comment(&self) -> bool {
        own_line_comment_follows(&self.open)
    }

    /// Whether a line comment is between the last item and the closing delimiter: `a, // c` / `)`
    fn has_line_comment_before_close(&self) -> bool {
        std::iter::successors(self.close.prev_token(), |t| t.prev_token())
            .take_while(|t| t.kind().is_trivia() || t.kind() == SyntaxKind::COMMA)
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

/// When the items may be put on a single line
#[derive(Clone, Copy)]
pub(crate) enum FlatLimit {
    /// Whenever the line fits within `max_width`
    MaxWidth,
    /// Only when the items between the delimiters are at most this wide (`fn_call_width`, ...)
    Within(u32),
}

impl FlatLimit {
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
/// are on the line of the opening delimiter, within the width of `limit` (as rustfmt does). The last
/// item has a comma only if it is `Required` (`(match a {` / `...` / `},)`).
fn overflow_layout(
    fmt: &Formatter,
    delims: &Delimiters,
    items: &[ListItem],
    last_layouts: &Frag,
    limit: FlatLimit,
    trailing: TrailingSeparator,
) -> Option<Frag> {
    let rest = &items[..items.len() - 1];
    let mut overflow = if rest.is_empty() {
        Builder::with_cursor(fmt, Some(delims.open.clone()))
    } else {
        let mut line = Builder::new(fmt);
        line.push(flat_layout(
            fmt,
            delims,
            rest,
            limit,
            TrailingSeparator::Required,
        )?);
        line
    };
    overflow.flush_trivia();
    overflow.push(last_layouts.clone());
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
            BrokenList::Fill => fill_list(fmt, items, open.clone()),
            BrokenList::Aligned => aligned_list(fmt, items, open.clone()),
        };
        // With a preferred overflow, the broken layouts are used only when it does not fit
        match (&overflow, last_overflow) {
            (Some(_), Overflow::Preferred(_) | Overflow::Only(_)) => fmt.dispreferred(broken),
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
