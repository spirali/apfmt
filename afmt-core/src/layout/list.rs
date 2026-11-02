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

/// An item of a comma-separated list with its source comma. The item is the formatted `Frag`, or
/// its parts when its layout is chosen later (see `ListItem::map`).
#[derive(Clone)]
pub(crate) struct ListItem<T = Frag> {
    pub frag: T,
    /// `None` for the last item without a trailing comma
    pub comma: Option<SyntaxToken>,
    /// Alone on its line in a FillList
    pub own_line: bool,
    pub overflow: Overflow,
}

/// How the last item of a list may overflow: it starts on the line of the opening delimiter and
/// ends on the line of the closing one (`f(a, |x| {` / `...` / `})`)
#[derive(Clone, Default)]
pub(crate) enum Overflow {
    #[default]
    Never,
    /// The list layout with fewer lines wins
    With(Frag),
    /// As `With`, and of layouts with as many lines the overflowing one wins (Style Guide: a single
    /// combinable argument, "apply this behavior recursively")
    WeakPreference(Frag),
    /// Preferred whenever it fits: the broken layouts of the list cost a split (Style Guide:
    /// closures)
    Preferred(Frag),
    /// The only broken layout of the list unless a comment rules it out (`|(` / `    a,` / `)|`)
    Only(Frag),
}

impl Overflow {
    pub fn layouts(&self) -> Option<&Frag> {
        match self {
            Overflow::Never => None,
            Overflow::With(layouts)
            | Overflow::WeakPreference(layouts)
            | Overflow::Preferred(layouts)
            | Overflow::Only(layouts) => Some(layouts),
        }
    }

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
    /// Without a comma, placed as any item and not overflowing
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

    pub fn own_line_if(mut self, own_line: bool) -> Self {
        self.own_line = own_line;
        self
    }

    pub fn with_overflow(mut self, overflow: Overflow) -> Self {
        self.overflow = overflow;
        self
    }
}

/// How the last item of a list may hug it (see `Overflow`)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Hug {
    Never,
    Allowed,
    WeakPreference,
    Preferred,
    /// The only broken layout of the list (`|(` / `    a,` / `)|`)
    Only,
}

impl Hug {
    pub fn overflow(self, overflow: &Overflow) -> Overflow {
        match (self, overflow) {
            (Hug::Never, _) => Overflow::Never,
            (Hug::Preferred, Overflow::With(frag)) => Overflow::Preferred(frag.clone()),
            (Hug::WeakPreference, Overflow::With(frag)) => Overflow::WeakPreference(frag.clone()),
            (Hug::Only, Overflow::With(frag)) => Overflow::Only(frag.clone()),
            (_, overflow) => overflow.clone(),
        }
    }
}

pub(crate) fn hug_last_item(items: &mut [ListItem], hug: Hug) {
    if let Some(item) = items.last_mut() {
        item.overflow = hug.overflow(&Overflow::With(item.frag.clone()));
    }
}

/// The items of the comma-separated list `node` with their commas. `format` formats a child node,
/// or returns `None` for children that are not items.
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
                // The parser rejects a doubled or missing comma
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

/// All items on one line, `a, b, c`, with a trailing comma only if it is `Required`. The caller
/// restricts it to a single line.
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
    builder.end_line_after_line_comment();
    builder.into_frag()
}

/// The comma after `item` if `comma`, otherwise its source comma is only consumed. The source
/// comma itself is printed, so comments in front of it stay there (`a /* c */, b`).
fn push_or_skip_comma(builder: &mut Builder, item: &ListItem, comma: bool) {
    match &item.comma {
        Some(source) if comma => builder.push_token(source.clone(), Comma),
        None if comma => builder.push_separator(SyntaxKind::COMMA),
        _ => builder.skip_separator(SyntaxKind::COMMA),
    }
}

/// Items filled on lines: `a, b, c,` / `d, e,`; items with `own_line` are alone on their line.
/// Comments are placed by rustfmt's rules (see CLAUDE.md).
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

/// One item per line, aligned after `open`, without a trailing comma: `|a: A,` / ` b: B|`. Only for
/// closure parameters, an exception to the Style Guide (see CLAUDE.md).
fn aligned_list(
    fmt: &Formatter,
    items: &[ListItem],
    open: Option<SyntaxToken>,
    keep_blank_lines: bool,
) -> Frag {
    fmt.align(aligned_items(fmt, items, open, keep_blank_lines))
}

