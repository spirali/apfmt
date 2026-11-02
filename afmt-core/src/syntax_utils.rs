//! Helpers on the syntax tree of `ra_ap_syntax`

use rustc_hash::FxHashMap;

use crate::comments::{is_code_kind, line_indentation};
use ra_ap_syntax::ast::{self, HasAttrs, HasModuleItem};
use ra_ap_syntax::{
    AstNode, Edition, NodeOrToken, SourceFile, SyntaxKind, SyntaxNode, SyntaxToken, T, TextSize,
};

pub(crate) fn has_attrs(node: &impl HasAttrs) -> bool {
    node.attrs().next().is_some()
}

/// `#[rustfmt::skip]` among its attributes
pub(crate) fn has_rustfmt_skip(node: &impl HasAttrs) -> bool {
    node.attrs().any(|attr| is_rustfmt_skip(&attr))
}

fn is_rustfmt_skip(attr: &ast::Attr) -> bool {
    let Some(ast::Meta::PathMeta(meta)) = attr.meta() else {
        return false;
    };
    meta.path().is_some_and(|path| path.syntax().text() == "rustfmt::skip")
}

/// `#[rustfmt::skip]` on it or on a node containing it: its source is not to be changed
pub(crate) fn is_skipped(node: &SyntaxNode) -> bool {
    node.ancestors()
        .filter_map(ast::AnyHasAttrs::cast)
        .any(|node| has_rustfmt_skip(&node))
}

pub(crate) fn macro_token_tree(expr: &ast::Expr) -> Option<ast::TokenTree> {
    let ast::Expr::MacroExpr(expr) = expr else {
        return None;
    };
    expr.macro_call()?.token_tree()
}

/// `m! { ... }`: copied; as the only content of a block it is a statement (it may expand to
/// statements), so the block cannot be removed
pub(crate) fn is_brace_macro(expr: &ast::Expr) -> bool {
    macro_token_tree(expr).is_some_and(|tree| tree.l_curly_token().is_some())
}

/// Ends its block with `;` (`trailing_semicolon`)
pub(crate) fn is_jump_expr(node: &SyntaxNode) -> bool {
    matches!(
        node.kind(),
        SyntaxKind::RETURN_EXPR | SyntaxKind::BREAK_EXPR | SyntaxKind::CONTINUE_EXPR
    )
}

/// `return;`, `break;` or `continue;` ending a block, whose `;` follows `trailing_semicolon`
pub(crate) fn trailing_jump_stmt(stmt_list: &ast::StmtList) -> Option<ast::ExprStmt> {
    if stmt_list.tail_expr().is_some() {
        return None;
    }
    match stmt_list.statements().last()? {
        ast::Stmt::ExprStmt(stmt) if stmt.expr().is_some_and(|e| is_jump_expr(e.syntax())) => {
            Some(stmt)
        }
        _ => None,
    }
}

/// A stray `;` in a block (`;;`), removed by `rewrite/empty_stmts.rs`
pub(crate) fn is_stray_semicolon(token: &SyntaxToken) -> bool {
    token.kind() == SyntaxKind::SEMICOLON
        && token
            .parent()
            .is_some_and(|p| p.kind() == SyntaxKind::STMT_LIST)
}

/// Tokens of a reparsed tree written as other text: in a `macro_rules!` transcriber, `_x` for `$x`
pub(crate) type Placeholders = FxHashMap<SyntaxToken, String>;

pub(crate) struct Reparsed<N> {
    pub node: N,
    pub placeholders: Vec<(SyntaxToken, String)>,
}

impl<N> Reparsed<N> {
    pub fn map<M>(self, f: impl FnOnce(N) -> M) -> Reparsed<M> {
        Reparsed {
            node: f(self.node),
            placeholders: self.placeholders,
        }
    }
}

impl<N: AstNode> Reparsed<N> {
    pub fn into_syntax(self) -> Reparsed<SyntaxNode> {
        self.map(|node| node.syntax().clone())
    }
}

/// The arguments of a macro call parsed as the formatter lays them out
pub(crate) enum MacroArgs {
    List(ast::ArgList),
    Array(ast::ArrayExpr),
}

impl MacroArgs {
    pub fn into_syntax(self) -> SyntaxNode {
        match self {
            MacroArgs::List(args) => args.syntax().clone(),
            MacroArgs::Array(array) => array.syntax().clone(),
        }
    }
}

/// `(...)` as call arguments, `[...]` as an array; `None` for braces or arguments that do not parse
pub(crate) fn parse_macro_args(
    tree: &ast::TokenTree,
    edition: Edition,
    placeholders: &Placeholders,
) -> Option<Reparsed<MacroArgs>> {
    match tree.syntax().first_token()?.kind() {
        SyntaxKind::L_PAREN => parse_token_tree::<ast::ArgList>(tree, edition, placeholders)
            .map(|parsed| parsed.map(MacroArgs::List)),
        SyntaxKind::L_BRACK => parse_token_tree::<ast::ArrayExpr>(tree, edition, placeholders)
            .map(|parsed| parsed.map(MacroArgs::Array)),
        _ => None,
    }
}

