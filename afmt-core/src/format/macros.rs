//! Macro calls and macro definitions (`macro_rules!`)

use super::common::format_name;
use super::expr::{format_expr, plain_stmt_list, single_line_block_expr};
use super::expr_list::{format_array_expr, format_expr_list, is_simple_expr};
use super::format_reindented;
use super::item::format_item_list;
use super::path::format_path;
use super::stmt::format_transcriber_block;
use crate::comments::{has_comments, is_code_kind, prev_code_token};
use crate::formatter::Edge::{BraceClose, BraceOpen, Close, Open, Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::{Delimiters, TrailingSeparator};
use crate::syntax_utils::{
    MacroArgs, MacroRule, macro_rules_rules, macro_token_tree, parse_macro_args, parse_transcriber,
};
use ra_ap_syntax::ast::{
    ArgList, ArrayExpr, BlockExpr, Expr, HasName, ItemList, MacroCall, MacroRules, StmtList,
};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode};

/// `name!(args)`, `name![args]` or `name! { tokens }`: arguments that parse as expressions are laid
/// out as call arguments or an array; others are kept as in the source
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

fn format_token_tree(fmt: &Formatter, call: &MacroCall) -> Frag {
    let tree = &call.token_tree().unwrap();
    let close = tree.syntax().last_token().unwrap();
    let edition = fmt.config().edition;
    let parsed = fmt.format_reparsed(
        |placeholders| parse_macro_args(tree, edition, placeholders),
        |args| match args {
            MacroArgs::List(args) => {
                let trailing = source_trailing_comma(args.syntax());
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
            }
            MacroArgs::Array(array) => format_bracket_args(fmt, call, array),
        },
    );
    match parsed {
        Some(frag) => frag.with_cursor(close),
        None => Delimiters::of_token_tree(tree).format_reindented(fmt),
    }
}

/// `macro_rules! name { rules }`: each rule on its own line with `;` (as rustfmt), the matchers
/// copied, the transcribers formatted. Otherwise copied as a whole.
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

fn format_macro_rule(fmt: &Formatter, rule: &MacroRule) -> Frag {
    let (eq, gt) = &rule.arrow;
    let mut builder = Builder::new(fmt);
    builder.push(Delimiters::of_token_tree(&rule.matcher).format_reindented(fmt));
    builder.push(fmt.verbatim_range(eq, gt, Operator, Operator));
    builder.push(format_transcriber(fmt, rule));
    builder.push_separator(SyntaxKind::SEMICOLON);
    builder.into_frag()
}

/// Broken items, or statements, or a single hugged block; formatted when it parses with
/// placeholders and `format_macro_bodies` is on, otherwise copied
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

fn format_transcriber_body(fmt: &Formatter, node: SyntaxNode) -> Frag {
    if let Some(list) = ItemList::cast(node.clone()) {
        return format_item_list(fmt, list);
    }
    let body = BlockExpr::cast(node).unwrap();
    match hugged_block(&body) {
        Some((list, block)) => format_hugged_block(fmt, list, block),
        None => format_transcriber_block(fmt, body),
    }
}

/// The block that is all of `body`, `{{ ... }}`, with the statement list of `body`
fn hugged_block(body: &BlockExpr) -> Option<(StmtList, BlockExpr)> {
    let Expr::BlockExpr(block) = single_line_block_expr(body)? else {
        return None;
    };
    plain_stmt_list(&block)?;
    Some((body.stmt_list()?, block))
}

/// `{{ ... }}`: the braces tight around `block` (as rustfmt does in `macro_rules!`)
fn format_hugged_block(fmt: &Formatter, list: StmtList, block: BlockExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(fmt.token_with_edges(list.l_curly_token().unwrap(), BraceOpen, Open));
    builder.push(format_expr(fmt, Expr::BlockExpr(block)));
    builder.push(fmt.token_with_edges(list.r_curly_token().unwrap(), Close, BraceClose));
    builder.into_frag()
}

/// Those of the standard library and `log` (as in rustfmt), with the number of arguments in front
/// of the format string
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

/// The numbers of arguments on the lines of a broken format-string macro call (Style Guide): before
/// the format string, the string, and after it; `None` if not applicable or with comments
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

fn has_trailing_comma(node: &SyntaxNode) -> bool {
    prev_code_token(&node.last_token().unwrap()).is_some_and(|t| t.kind() == SyntaxKind::COMMA)
}

/// Kept as in the source, on a single line too: the macro may reject or require it (as rustfmt)
fn source_trailing_comma(node: &SyntaxNode) -> TrailingSeparator {
    if has_trailing_comma(node) {
        TrailingSeparator::Required
    } else {
        TrailingSeparator::Omitted
    }
}

/// Laid out as an array; `vec!` gets a trailing comma when broken, another macro keeps the source
/// one
fn format_bracket_args(fmt: &Formatter, call: &MacroCall, array: ArrayExpr) -> Frag {
    let is_vec =
        (call.path().and_then(|p| p.segment())).is_some_and(|s| s.syntax().text() == "vec");
    if is_vec || array.semicolon_token().is_some() {
        return format_array_expr(fmt, array);
    }
    let trailing = source_trailing_comma(array.syntax());
    format_expr_list(
        fmt,
        array.syntax(),
        &Delimiters::brackets_of(array.syntax()),
        fmt.config().array_width,
        |layout| layout.trailing_separator(trailing),
    )
}

/// From the commas outside nested delimiters, so it may be more (closure parameters, generics);
/// `None` for another expression
pub(super) fn macro_argument_count(expr: &Expr) -> Option<usize> {
    if !is_list_macro(expr) {
        return None;
    }
    let tree = macro_token_tree(expr)?;
    let mut count = 0;
    let mut in_argument = false;
    // Nested delimiters are child token trees
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

fn macro_delimiter(expr: &Expr) -> Option<SyntaxKind> {
    Some(macro_token_tree(expr)?.syntax().first_token()?.kind())
}

/// Laid out as a list, so it may overflow as the last argument of a call
pub(super) fn is_list_macro(expr: &Expr) -> bool {
    matches!(
        macro_delimiter(expr),
        Some(SyntaxKind::L_PAREN | SyntaxKind::L_BRACK)
    )
}

/// Its token tree is copied (reindented) rather than formatted: braces, or arguments that do not
/// parse
pub(super) fn is_copied_macro(fmt: &Formatter, expr: &Expr) -> bool {
    let Some(tree) = macro_token_tree(expr) else {
        return false;
    };
    let edition = fmt.config().edition;
    !fmt.can_reparse(|p| parse_macro_args(&tree, edition, p))
}

/// `vec![a, b]`
pub(super) fn is_array_macro(expr: &Expr) -> bool {
    macro_delimiter(expr) == Some(SyntaxKind::L_BRACK)
}