fn aligned_items(
    fmt: &Formatter,
    items: &[ListItem],
    open: Option<SyntaxToken>,
    keep_blank_lines: bool,
) -> Frag {
    let mut builder = Builder::with_cursor(fmt, open);
    builder.flush_trivia();
    for (item, is_last) in with_is_last(items) {
        builder.push(item.frag.clone());
        if is_last {
            builder.skip_separator(SyntaxKind::COMMA);
            break;
        }
        builder.push_separator(SyntaxKind::COMMA);
        if keep_blank_lines {
            builder.flush_trivia_and_end_line();
        } else {
            builder.flush_trivia();
            builder.push_newline();
        }
    }
    builder.flush_trivia();
    builder.end_line_after_line_comment();
    builder.into_frag()
}

/// As `aligned_list`, followed by `tail` on a line of its own: `|a: A,` / ` b: B|` / ` -> T`
pub(crate) fn aligned_list_with_tail(
    fmt: &Formatter,
    delims: &Delimiters,
    items: &[ListItem],
    keep_blank_lines: bool,
    tail: Frag,
) -> Frag {
    let mut aligned = Builder::new(fmt);
    let open = Some(delims.open.clone());
    aligned.push(aligned_items(fmt, items, open, keep_blank_lines));
    aligned.push(delims.format_close(fmt));
    aligned.push_newline();
    aligned.push(tail);
    let mut builder = Builder::new(fmt);
    builder.push(delims.format_open(fmt));
    builder.push(fmt.align(aligned.into_frag()));
    builder.into_frag()
}

/// One item per line with a trailing comma (unless `Never` or `Omitted`), indented. With `lines`,
/// the lines have these numbers of items (see `ListLayout::lines_if`).
fn vertical_list(
    fmt: &Formatter,
    items: &[ListItem],
    open: Option<SyntaxToken>,
    keep_blank_lines: bool,
    trailing: TrailingSeparator,
    lines: Option<&[usize]>,
) -> Frag {
    let mut lines = lines.map(|lines| lines.iter().copied());
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
        if is_last {
            // Blank lines in front of the closing delimiter are dropped
            builder.flush_trivia();
            builder.push_newline();
        } else {
            builder.end_line_before_item(keep_blank_lines);
        }
    }
    fmt.body_after_opener(line_comments, builder.into_frag())
}

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
    Angles,
    Brackets,
    Pipes,
    /// `{ }`: a single-line list has spaces inside, comments of an empty list go on own lines
    Braces,
    /// `{ }` without spaces inside: `a::{b, c}`
    TightBraces,
}

impl DelimiterKind {
    fn tokens(self) -> (SyntaxKind, SyntaxKind) {
        match self {
            DelimiterKind::Parens => (SyntaxKind::L_PAREN, SyntaxKind::R_PAREN),
            DelimiterKind::Angles => (SyntaxKind::L_ANGLE, SyntaxKind::R_ANGLE),
            DelimiterKind::Brackets => (SyntaxKind::L_BRACK, SyntaxKind::R_BRACK),
            DelimiterKind::Pipes => (SyntaxKind::PIPE, SyntaxKind::PIPE),
            DelimiterKind::Braces | DelimiterKind::TightBraces => {
                (SyntaxKind::L_CURLY, SyntaxKind::R_CURLY)
            }
        }
    }
}

impl Delimiters {
    pub fn parens_of(node: &SyntaxNode) -> Self {
        Self::of(node, DelimiterKind::Parens)
    }

    pub fn try_parens_of(node: &SyntaxNode) -> Option<Self> {
        Self::try_of(node, DelimiterKind::Parens)
    }

    pub fn angles_of(node: &SyntaxNode) -> Self {
        Self::of(node, DelimiterKind::Angles)
    }

    pub fn brackets_of(node: &SyntaxNode) -> Self {
        Self::of(node, DelimiterKind::Brackets)
    }

    pub fn pipes_of(node: &SyntaxNode) -> Self {
        Self::of(node, DelimiterKind::Pipes)
    }

    pub fn braces_of(node: &SyntaxNode) -> Self {
        Self::of(node, DelimiterKind::Braces)
    }

    pub fn tight_braces_of(node: &SyntaxNode) -> Self {
        Self::of(node, DelimiterKind::TightBraces)
    }

    fn of(node: &SyntaxNode, kind: DelimiterKind) -> Self {
        Self::try_of(node, kind).unwrap()
    }

