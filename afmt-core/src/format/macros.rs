//! Macro calls and macro definitions (`macro_rules!`)

use super::common::format_name;
use super::expr::{format_expr, plain_stmt_list, single_line_block_expr};
use super::expr_list::{format_array_expr, format_expr_list, is_simple_expr};
use super::format_reindented;
use super::item::format_item_list;
use super::path::format_path;
use super::stmt::format_block_expr;
use crate::comments::{has_comments, is_code_kind, prev_code_token};
use crate::formatter::Edge::{BraceClose, BraceOpen, Close, Open, Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::{Delimiters, TrailingSeparator};
use crate::syntax_utils::{MacroRule, macro_rules_rules, parse_token_tree, parse_transcriber};
use ra_ap_syntax::ast::{
    ArgList, ArrayExpr, BlockExpr, Expr, HasName, ItemList, MacroCall, MacroRules, StmtList,
};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode};

/// `name!(args)`, `name![args]` or `name! { tokens }` (Style Guide "Macro uses"): arguments that parse
/// as expressions are laid out as call arguments (`()`) or as an array (`[]`); other arguments and
/// `{}` are kept as they are in the source
pub(super) fn format_macro_call(fmt: &Formatter, call: &MacroCall) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_path(fmt, call.path().unwrap()));
    builder.push(fmt.token_with_edges(call.excl_token().unwrap(), Tight, Word));
    builder.push(format_token_tree(fmt, call));
    if call.semicolon_token().is_some() {
        builder.push_separator(SyntaxKind::SEMICOLON);
    }
    builder.into_frag()
}

/// The arguments of a macro call, see `format_macro_call`
fn format_token_tree(fmt: &Formatter, call: &MacroCall) -> Frag {
    let tree = &call.token_tree().unwrap();
    let close = tree.syntax().last_token().unwrap();
    let edition = fmt.config().edition;
    let parsed = match tree.syntax().first_token().unwrap().kind() {
        SyntaxKind::L_PAREN => fmt.format_reparsed(
            |placeholders| parse_token_tree::<ArgList>(tree, edition, placeholders),
            |args| {
                // The trailing comma is kept as it is: a macro may not accept one, or require it
                let trailing = if has_trailing_comma(args.syntax()) {
                    TrailingSeparator::Required
                } else {
                    TrailingSeparator::Omitted
                };
                format_expr_list(
                    fmt,
                    args.syntax(),
                    &Delimiters::parens_of(args.syntax()),
                    fmt.config().fn_call_width,
                    |layout| {
                        let lines = format_string_lines(call, &args);
                        layout.trailing_separator(trailing).lines_if(lines)
                    },
                )
            },
        ),
        SyntaxKind::L_BRACK => fmt.format_reparsed(
            |placeholders| parse_token_tree::<ArrayExpr>(tree, edition, placeholders),
            |array| format_array_expr(fmt, array),
        ),
        _ => None,
    };
    match parsed {
        Some(frag) => frag.with_cursor(close),
        None => Delimiters::of_token_tree(tree).format_reindented(fmt),
    }
}

/// `macro_rules! name { rules }`: each rule `matcher => transcriber;` on its own line, block indented,
/// with a `;` after the last one too (as rustfmt; the Style Guide has no rules for macro definitions);
/// the matchers are copied from the source, reindented, the transcribers formatted (see
/// `format_transcriber`). A definition in `()` or `[]`, or whose token tree is not a list of rules, is
/// copied as a whole.
pub(super) fn format_macro_rules(fmt: &Formatter, item: &MacroRules) -> Frag {
    let tree = item.token_tree().unwrap();
    let Some(rules) = macro_rules_rules(&tree) else {
        return format_reindented(fmt, item.syntax());
    };
    let mut builder = Builder::new(fmt);
    builder.push_token(item.macro_rules_token().unwrap(), Word);
    builder.push(fmt.token_with_edges(item.excl_token().unwrap(), Tight, Word));
    builder.push(format_name(fmt, item.name().unwrap()));
    let rules = rules.iter().map(|rule| format_macro_rule(fmt, rule));
    builder.push(Delimiters::braces_of(tree.syntax()).format_block(fmt, rules));
    builder.into_frag()
}