struct SubstitutedText {
    text: String,
    /// The offset and length of each placeholder in `text`, with the text it stands for
    placeholders: Vec<(usize, usize, String)>,
}

/// The text of `tree`; with `specifiers` (a `macro_rules!` transcriber), `$name` is replaced by a
/// same-width token valid for its fragment, a single-line repetition by `_rep`
fn substituted_text(
    tree: &ast::TokenTree,
    specifiers: Option<&FxHashMap<String, String>>,
    placeholders: &Placeholders,
) -> Option<SubstitutedText> {
    let mut text = String::new();
    let mut substituted = Vec::new();
    let tokens: Vec<_> = (tree.syntax().descendants_with_tokens())
        .filter_map(|e| e.into_token())
        .collect();
    let mut i = 0;
    while let Some(token) = tokens.get(i) {
        i += 1;
        if let Some(specifiers) = specifiers.filter(|_| token.kind() == SyntaxKind::DOLLAR) {
            let next = tokens.get(i)?;
            if next.kind() == T!['('] {
                let end = repetition_end(&tokens, i)?;
                let source: String = tokens[i - 1..=end].iter().map(|t| t.text()).collect();
                if source.contains('\n') {
                    return None;
                }
                let placeholder = "_rep";
                substituted.push((text.len(), placeholder.len(), source));
                text.push_str(placeholder);
                i = end + 1;
                continue;
            }
            let name = Some(next).filter(|n| n.kind().is_any_identifier())?;
            i += 1;
            let placeholder = match specifiers.get(name.text()).map(String::as_str) {
                Some("lifetime") => format!("'{}", name.text()),
                Some("vis") => "pub".to_string(),
                _ => format!("_{}", name.text()),
            };
            let source = format!("${}", name.text());
            substituted.push((text.len(), placeholder.len(), source));
            text.push_str(&placeholder);
            continue;
        }
        if let Some(source) = placeholders.get(token) {
            substituted.push((text.len(), token.text().len(), source.clone()));
        }
        text.push_str(token.text());
    }
    Some(SubstitutedText {
        text,
        placeholders: substituted,
    })
}

/// The index of the operator `*`, `+` or `?` of the repetition whose `(` is at `open`
fn repetition_end(tokens: &[SyntaxToken], open: usize) -> Option<usize> {
    let group = tokens[open].parent()?;
    let is_close = |t: &SyntaxToken| t.kind() == T![')'] && t.parent().as_ref() == Some(&group);
    let close = open + tokens[open..].iter().position(is_close)?;
    let is_op = |t: &SyntaxToken| matches!(t.kind(), T![*] | T![+] | T![?]);
    let after = |i: usize| (i..tokens.len()).find(|&i| is_code_kind(tokens[i].kind()));
    let first = after(close + 1)?;
    if is_op(&tokens[first]) {
        return Some(first);
    }
    let op = after(first + 1)?;
    is_op(&tokens[op]).then_some(op)
}

/// Whether formatting could join or split adjacent punctuation, which a procedural macro may read
/// as one operator (`& &x` → `&&x`); commas, semicolons, `?` and `_` may change
fn changes_punct_spacing(node: &SyntaxNode) -> bool {
    // A nested token tree is checked when it is parsed itself
    let is_punct = |t: &SyntaxToken| {
        t.kind().is_punct()
            && t.parent()
                .is_none_or(|p| p.kind() != SyntaxKind::TOKEN_TREE)
            && !matches!(
                t.kind(),
                T!['('] | T![')'] | T!['['] | T![']'] | T!['{'] | T!['}'] | T![,] | T![;] | T![_]
            )
    };
    let tokens = node
        .descendants_with_tokens()
        .filter_map(|e| e.into_token());
    tokens
        .filter(|t| is_punct(t) && t.kind() != T![?])
        .any(|first| {
            let Some(next) = first.next_token() else {
                return false;
            };
            let joined = next.kind() != SyntaxKind::WHITESPACE;
            let second = if joined {
                Some(next)
            } else {
                next.next_token()
            };
            second.is_some_and(|second| {
                is_punct(&second)
                    && node.text_range().contains_range(second.text_range())
                    && joined != is_printed_joined(&first, &second)
            })
        })
}

