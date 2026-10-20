//! Helpers on the syntax tree of `ra_ap_syntax`

use std::collections::HashMap;

use crate::comments::line_indentation;
use ra_ap_syntax::ast::{self, HasAttrs, HasModuleItem};
use ra_ap_syntax::{
    AstNode, Edition, NodeOrToken, SourceFile, SyntaxKind, SyntaxNode, SyntaxToken, TextSize,
};

/// Whether the node has attributes
pub(crate) fn has_attrs(node: &impl HasAttrs) -> bool {
    node.attrs().next().is_some()
}

/// Whether `node` is a macro call with braces, `m! { ... }`, as an expression: as the only content of a
/// block, it is a statement (it may expand to statements or items, `let a = 1;`), so the block cannot
/// be removed; with `()` or `[]`, it is an expression there as anywhere else
pub(crate) fn is_brace_macro_expr(node: &SyntaxNode) -> bool {
    ast::MacroExpr::cast(node.clone())
        .and_then(|expr| expr.macro_call()?.token_tree()?.l_curly_token())
        .is_some()
}

/// Whether `node` is a `return`, `break` or `continue` expression, which ends its block with `;`
/// (`trailing_semicolon`)
pub(crate) fn is_jump_expr(node: &SyntaxNode) -> bool {
    matches!(
        node.kind(),
        SyntaxKind::RETURN_EXPR | SyntaxKind::BREAK_EXPR | SyntaxKind::CONTINUE_EXPR
    )
}

/// The statement `return;`, `break;` or `continue;` ending a block without a tail expression, whose
/// `;` the formatter adds or removes (`trailing_semicolon`)
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

/// A stray `;` in a block, not part of a statement (`;;`), which the formatter removes
/// (`rewrite/empty_stmts.rs`)
pub(crate) fn is_stray_semicolon(token: &SyntaxToken) -> bool {
    token.kind() == SyntaxKind::SEMICOLON
        && token
            .parent()
            .is_some_and(|p| p.kind() == SyntaxKind::STMT_LIST)
}

/// The tokens of a tree parsed again that are written as other text, with that text: in a
/// transcriber of `macro_rules!`, the placeholders of the metavariables (`_x` for `$x`, see
/// `parse_transcriber`)
pub(crate) type Placeholders = HashMap<SyntaxToken, String>;

/// A token tree parsed again as the node `N` (see `parse_token_tree`, `parse_transcriber`)
pub(crate) struct Reparsed<N> {
    pub node: N,
    /// The tokens of `node` written as other text, see `Placeholders`
    pub placeholders: Vec<(SyntaxToken, String)>,
}

impl<N: AstNode> Reparsed<N> {
    /// The node as a plain syntax node
    pub fn into_syntax(self) -> Reparsed<SyntaxNode> {
        Reparsed {
            node: self.node.syntax().clone(),
            placeholders: self.placeholders,
        }
    }
}

/// The text of a token tree to parse again, with the placeholders in it (see `substituted_text`)
struct SubstitutedText {
    text: String,
    /// The offset and the length of each placeholder in `text`, with the text it stands for
    placeholders: Vec<(usize, usize, String)>,
}

/// The text of `tree`, with its delimiters. With `specifiers` (in a transcriber of `macro_rules!`,
/// the fragment specifiers of its matcher), each metavariable `$name` (and `$crate`) is replaced
/// with one token that may stand where the fragment may, of the same width except for `vis`:
/// `'name` for a `lifetime`, `pub` for a `vis`, otherwise the identifier `_name`; `None` when a `$`
/// is not followed by a name (a repetition `$( ... )`). The tokens of `tree` that are `placeholders`
/// already (`tree` is in a tree parsed again) stay placeholders.
fn substituted_text(
    tree: &ast::TokenTree,
    specifiers: Option<&HashMap<String, String>>,
    placeholders: &Placeholders,
) -> Option<SubstitutedText> {
    let mut text = String::new();
    let mut substituted = Vec::new();
    let mut tokens = (tree.syntax().descendants_with_tokens()).filter_map(|e| e.into_token());
    while let Some(token) = tokens.next() {
        if let Some(specifiers) = specifiers.filter(|_| token.kind() == SyntaxKind::DOLLAR) {
            let name = tokens.next().filter(|n| n.kind().is_any_identifier())?;
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
        if let Some(source) = placeholders.get(&token) {
            substituted.push((text.len(), token.text().len(), source.clone()));
        }
        text.push_str(token.text());
    }
    Some(SubstitutedText {
        text,
        placeholders: substituted,
    })
}

/// `substituted` (the text of `tree`) parsed between `prefix` and `suffix` as the node `pick` finds
/// in the file; `None` if the file has syntax errors or the node is not exactly the text. The parsed
/// text starts with the indentation of the source line of `tree`, so the lines of the tree have the
/// same indentation relative to its first line as in the source (see `Formatter::reindented_range`).
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
    if !file.errors().is_empty() {
        return None;
    }
    let root = file.tree().syntax().clone();
    let node = pick(file.tree())?;
    if node.syntax().text() != text.as_str() {
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

/// The token tree of a macro call, with its delimiters, parsed again as the node `N` (`(...)` as the
/// arguments of a call `__afmt(...)`, `[...]` as an array); `None` if the text does not parse as
/// exactly that node. Its `placeholders` stay placeholders (see `reparse`).
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

/// The transcriber of `rule` parsed again as the items of a module (an `ItemList`), else as the body
/// of a function (a `BlockExpr`: statements and a tail expression), with its metavariables replaced
/// by placeholders (see `substituted_text`); `None` for a transcriber in `()` or `[]`, with a
/// repetition `$( ... )`, or that does not parse. Its `placeholders` stay placeholders.
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

/// A rule of `macro_rules!`, `(matcher) => { transcriber }`; `=>` is two tokens in a token tree
pub(crate) struct MacroRule {
    pub matcher: ast::TokenTree,
    pub arrow: (SyntaxToken, SyntaxToken),
    pub transcriber: ast::TokenTree,
}

/// The rules in the braces of `macro_rules!` (its token tree `tree`), separated by `;` (optional after
/// the last one); `None` when the token tree is not such a list
pub(crate) fn macro_rules_rules(tree: &ast::TokenTree) -> Option<Vec<MacroRule>> {
    tree.l_curly_token()?;
    let mut elements = (tree.syntax().children_with_tokens())
        .filter(|e| !e.kind().is_trivia())
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

/// The fragment specifiers of the metavariables of a matcher, by name: `x` → `expr` for `$x:expr`
fn fragment_specifiers(matcher: &ast::TokenTree) -> HashMap<String, String> {
    let tokens: Vec<_> = (matcher.syntax().descendants_with_tokens())
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
        .collect();
    tokens
        .windows(4)
        .filter(|w| w[0].kind() == SyntaxKind::DOLLAR && w[2].kind() == SyntaxKind::COLON)
        .map(|w| (w[1].text().to_string(), w[3].text().to_string()))
        .collect()
}

/// The line and the column (both from 1, the column in characters) of `offset` in `code`
pub(crate) fn line_column(code: &str, offset: TextSize) -> (usize, usize) {
    let before = &code[..usize::from(offset).min(code.len())];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    (
        before.matches('\n').count() + 1,
        before[line_start..].chars().count() + 1,
    )
}