    fn try_of(node: &SyntaxNode, kind: DelimiterKind) -> Option<Self> {
        let (open, close) = kind.tokens();
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

    /// The source between the delimiters, reindented: `{ ... }` spaced as a block, `(...)` and
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

    /// `inner` between the delimiters; `inner` places the comments following the opening delimiter
    pub fn wrap_body(&self, fmt: &Formatter, inner: Frag) -> Frag {
        self.surround(fmt, self.format_open(fmt), inner)
    }

    /// `inner` between the delimiters, e.g. `(a + b)`. When comments put `inner` on its own line,
    /// the delimiters are broken as those of a block: `(` / `    // c` / `    a && b` / `)`.
    pub fn enclose(&self, fmt: &Formatter, inner: Frag) -> Frag {
        let (open_edge, _) = self.edges();
        if !self.comments_break_body() {
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

    /// Comments put the body on lines of its own: `(` / `    // c` / `    a` / `)`
    pub fn comments_break_body(&self) -> bool {
        comments_break_line_after(&self.open)
            || prev_code_token(&self.close).is_some_and(|t| own_line_comment_follows(&t))
    }

    fn format_open(&self, fmt: &Formatter) -> Frag {
        fmt.token_without_cursor(self.open.clone(), self.edges().0)
    }

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

    /// `()`, `{}`, or only comments between the delimiters
    pub fn format_empty(&self, fmt: &Formatter) -> Frag {
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

    /// `{`, then `lines` block indented, then `}`. Own-line comments at the indentation of `}` in
    /// front of `} else` stay there (`comments_at_closing_indentation`).
    pub fn format_block(&self, fmt: &Formatter, lines: impl IntoIterator<Item = Frag>) -> Frag {
        self.format_lines(fmt, lines, false)
    }

    /// As `format_block`, for lines that are items of a list: a block comment followed by the next
    /// item on its line stays in front of it, as in `vertical_list`
    pub fn format_list_block(
        &self,
        fmt: &Formatter,
        lines: impl IntoIterator<Item = Frag>,
    ) -> Frag {
        self.format_lines(fmt, lines, true)
    }

    fn format_lines(
        &self,
        fmt: &Formatter,
        lines: impl IntoIterator<Item = Frag>,
        items: bool,
    ) -> Frag {
        let mut lines = lines.into_iter().peekable();
        if lines.peek().is_none() {
            return self.format_empty(fmt);
        }
        // Tokens following `{` belong to the body, so they are nested with it
        let mut builder = Builder::with_cursor(fmt, Some(self.open.clone()));
        let line_comments = builder.push_comments_ending_line();
        builder.push_comments();
        for line in lines {
            if items && !builder.is_empty() {
                builder.end_line_before_item(true);
                builder.push(line);
            } else {
                builder.push_on_new_line(line);
            }
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

    fn has_own_line_comment(&self) -> bool {
        own_line_comment_follows(&self.open)
    }

    fn has_line_comment_before_close(&self) -> bool {
        std::iter::successors(self.close.prev_token(), |t| t.prev_token())
            .take_while(|t| !is_code_kind(t.kind()) || t.kind() == SyntaxKind::COMMA)
            .any(|t| is_line_comment(&t))
    }
}

/// The layouts a delimited list may take. Start with `tall` or `fixed_vertical` and refine it with
/// `fill_if`, `broken_if`, `keep_blank_lines` and `trailing_separator`.
pub(crate) struct ListLayout {
    /// `None` allows only the broken layout
    flat: Option<FlatLimit>,
    broken: BrokenList,
    /// Blank lines between items are kept when one per line; never on a single line or filled
    keep_blank_lines: bool,
    trailing: TrailingSeparator,
    lines: Option<Vec<usize>>,
}

impl ListLayout {
    pub fn tall(flat: FlatLimit) -> Self {
        ListLayout {
            flat: Some(flat),
            broken: BrokenList::Vertical,
            keep_blank_lines: false,
            trailing: TrailingSeparator::IfVertical,
            lines: None,
        }
    }

    /// Blank lines between items are kept (struct fields, enum variants)
    pub fn fixed_vertical() -> Self {
        ListLayout {
            flat: None,
            broken: BrokenList::Vertical,
            keep_blank_lines: true,
            trailing: TrailingSeparator::IfVertical,
            lines: None,
        }
    }

    pub fn aligned(mut self) -> Self {
        self.broken = BrokenList::Aligned;
        self
    }

    pub fn fill_if(mut self, fill: bool) -> Self {
        if fill {
            self.broken = BrokenList::Fill;
        }
        self
    }

    /// Kept for declarations (function parameters), as rustfmt does
    pub fn keep_blank_lines(mut self) -> Self {
        self.keep_blank_lines = true;
        self
    }

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

    /// The list may also be broken with these numbers of items on its lines, each line flat, e.g.
    /// `x, y,` / `"{} {}",` / `a, b` in a format macro; no comments between the items
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrailingSeparator {
    IfVertical,
    /// Also on a single line: the tuple pattern `(a,)` is not the parenthesized pattern `(a)`
    Required,
    /// In a vertical layout, the token after the list follows the last item: `    T: Clone;`
    Never,
    /// No item may follow the last one; the closing delimiter is on its own line: `    ..` / `}`
    Omitted,
}

impl TrailingSeparator {
    fn comma_when_broken(self) -> bool {
        matches!(
            self,
            TrailingSeparator::IfVertical | TrailingSeparator::Required
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) enum FlatLimit {
    MaxWidth,
    Within(u32),
}

impl FlatLimit {
    /// `width` (`fn_call_width`, ...), but only `max_width` for a single item that cannot hug the
    /// list or span more lines (`f("...")`): broken, it would only move to its own line
    pub fn of_arguments(fmt: &Formatter, items: &[ListItem], hug: Hug, width: u32) -> Self {
        match items {
            [item] if hug == Hug::Never || !fmt.has_line_break(&item.frag) => FlatLimit::MaxWidth,
            _ => FlatLimit::Within(width),
        }
    }

    pub fn apply(self, fmt: &Formatter, frag: Frag) -> Option<Frag> {
        match self {
            FlatLimit::MaxWidth => fmt.flat_if_possible(frag),
            FlatLimit::Within(width) => fmt.flat_within(frag, width),
        }
    }
}

#[derive(Clone, Copy)]
enum BrokenList {
    Vertical,
    Fill,
    Aligned,
}

/// The items on a single line, without the delimiters, unless an own-line comment follows the
/// opening delimiter; wrap it with `Delimiters::wrap_body`
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

/// Items after a keyword, one per line and indented: `where` / `    T: Clone,`. The next token
/// starts a new line, unless `trailing` is `Never` (`    T: Clone;`).
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

/// The last item overflows with `last_layouts`; the items in front of it and its first line fit
/// `limit`, and none spans more lines on its single line (Style Guide).
fn overflow_layout(
    fmt: &Formatter,
    delims: &Delimiters,
    items: &[ListItem],
    last_layouts: &Frag,
    limit: FlatLimit,
    trailing: TrailingSeparator,
) -> Option<Frag> {
    let rest = &items[..items.len() - 1];
    if rest.iter().any(|item| fmt.has_multi_line_text(&item.frag)) {
        return None;
    }
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
                // A layout that costs a split (`|x| match x` / `{`) is not squeezed into the width,
                // as in rustfmt
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

pub(crate) fn delimited_list(
    fmt: &Formatter,
    delims: &Delimiters,
    items: &[ListItem],
    layout: ListLayout,
) -> Frag {
    if items.is_empty() {
        return delims.format_empty(fmt);
    }
    // Limits the items in front of an overflowing last item, also when the list has no single line
    let overflow_limit = layout.flat.unwrap_or(FlatLimit::MaxWidth);
    // A string spanning lines is not on a single line (Style Guide, as in rustfmt)
    let layout = layout.broken_if(items.iter().any(|item| fmt.has_multi_line_text(&item.frag)));
    let open = Some(delims.open.clone());
    let last_overflow = &items.last().unwrap().overflow;
    // A line comment after the last item would end the line in front of the closing delimiter
    let overflow = (last_overflow.layouts())
        .filter(|_| !delims.has_own_line_comment() && !delims.has_line_comment_before_close())
        .and_then(|last_layouts| {
            let overflow = overflow_layout(
                fmt,
                delims,
                items,
                last_layouts,
                overflow_limit,
                layout.trailing,
            )?;
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
            BrokenList::Vertical => vertical_list(
                fmt,
                items,
                open.clone(),
                layout.keep_blank_lines,
                layout.trailing,
                None,
            ),
            BrokenList::Fill => fill_list(fmt, items, open.clone(), layout.trailing),
            BrokenList::Aligned => aligned_list(fmt, items, open.clone(), layout.keep_blank_lines),
        };
        // With a preferred overflow, the broken layouts are used only when it does not fit
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