/// Whether afmt prints `first` and `second` without whitespace between them: prefix operators, ref
/// patterns, ranges, paths, and angle brackets (`!!x`, `|&x|`, `..=-1`, `a::*`, `>>`)
fn is_printed_joined(first: &SyntaxToken, second: &SyntaxToken) -> bool {
    use SyntaxKind::{PARAM_LIST, PREFIX_EXPR, PTR_TYPE, REF_EXPR, REF_PAT, REF_TYPE};
    let parent = first.parent();
    let opens_parent = || parent.as_ref().and_then(|p| p.first_token()).as_ref() == Some(first);
    match (first.kind(), parent.as_ref().map(|p| p.kind())) {
        (_, Some(PREFIX_EXPR | REF_EXPR | REF_PAT | REF_TYPE | PTR_TYPE))
        | (T![|], Some(PARAM_LIST)) => opens_parent(),
        (T![..] | T![..=] | T![...], Some(SyntaxKind::RANGE_EXPR | SyntaxKind::RANGE_PAT)) => true,
        (T![::], _) => matches!(second.kind(), T![<] | T![*]),
        (T![<], Some(kind)) => kind == SyntaxKind::GENERIC_PARAM_LIST || is_angle_list(kind),
        (T![>], Some(kind)) => {
            is_angle_list(kind)
                && matches!(
                    second.kind(),
                    T![>] | T![::] | T![|] | T![:] | T![.] | T![?] | T![!]
                )
        }
        _ => false,
    }
}

/// `<` and `>` as angle brackets (`Vec<T>`), not comparisons; item generic parameters are followed
/// by a space (`impl<T> ::a::B`)
fn is_angle_list(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::GENERIC_ARG_LIST | SyntaxKind::TYPE_ANCHOR | SyntaxKind::PATH_SEGMENT
    )
}

/// `a < b > c`, `a == b == c`: rust-analyzer parses it, rustc rejects it
fn has_chained_comparison(node: &SyntaxNode) -> bool {
    let is_comparison =
        |expr: &ast::BinExpr| matches!(expr.op_kind(), Some(ast::BinaryOp::CmpOp(_)));
    node.descendants()
        .filter_map(ast::BinExpr::cast)
        .filter(is_comparison)
        .any(|expr| {
            (expr.lhs().into_iter().chain(expr.rhs())).any(|operand| match operand {
                ast::Expr::BinExpr(operand) => is_comparison(&operand),
                _ => false,
            })
        })
}