/// `matcher => transcriber;`, see `format_macro_rules`
fn format_macro_rule(fmt: &Formatter, rule: &MacroRule) -> Frag {
    let (eq, gt) = &rule.arrow;
    let mut builder = Builder::new(fmt);
    builder.push(Delimiters::of_token_tree(&rule.matcher).format_reindented(fmt));
    builder.push(fmt.verbatim_range(eq, gt, Operator, Operator));
    builder.push(format_transcriber(fmt, rule));
    builder.push_separator(SyntaxKind::SEMICOLON);
    builder.into_frag()
}

/// The transcriber of a rule in braces: its items, or its statements and tail expression, broken
/// (as rustfmt), or a single block hugged (`format_hugged_block`); formatted when it parses as such
/// with the metavariables replaced (see `parse_transcriber`) and `format_macro_bodies` is on,
/// otherwise copied, reindented
fn format_transcriber(fmt: &Formatter, rule: &MacroRule) -> Frag {
    let tree = &rule.transcriber;
    let edition = fmt.config().edition;
    let parsed = (fmt.config().format_macro_bodies)
        .then(|| {
            fmt.format_reparsed(
                |placeholders| parse_transcriber(rule, edition, placeholders),
                |node| format_transcriber_body(fmt, node),
            )
        })
        .flatten();
    match parsed {
        Some(frag) => frag.with_cursor(tree.syntax().last_token().unwrap()),
        None => Delimiters::of_token_tree(tree).format_reindented(fmt),
    }
}

/// A transcriber parsed again (see `parse_transcriber`): an `ItemList` or a `BlockExpr`
fn format_transcriber_body(fmt: &Formatter, node: SyntaxNode) -> Frag {
    if let Some(list) = ItemList::cast(node.clone()) {
        return format_item_list(fmt, list);
    }
    let body = BlockExpr::cast(node).unwrap();
    match hugged_block(&body) {
        Some((list, block)) => format_hugged_block(fmt, list, block),
        None => format_block_expr(fmt, body),
    }
}

/// The block that is all of `body` (see `single_line_block_expr`), `{{ ... }}`, without a label or
/// modifiers (see `plain_stmt_list`), with the statement list of `body`, whose braces hug it
fn hugged_block(body: &BlockExpr) -> Option<(StmtList, BlockExpr)> {
    let Expr::BlockExpr(block) = single_line_block_expr(body)? else {
        return None;
    };
    plain_stmt_list(&block)?;
    Some((body.stmt_list()?, block))
}

/// `{{ ... }}`: the braces of `list` tight around `block`, formatted as a block expression (as
/// rustfmt does with the transcribers of `macro_rules!`)
fn format_hugged_block(fmt: &Formatter, list: StmtList, block: BlockExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(fmt.token_with_edges(list.l_curly_token().unwrap(), BraceOpen, Open));
    builder.push(format_expr(fmt, Expr::BlockExpr(block)));
    builder.push(fmt.token_with_edges(list.r_curly_token().unwrap(), Close, BraceClose));
    builder.into_frag()
}

/// The format-string macros of the standard library (and those of the `log` crate, as in rustfmt)
/// with the number of arguments in front of the format string
const FORMAT_STRING_MACROS: &[(&str, usize)] = &[
    ("print", 0),
    ("println", 0),
    ("eprint", 0),
    ("eprintln", 0),
    ("format", 0),
    ("format_args", 0),
    ("panic", 0),
    ("unreachable", 0),
    ("todo", 0),
    ("unimplemented", 0),
    ("debug", 0),
    ("error", 0),
    ("info", 0),
    ("warn", 0),
    ("trace", 0),
    ("assert", 1),
    ("debug_assert", 1),
    ("write", 1),
    ("writeln", 1),
    ("assert_eq", 2),
    ("assert_ne", 2),
    ("debug_assert_eq", 2),
    ("debug_assert_ne", 2),
];

