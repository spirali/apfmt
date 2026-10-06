//! Delimited, comma-separated lists

use crate::formatter::Edge::{BraceClose, BraceOpen, Close, Comma, Open, Word};
use crate::formatter::{Builder, Edge, Formatter, Frag};
use crate::comments::{own_line_comment_follows, trivia_from};
use crate::utils::with_is_last;
use ra_ap_syntax::{AstNode, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

/// An item of a comma-separated list with the source comma following it. The item is usually
/// the formatted `Frag`, or its parts when its layout is chosen later (see `ListItem::map`).
#[derive(Clone)]
pub(crate) struct ListItem<T = Frag> {
    pub frag: T,
    /// `None` for the last item without a trailing comma
    pub comma: Option<SyntaxToken>,
}

impl<T> ListItem<T> {
    /// The item formatted by `format`, with the same comma
    pub fn map(&self, format: impl FnOnce(&T) -> Frag) -> ListItem {
        ListItem {
            frag: format(&self.frag),
            comma: self.comma.clone(),
        }
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
                    items.push(ListItem { frag, comma: None });
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
    builder.flush_trivia(false);
    for (item, is_last) in with_is_last(items) {
        builder.push(item.frag.clone());
        let comma = !is_last || trailing == TrailingSeparator::Required;
        match &item.comma {
            // The source separator, so comments in front of it stay there: `a /* c */, b`
            Some(source) if comma => builder.push_token(source.clone(), Comma),
            // A required trailing comma missing in the source
            None if comma => builder.push_delimiter(SyntaxKind::COMMA),
            // No trailing comma, but the source one and its comments still have to be consumed
            _ => builder.skip_delimiter(SyntaxKind::COMMA),
        }
    }
    builder.flush_trivia(false);
    // A line comment ends the line inside the list, so `Flat` rejects such a layout
    builder.end_line_after_line_comment();
    builder.into_frag()
}

/// Items filled on lines, as many on each line as fit: `a, b, c,` / `d, e,`; with a line break after `open`,
/// indented, and a trailing comma. Comments are placed like rustfmt does: a comment before the comma or
/// ending the line belongs to the item and ends the line, a comment followed by an item on the same line
/// stays in front of it, and an own-line comment gets its own line. Comments ending the line of `open`
/// stay there, as in vertical lists.
fn fill_list(fmt: &Formatter, items: &[ListItem], open: Option<SyntaxToken>) -> Frag {
    let mut builder = Builder::with_cursor(fmt, open);
    let line_comments = builder.push_comments_ending_line();
    for (item, is_last) in with_is_last(items) {
        builder.push_leading_comments();
        builder.push(item.frag.clone());
        let had_post_comment = builder.push_separator_with_trailing_comments(SyntaxKind::COMMA);
        if is_last {
            break;
        }
        if had_post_comment || builder.cursor().is_some_and(own_line_comment_follows) {
            builder.push_newline();
        } else {
            builder.push_soft_line_break();
        }
    }
    builder.flush_trivia(false);
    builder.push_newline();
    fmt.body_after_opener(line_comments, builder.into_frag())
}

/// One item per line with a trailing comma (unless it is `Never`), indented; `open` is the opening
/// delimiter and the comments following it are part of the list. With `keep_blank_lines`, blank lines between items are kept
/// (up to `blank_lines_upper_bound`), e.g. between struct fields.
fn vertical_list(
    fmt: &Formatter,
    items: &[ListItem],
    open: Option<SyntaxToken>,
    keep_blank_lines: bool,
    trailing: TrailingSeparator,
) -> Frag {
    let mut builder = Builder::with_cursor(fmt, open);
    let line_comments = builder.push_comments_ending_line();
    builder.push_leading_comments();
    for (item, is_last) in with_is_last(items) {
        builder.push(item.frag.clone());
        if is_last && trailing == TrailingSeparator::Never {
            // The token after the list follows the last item on its line: `T: Clone;`
            builder.skip_delimiter(SyntaxKind::COMMA);
            break;
        }
        builder.push_delimiter(SyntaxKind::COMMA);
        // Blank lines in front of the closing delimiter are dropped
        if keep_blank_lines && !is_last {
            builder.ensure_newline();
        } else {
            builder.flush_trivia(false);
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
    /// `{ }`: a single-line list has spaces inside, comments of an empty list go on their own
    /// indented lines
    Braces,
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

    /// The `<` and `>` of the list `node` (generic arguments, `<T as Trait>`)
    pub fn angles_of(node: &SyntaxNode) -> Self {
        Self::of(
            node,
            SyntaxKind::L_ANGLE,
            SyntaxKind::R_ANGLE,
            DelimiterKind::Angles,
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

    /// The first `open` and the last `close` token among the children of `node`
    fn of(node: &SyntaxNode, open: SyntaxKind, close: SyntaxKind, kind: DelimiterKind) -> Self {
        let token = |kind: SyntaxKind| {
            node.children_with_tokens()
                .filter_map(SyntaxElement::into_token)
                .filter(move |t| t.kind() == kind)
        };
        Delimiters {
            open: token(open).next().unwrap(),
            close: token(close).last().unwrap(),
            kind,
        }
    }

    fn edges(&self) -> (Edge, Edge) {
        match self.kind {
            DelimiterKind::Parens | DelimiterKind::Angles => (Open, Close),
            DelimiterKind::Braces => (BraceOpen, BraceClose),
        }
    }

    /// `inner` between the delimiters; `inner` places the comments following the opening delimiter, as
    /// the list layouts do (they start with the opening delimiter as their cursor)
    pub fn wrap(&self, fmt: &Formatter, inner: Frag) -> Frag {
        let (open_edge, _) = self.edges();
        self.surround(fmt, fmt.token_without_cursor(self.open.clone(), open_edge), inner)
    }

    /// `inner` between the delimiters, e.g. `(a + b)`, `<T as Trait>`, `{ x }` (taxonomy: Enclosed,
    /// Braced); the comments following the opening delimiter are placed in front of `inner`
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
    pub fn empty(&self, fmt: &Formatter) -> Frag {
        // The container is empty, so only trivia is between the delimiters
        let inside = || trivia_from(self.open.next_token());
        if !inside().any(|t| t.kind() == SyntaxKind::COMMENT) {
            return self.wrap(fmt, Frag::empty());
        }
        if !inside().any(|t| t.text().contains('\n')) {
            let mut comments = Builder::with_cursor(fmt, Some(self.open.clone()));
            comments.flush_trivia(false);
            let comments = comments.into_frag();
            let mut own_line = Builder::new(fmt);
            own_line.push(comments.clone());
            own_line.push_newline();
            // On their own line only if they do not fit on the delimiters' line
            let inline = fmt.flat(self.wrap(fmt, comments));
            let own_line = self.wrap(fmt, fmt.body_after_opener(false, own_line.into_frag()));
            return fmt.alt_frag(inline, own_line);
        }
        let mut body = Builder::with_cursor(fmt, Some(self.open.clone()));
        let line_comments = body.push_comments_ending_line();
        body.flush_trivia(false);
        body.push_newline();
        self.wrap(fmt, fmt.body_after_opener(line_comments, body.into_frag()))
    }

    /// An own-line comment after the opening delimiter allows only broken layouts
    fn has_own_line_comment(&self) -> bool {
        own_line_comment_follows(&self.open)
    }
}

/// The layouts a delimited list may take, see `delimited_list`. Start with `tall` or `fixed_vertical`
/// and refine it with `fill_if`, `hug_if` and `trailing_separator`.
pub(crate) struct ListLayout {
    /// The items on a single line; `None` allows only the broken layout
    flat: Option<FlatLimit>,
    broken: BrokenList,
    /// The only item may overflow: it starts on the line of the opening delimiter and ends on the line
    /// of the closing one (`f(if a {` / `...` / `})`); it has to be a combinable expression
    hug: bool,
    trailing: TrailingSeparator,
}

impl ListLayout {
    /// On a single line if it fits, otherwise one item per line
    pub fn tall(flat: FlatLimit) -> Self {
        ListLayout {
            flat: Some(flat),
            broken: BrokenList::Vertical {
                keep_blank_lines: false,
            },
            hug: false,
            trailing: TrailingSeparator::IfVertical,
        }
    }

    /// Always one item per line, blank lines between items are kept (struct fields, enum variants)
    pub fn fixed_vertical() -> Self {
        ListLayout {
            flat: None,
            broken: BrokenList::Vertical {
                keep_blank_lines: true,
            },
            hug: false,
            trailing: TrailingSeparator::IfVertical,
        }
    }

    /// If `fill`, the broken layout has as many items per line as fit instead of one per line
    pub fn fill_if(mut self, fill: bool) -> Self {
        if fill {
            self.broken = BrokenList::Fill;
        }
        self
    }

    /// If `hug`, the only item may overflow (see `ListLayout::hug`)
    pub fn hug_if(mut self, hug: bool) -> Self {
        self.hug = hug;
        self
    }

    pub fn trailing_separator(mut self, trailing: TrailingSeparator) -> Self {
        self.trailing = trailing;
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
}

/// When the items may be put on a single line
#[derive(Clone, Copy)]
pub(crate) enum FlatLimit {
    /// Whenever the line fits within `max_width`
    MaxWidth,
    /// Only when the items between the delimiters are at most this wide (`fn_call_width`, ...)
    Within(u32),
}

/// The layout of a list that does not fit on a single line
#[derive(Clone, Copy)]
enum BrokenList {
    /// One item per line, see `vertical_list`
    Vertical { keep_blank_lines: bool },
    /// As many items per line as fit, see `fill_list`
    Fill,
}

/// The items on a single line, without the delimiters, if `limit` allows it and no own-line comment
/// follows the opening delimiter; wrap it with `Delimiters::wrap`
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
    match limit {
        FlatLimit::MaxWidth => Some(fmt.flat(flat)),
        FlatLimit::Within(width) => fmt.flat_within(flat, width),
    }
}

/// Items following a keyword, always one per line and indented: `where` / `    T: Clone,` (taxonomy:
/// WhereClause). The list ends with a line break, so the next token starts a new line (`{`), unless
/// `trailing` is `Never`: then it follows the last item, which has no comma (`    T: Clone;`).
pub(crate) fn keyword_list(
    fmt: &Formatter,
    keyword: SyntaxToken,
    items: &[ListItem],
    trailing: TrailingSeparator,
) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(fmt.token_without_cursor(keyword.clone(), Word));
    builder.push(vertical_list(fmt, items, Some(keyword), false, trailing));
    builder.into_frag()
}

/// A delimited list, `(a, b)`, `<A, B>` or `{ a, b }`, in the layouts `layout` allows
pub(crate) fn delimited_list(
    fmt: &Formatter,
    delims: &Delimiters,
    items: &[ListItem],
    layout: ListLayout,
) -> Frag {
    if items.is_empty() {
        return delims.empty(fmt);
    }
    let open = Some(delims.open.clone());
    let mut layouts = match layout.broken {
        BrokenList::Vertical { keep_blank_lines } => {
            vertical_list(fmt, items, open.clone(), keep_blank_lines, layout.trailing)
        }
        BrokenList::Fill => fill_list(fmt, items, open.clone()),
    };
    if let Some(limit) = layout.flat {
        layouts = fmt.alt_opt(
            layouts,
            flat_layout(fmt, delims, items, limit, layout.trailing),
        );
    }
    if layout.hug && !delims.has_own_line_comment() {
        let [item] = items else {
            panic!("only a single item can overflow");
        };
        let mut overflow = Builder::with_cursor(fmt, open);
        overflow.flush_trivia(false);
        overflow.push(item.frag.clone());
        overflow.skip_delimiter(SyntaxKind::COMMA);
        overflow.flush_trivia(false);
        // Only an item that is broken overflows; on a single line, it is the flat layout
        layouts = fmt.alt_frag(layouts, fmt.multi_line(overflow.into_frag()));
    }
    delims.wrap(fmt, layouts)
}