/// `substituted` parsed between `prefix` and `suffix` as the node `pick` finds; `None` on syntax
/// errors, punctuation spacing changes, or if the node is not exactly the text.
fn reparse<N: AstNode>(
    tree: &ast::TokenTree,
    substituted: &SubstitutedText,
    (prefix, suffix): (&str, &str),
    edition: Edition,
    pick: impl FnOnce(SourceFile) -> Option<N>,
) -> Option<Reparsed<N>> {
    let text = &substituted.text;
    let prefix = format!(
        "{}{prefix}",
        line_indentation(&tree.syntax().first_token()?)
    );
    let file = SourceFile::parse(&format!("{prefix}{text}{suffix}"), edition);
    let root = file.tree().syntax().clone();
    if !file.errors().is_empty() || has_chained_comparison(&root) {
        return None;
    }
    let node = pick(file.tree())?;
    if node.syntax().text() != text.as_str() || changes_punct_spacing(node.syntax()) {
        return None;
    }
    let placeholders = (substituted.placeholders.iter())
        .map(|(offset, len, source)| {
            let start = TextSize::from((prefix.len() + offset) as u32);
            let token = root.token_at_offset(start).right_biased()?;
            // Not merged with a neighbour: `$a$b` would be the identifier `_a_b`
            let range = token.text_range();
            (range.start() == start && range.len() == TextSize::from(*len as u32))
                .then(|| (token, source.clone()))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Reparsed { node, placeholders })
}

/// The token tree parsed as `N` (`(...)` as call arguments, `[...]` as an array); `None` if it does
/// not parse as exactly that node
pub(crate) fn parse_token_tree<N: AstNode>(
    tree: &ast::TokenTree,
    edition: Edition,
    placeholders: &Placeholders,
) -> Option<Reparsed<N>> {
    let substituted = substituted_text(tree, None, placeholders)?;
    let prefix = if tree.l_paren_token().is_some() {
        "const _: () = __afmt"
    } else {
        "const _: () = "
    };
    reparse(tree, &substituted, (prefix, ";"), edition, |file| {
        let Some(ast::Item::Const(item)) = file.items().next() else {
            return None;
        };
        item.body()?.syntax().descendants().find_map(N::cast)
    })
}

/// The transcriber of `rule` parsed as items, else as a function body, with placeholders for its
/// metavariables; `None` for `()`/`[]`, a repetition, or if it does not parse
pub(crate) fn parse_transcriber(
    rule: &MacroRule,
    edition: Edition,
    placeholders: &Placeholders,
) -> Option<Reparsed<SyntaxNode>> {
    let tree = &rule.transcriber;
    tree.l_curly_token()?;
    let specifiers = fragment_specifiers(&rule.matcher);
    let substituted = substituted_text(tree, Some(&specifiers), placeholders)?;
    let items = reparse(
        tree,
        &substituted,
        ("mod __afmt ", ""),
        edition,
        |file| match file.items().next()? {
            ast::Item::Module(item) => item.item_list(),
            _ => None,
        },
    );
    let body = || {
        reparse(
            tree,
            &substituted,
            ("fn __afmt() ", ""),
            edition,
            |file| match file.items().next()? {
                ast::Item::Fn(item) => item.body(),
                _ => None,
            },
        )
    };
    (items.map(Reparsed::into_syntax)).or_else(|| body().map(Reparsed::into_syntax))
}

/// `(matcher) => { transcriber }`; `=>` is two tokens in a token tree
pub(crate) struct MacroRule {
    pub matcher: ast::TokenTree,
    pub arrow: (SyntaxToken, SyntaxToken),
    pub transcriber: ast::TokenTree,
}

/// Separated by `;`; `None` when the token tree is not such a list
pub(crate) fn macro_rules_rules(tree: &ast::TokenTree) -> Option<Vec<MacroRule>> {
    tree.l_curly_token()?;
    let mut elements = (tree.syntax().children_with_tokens())
        .filter(|e| is_code_kind(e.kind()))
        .skip(1);
    let mut rules = Vec::new();
    loop {
        let matcher = match elements.next()? {
            NodeOrToken::Node(node) => ast::TokenTree::cast(node)?,
            NodeOrToken::Token(token) if token.kind() == SyntaxKind::R_CURLY => return Some(rules),
            NodeOrToken::Token(_) => return None,
        };
        let eq = elements.next()?.into_token()?;
        let gt = elements.next()?.into_token()?;
        if eq.kind() != SyntaxKind::EQ
            || eq.next_token().as_ref() != Some(&gt)
            || gt.kind() != SyntaxKind::R_ANGLE
        {
            return None;
        }
        let transcriber = ast::TokenTree::cast(elements.next()?.into_node()?)?;
        rules.push(MacroRule {
            matcher,
            arrow: (eq, gt),
            transcriber,
        });
        match elements.next()?.kind() {
            SyntaxKind::SEMICOLON => {}
            SyntaxKind::R_CURLY => return Some(rules),
            _ => return None,
        }
    }
}

/// `x` → `expr` for `$x:expr`
fn fragment_specifiers(matcher: &ast::TokenTree) -> FxHashMap<String, String> {
    let tokens: Vec<_> = (matcher.syntax().descendants_with_tokens())
        .filter_map(|e| e.into_token())
        .filter(|t| is_code_kind(t.kind()))
        .collect();
    tokens
        .windows(4)
        .filter(|w| w[0].kind() == SyntaxKind::DOLLAR && w[2].kind() == SyntaxKind::COLON)
        .map(|w| (w[1].text().to_string(), w[3].text().to_string()))
        .collect()
}

/// Both from 1, the column in characters
pub(crate) fn line_column(code: &str, offset: TextSize) -> (usize, usize) {
    LineColumns::new(code).at(offset)
}

/// `line_column` of ascending offsets, each scanning only the text after the previous one
pub(crate) struct LineColumns<'a> {
    code: &'a str,
    scanned: usize,
    line: usize,
    line_start: usize,
}

impl<'a> LineColumns<'a> {
    pub fn new(code: &'a str) -> Self {
        LineColumns {
            code,
            scanned: 0,
            line: 1,
            line_start: 0,
        }
    }

    pub fn at(&mut self, offset: TextSize) -> (usize, usize) {
        let offset = usize::from(offset).min(self.code.len());
        let text = &self.code[self.scanned..offset];
        self.line += text.matches('\n').count();
        if let Some(i) = text.rfind('\n') {
            self.line_start = self.scanned + i + 1;
        }
        self.scanned = offset;
        (self.line, self.code[self.line_start..offset].chars().count() + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_columns_of_ascending_offsets() {
        let code = "aé\n\nbc\nd";
        let mut line_columns = LineColumns::new(code);
        // Repeated, then in the same line, then lines further, the last past the end
        for (offset, expected) in [
            (0, (1, 1)),
            (0, (1, 1)),
            (3, (1, 3)),
            (4, (2, 1)),
            (5, (3, 1)),
            (6, (3, 2)),
            (8, (4, 1)),
            (100, (4, 2)),
        ] {
            let offset = TextSize::new(offset);
            assert_eq!(line_columns.at(offset), expected, "{offset:?}");
            assert_eq!(line_column(code, offset), expected, "{offset:?}");
        }
    }
}
