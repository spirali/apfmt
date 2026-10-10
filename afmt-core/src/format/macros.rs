//! Macro calls

use super::expr::{format_array_expr, format_expr_list, is_simple_expr};
use super::path::format_path;
use crate::comments::{has_comments, prev_code_token};
use crate::formatter::Edge::{BraceClose, BraceOpen, Close, Open, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::layout::list::{Delimiters, TrailingSeparator};
use ra_ap_syntax::ast::{ArgList, ArrayExpr, Expr, HasModuleItem, Item, MacroCall, TokenTree};
use ra_ap_syntax::{AstNode, SourceFile, SyntaxKind, SyntaxNode};

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
    let parsed = match tree.syntax().first_token().unwrap().kind() {
        SyntaxKind::L_PAREN => parse_token_tree::<ArgList>(fmt, tree, "__afmt").map(|args| {
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
        }),
        SyntaxKind::L_BRACK => {
            parse_token_tree::<ArrayExpr>(fmt, tree, "").map(|array| format_array_expr(fmt, array))
        }
        _ => None,
    };
    match parsed {
        Some(frag) => frag.with_cursor(close),
        None if close.kind() == SyntaxKind::R_CURLY => {
            fmt.verbatim(tree.syntax(), BraceOpen, BraceClose)
        }
        None => fmt.verbatim(tree.syntax(), Open, Close),
    }
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

/// The token tree with its delimiters parsed again as the node `N`, after `prefix` (`__afmt` makes
/// `(...)` the arguments of a call); `None` if the text does not parse as exactly that node
fn parse_token_tree<N: AstNode>(fmt: &Formatter, tree: &TokenTree, prefix: &str) -> Option<N> {
    let text = tree.syntax().text().to_string();
    let file = SourceFile::parse(
        &format!("const _: () = {prefix}{text};"),
        fmt.config().edition,
    );
    if !file.errors().is_empty() {
        return None;
    }
    let Some(Item::Const(item)) = file.tree().items().next() else {
        return None;
    };
    let node = item.body()?.syntax().descendants().find_map(N::cast)?;
    (node.syntax().text() == text.as_str()).then_some(node)
}

/// Whether the expression is a macro call whose arguments are laid out as a list (see
/// `format_macro_call`), so it may overflow as the last argument of a call
pub(super) fn is_list_macro(expr: &Expr) -> bool {
    let Expr::MacroExpr(expr) = expr else {
        return false;
    };
    expr.macro_call()
        .and_then(|call| call.token_tree())
        .and_then(|tree| tree.syntax().first_token())
        .is_some_and(|t| matches!(t.kind(), SyntaxKind::L_PAREN | SyntaxKind::L_BRACK))
}