/// The numbers of arguments on the lines of a broken format-string macro call whose other arguments
/// are simple (Style Guide "Format string macros"; simple as in rustfmt, see `is_simple_expr`): those
/// in front of the format string, the format string, and those after it, each group on one line;
/// `None` for another macro, or with comments among the arguments
fn format_string_lines(call: &MacroCall, args: &ArgList) -> Option<Vec<usize>> {
    let name = call.path()?.as_single_name_ref()?;
    let &(_, before) = FORMAT_STRING_MACROS
        .iter()
        .find(|(macro_name, _)| name.text() == *macro_name)?;
    let exprs = args.args().collect::<Vec<_>>();
    if exprs.len() <= before
        || has_comments(args.syntax())
        || !(exprs.iter().enumerate()).all(|(i, e)| i == before || is_simple_expr(e))
    {
        return None;
    }
    let lines = [before, 1, exprs.len() - before - 1];
    Some(lines.into_iter().filter(|&n| n > 0).collect())
}

/// Whether the list `node` ends with a comma before its closing delimiter: `(a, b,)`
fn has_trailing_comma(node: &SyntaxNode) -> bool {
    prev_code_token(&node.last_token().unwrap()).is_some_and(|t| t.kind() == SyntaxKind::COMMA)
}

/// The number of arguments of a list-like macro call (see `is_list_macro`), from the commas of its
/// token tree outside nested delimiters, without a trailing one; a comma between closure parameters
/// or generic arguments counts too, so it may be more; `None` for another expression
pub(super) fn macro_argument_count(expr: &Expr) -> Option<usize> {
    if !is_list_macro(expr) {
        return None;
    }
    let Expr::MacroExpr(expr) = expr else {
        return None;
    };
    let tree = expr.macro_call()?.token_tree()?;
    let mut count = 0;
    let mut in_argument = false;
    // The nested delimiters and what they contain are child token trees; the tokens of this one are
    // its delimiters, the commas and the tokens of the arguments
    for child in tree.syntax().children_with_tokens() {
        match child.kind() {
            SyntaxKind::COMMA => {
                count += usize::from(in_argument);
                in_argument = false;
            }
            SyntaxKind::L_PAREN
            | SyntaxKind::R_PAREN
            | SyntaxKind::L_BRACK
            | SyntaxKind::R_BRACK => {}
            kind if !is_code_kind(kind) => {}
            _ => in_argument = true,
        }
    }
    Some(count + usize::from(in_argument))
}

/// The opening delimiter of the arguments of a macro call, `None` for another expression
fn macro_delimiter(expr: &Expr) -> Option<SyntaxKind> {
    let Expr::MacroExpr(expr) = expr else {
        return None;
    };
    let tree = expr.macro_call()?.token_tree()?;
    Some(tree.syntax().first_token()?.kind())
}

/// Whether the expression is a macro call whose arguments are laid out as a list (see
/// `format_macro_call`), so it may overflow as the last argument of a call
pub(super) fn is_list_macro(expr: &Expr) -> bool {
    matches!(
        macro_delimiter(expr),
        Some(SyntaxKind::L_PAREN | SyntaxKind::L_BRACK)
    )
}

/// Whether the expression is a macro call with braces, `m! { ... }`, whose body is copied (re-indented,
/// see `format_macro_call`); like a block, it may overflow as the only argument of a call
pub(super) fn is_brace_macro(expr: &Expr) -> bool {
    macro_delimiter(expr) == Some(SyntaxKind::L_CURLY)
}

/// Whether the expression is a macro call whose arguments are laid out as an array, `vec![a, b]`
/// (see `format_macro_call`)
pub(super) fn is_array_macro(expr: &Expr) -> bool {
    macro_delimiter(expr) == Some(SyntaxKind::L_BRACK)
}
