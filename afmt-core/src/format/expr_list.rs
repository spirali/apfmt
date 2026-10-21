//! Lists of expressions: call arguments, tuples and arrays, and how their last item hugs them

use super::chain::{operand_chain, operator_chain_parts};
use super::closure::closure_layouts;
use super::expr::format_expr;
use super::macros::{is_array_macro, is_list_macro, macro_argument_count};
use crate::format::common::format_repeat;
use crate::formatter::{Formatter, Frag};
use crate::layout::chain::ChainLayouts;
use crate::layout::list::TrailingSeparator;
use crate::layout::list::{
    Delimiters, FlatLimit, Hug, ListItem, ListLayout, Overflow, delimited_list, list_items_cast,
};
use crate::syntax_utils::has_attrs;
use crate::utils::with_is_last;
use ra_ap_syntax::ast::{ArgList, ArrayExpr, CallExpr, Expr, HasArgList, TupleExpr};
use ra_ap_syntax::{AstNode, SyntaxNode};

/// Whether a single argument can overflow, i.e. start on the line of `(` and end on the line of `)`:
/// `f(if a {\n ...\n})`. Follows the combinable expressions of the style guide.
/// Also behind operators (`f(!g(\n...\n))`, `f(&[\n...\n])`, `f(g(\n...\n)?)` as rustfmt does, see
/// `without_operators`).
fn is_combinable_expr(expr: &Expr) -> bool {
    match without_operators(expr) {
        Expr::CallExpr(_)
        | Expr::MethodCallExpr(_)
        | Expr::IfExpr(_)
        | Expr::BlockExpr(_)
        | Expr::ArrayExpr(_)
        | Expr::TupleExpr(_)
        | Expr::ClosureExpr(_)
        | Expr::RecordExpr(_)
        | Expr::MatchExpr(_) => true,
        // `g(...).await`: the root breaks, `.await` follows its closing delimiter
        Expr::AwaitExpr(e) => e.expr().is_some_and(|e| is_combinable_expr(&e)),
        expr => is_list_macro(&expr),
    }
}

/// Whether the expression is a delimited list itself: a call, a list-like macro call, a struct
/// literal or an array, also behind operators (`f(&[` / `...` / `])`, `f(g(` / `...` / `)?)`, see
/// `without_operators`); the Style Guide's examples of a combined single argument
fn is_delimited_list_expr(expr: &Expr) -> bool {
    match without_operators(expr) {
        Expr::CallExpr(_) | Expr::ArrayExpr(_) | Expr::RecordExpr(_) => true,
        expr => is_list_macro(&expr),
    }
}

/// Whether the delimited list expression (see `is_delimited_list_expr`) has more than a single simple
/// item (see `is_simple_expr`; a macro call more than a single argument): hugged, it puts more on
/// lines of their own than one short argument (`Err(custom(` / `span,` / `"..."` / `))`, not
/// `Compact(new_units(` / `value,` / `))`)
fn has_list_content(expr: &Expr) -> bool {
    let has_content = |exprs: &[Expr]| match exprs {
        [] => false,
        [single] => !is_simple_expr(single),
        _ => true,
    };
    match without_operators(expr) {
        Expr::CallExpr(call) => call
            .arg_list()
            .is_some_and(|args| has_content(&args.args().collect::<Vec<_>>())),
        Expr::ArrayExpr(array) => has_content(&array.exprs().collect::<Vec<_>>()),
        Expr::RecordExpr(record) => record.record_expr_field_list().is_some_and(|list| {
            let values = list.fields().map(|f| f.expr()).collect::<Option<Vec<_>>>();
            // A shorthand field (`a`) is simple
            list.fields().count() > 1 || values.is_some_and(|values| has_content(&values))
        }),
        expr => macro_argument_count(&expr).is_some_and(|n| n > 1),
    }
}

/// Whether the call is a constructor: its callee is a path whose last segment starts with an
/// uppercase letter, a tuple struct or an enum variant by Rust's naming conventions (`Ok`, `Some`,
/// `Error::Io`)
fn is_constructor_call(call: &CallExpr) -> bool {
    let Some(Expr::PathExpr(callee)) = call.expr() else {
        return false;
    };
    callee
        .path()
        .and_then(|path| path.segment())
        .and_then(|segment| segment.name_ref())
        .is_some_and(|name| name.text().starts_with(char::is_uppercase))
}

/// Whether the expression is an array or an array-like macro call (`vec![a, b]`), also behind
/// operators (see `without_operators`); not a repeat (`[a; n]`)
fn is_array_like_expr(expr: &Expr) -> bool {
    match without_operators(expr) {
        Expr::ArrayExpr(array) => array.semicolon_token().is_none(),
        expr => is_array_macro(&expr),
    }
}

/// Whether the single argument `expr` of the argument list `node` hugs it whenever it fits, also in
/// more lines than the broken call (see `Overflow::Preferred`): a delimited list with more than a
/// single short item (see `has_list_content`; not `X(Y(` / `value,` / `))` only to put it on its own
/// line), after a constructor, which adds no information of its own (`Ok(X::new(` / `a,` / `b,` /
/// `))`, not `Ok(` / `X::new(a, b),` / `)`), or an array, whose elements are best one per line
/// (`f(&[` / `a,` / `b,` / `])`, the Style Guide's `combinable([`)
fn is_preferred_hug(node: &SyntaxNode, expr: &Expr) -> bool {
    let after_constructor =
        || (node.parent().and_then(CallExpr::cast)).is_some_and(|call| is_constructor_call(&call));
    ArgList::can_cast(node.kind())
        && is_delimited_list_expr(expr)
        && has_list_content(expr)
        && (is_array_like_expr(expr) || after_constructor())
}

/// The expression without the operators applied to it: prefix operators, `&` / `&mut`, casts and
/// `?` (`a.b()` of `&*a.b()? as T`)
fn without_operators(expr: &Expr) -> Expr {
    let operand = match expr {
        Expr::PrefixExpr(e) => e.expr(),
        Expr::RefExpr(e) => e.expr(),
        Expr::CastExpr(e) => e.expr(),
        Expr::TryExpr(e) => e.expr(),
        _ => None,
    };
    operand.map_or_else(|| expr.clone(), |e| without_operators(&e))
}

/// Simple expressions in the sense of rustfmt's `is_simple_expr`: literals, plain names, and simple
/// expressions after a unary operator, `&` or `&mut`, followed by a field, `?` or `as Type`, indexed by
/// a simple expression (`a[i]`) or repeated (`[a; n]`)
pub(super) fn is_simple_expr(expr: &Expr) -> bool {
    let is_simple = |expr: Option<Expr>| expr.is_some_and(|e| is_simple_expr(&e));
    match expr {
        Expr::Literal(_) | Expr::PathExpr(_) => is_literal_or_name(expr),
        Expr::PrefixExpr(expr) => is_simple(expr.expr()),
        Expr::RefExpr(expr) => is_simple(expr.expr()),
        Expr::CastExpr(expr) => is_simple(expr.expr()),
        Expr::FieldExpr(expr) => is_simple(expr.expr()),
        Expr::TryExpr(expr) => is_simple(expr.expr()),
        Expr::IndexExpr(expr) => is_simple(expr.base()) && is_simple(expr.index()),
        Expr::ArrayExpr(expr) if expr.semicolon_token().is_some() => {
            expr.exprs().all(|e| is_simple_expr(&e))
        }
        _ => false,
    }
}

/// A literal or a plain name, optionally after unary operators: `x`, `!x`, `-1`
pub(super) fn is_literal_or_name(expr: &Expr) -> bool {
    match expr {
        Expr::Literal(_) => true,
        Expr::PathExpr(expr) => expr.path().is_some_and(|path| {
            path.qualifier().is_none()
                && path
                    .segment()
                    .is_some_and(|s| s.type_anchor().is_none() && s.coloncolon_token().is_none())
        }),
        Expr::PrefixExpr(expr) => expr.expr().is_some_and(|e| is_literal_or_name(&e)),
        _ => false,
    }
}

/// How the last of the expressions `exprs` of the list `node` may hug it: a single combinable
/// expression (see `is_combinable_expr`), rather than break the list in as many lines if it is a
/// delimited list (see `is_delimited_list_expr`), whenever it fits after a constructor or as an
/// array (see `is_preferred_hug`); a closure or a block after other expressions (`f(a, |x| {` /
/// `...` / `})`, see `is_overflowing_last`)
fn last_item_hug(node: &SyntaxNode, exprs: &[Expr]) -> Hug {
    match exprs {
        [single] if is_preferred_hug(node, single) => Hug::Preferred,
        [single] if is_delimited_list_expr(single) => Hug::WeakPreference,
        [single] if is_combinable_expr(single) => Hug::Allowed,
        _ if is_overflowing_last(exprs) => Hug::Allowed,
        _ => Hug::Never,
    }
}

/// Whether a broken list of the expressions `exprs` (formatted as `items`) is filled on lines instead
/// of one item per line: all items are simple and at most `short_array_element_width_threshold` wide
/// on a single line, as in rustfmt. This is an explicit exception to the Style Guide.
fn is_fillable_list(fmt: &Formatter, exprs: &[Expr], items: &[ListItem]) -> bool {
    let threshold = fmt.config().short_array_element_width_threshold;
    threshold > 0
        && exprs.iter().all(is_simple_expr)
        && items
            .iter()
            .all(|i| fmt.min_flat_width(&i.frag).is_some_and(|w| w <= threshold))
}

/// Whether the last of more expressions may overflow: a closure if no other one is (Style Guide), or
/// a block, also `unsafe`, `async` or labelled (`f(a, unsafe {` / `...` / `})`, as rustfmt does)
fn is_overflowing_last(exprs: &[Expr]) -> bool {
    let is_closure = |e: &Expr| matches!(e, Expr::ClosureExpr(_));
    match exprs {
        [rest @ .., last] if !rest.is_empty() => {
            matches!(last, Expr::BlockExpr(_)) || is_closure(last) && !rest.iter().any(is_closure)
        }
        _ => false,
    }
}

/// An expression formatted as a list item, see `format_expr_item`
pub(super) struct ExprItem {
    pub frag: Frag,
    /// How it may overflow as the last item
    pub overflow: Overflow,
    /// Whether its single line is limited by its own width rules rather than by the width of the
    /// list (`fn_call_width`) when it is the only item: a chain with elements (`chain_width`) or a
    /// closure, as in rustfmt
    pub own_width: bool,
}

/// An expression as a list item, and how it may overflow as the last item (see `Overflow`): a
/// combinable expression with all its layouts (see `is_combinable_expr`), a method call chain and
/// a closure with some of them (see `postfix_chain_layouts` and `ClosureLayouts`)
pub(super) fn format_expr_item(fmt: &Formatter, expr: Expr) -> ExprItem {
    match expr {
        // A chain, also behind operators: on the line of `(` only with its elements on one line, the
        // arguments of the last one may break: `f(a.b(|x| {` / `...` / `}))`, `f(&a.b(` / `...` /
        // `)?)`, not the chain broken before every `.` (its lines would start at the indentation of
        // the statement, an operator seemingly applying to the root only: `f(*rt` / `.worker(w)` /
        // `.unwrap())`); `f(g(` / `...` / `)?)` is the root of a chain without elements, and a single
        // element may follow a broken root on its last line (`f(g(` / `...` / `).into())`, see
        // `ChainLayouts::joined`)
        expr if !has_attrs(&expr)
            && let Some(chain) = operand_chain(&expr).filter(|chain| {
                matches!(
                    chain,
                    Expr::MethodCallExpr(_) | Expr::TryExpr(_) | Expr::AwaitExpr(_)
                ) && is_combinable_expr(chain)
            }) =>
        {
            // A chain without elements, `g(...)?`, is limited as the call is
            let own_width = matches!(
                without_operators(&chain),
                Expr::MethodCallExpr(_) | Expr::AwaitExpr(_)
            );
            let ChainLayouts { layouts, joined } = operator_chain_parts(fmt, &expr, chain);
            let overflow = fmt
                .alt_some(layouts.other.clone(), joined)
                .map_or(Overflow::Never, Overflow::With);
            ExprItem {
                frag: layouts.all(fmt),
                overflow,
                own_width,
            }
        }
        Expr::ClosureExpr(closure) if !has_attrs(&closure) => {
            let layouts = closure_layouts(fmt, closure);
            let overflow = layouts
                .overflow
                .map_or(Overflow::Never, Overflow::Preferred);
            ExprItem {
                frag: layouts.all,
                overflow,
                own_width: true,
            }
        }
        expr => {
            let combinable = is_combinable_expr(&expr);
            // A closure behind operators (`&|x| ...`) or with attributes
            let own_width = matches!(without_operators(&expr), Expr::ClosureExpr(_));
            let frag = format_expr(fmt, expr);
            let overflow = if combinable {
                Overflow::With(frag.clone())
            } else {
                Overflow::Never
            };
            ExprItem {
                frag,
                overflow,
                own_width,
            }
        }
    }
}

/// The expressions of the comma-separated list `node` (call arguments, tuple fields, array elements)
/// on a single line if they fit within `width`, otherwise one per line, or filled on lines if they
/// are short (see `is_fillable_list`). The last expression may hug the list (see `last_item_hug`); a
/// single other expression is limited only by `max_width`. A list with an item spanning more lines
/// on a single line (a string literal, see `Formatter::has_multi_line_text`) is always broken.
/// `refine` adjusts the layouts, e.g. the trailing separator.
pub(super) fn format_expr_list(
    fmt: &Formatter,
    node: &SyntaxNode,
    delims: &Delimiters,
    width: u32,
    refine: impl FnOnce(ListLayout) -> ListLayout,
) -> Frag {
    let exprs = node.children().filter_map(Expr::cast).collect::<Vec<_>>();
    let hug = last_item_hug(node, &exprs);
    let items = list_items_cast(node, |e| format_expr_item(fmt, e));
    let own_width = matches!(items.as_slice(), [item] if item.frag.own_width);
    let items = with_is_last(&items)
        .map(|(item, is_last)| {
            let ExprItem { frag, overflow, .. } = &item.frag;
            // "Where a function call has a single argument, and that argument is formatted across
            // multiple lines, format the outer call as if it were a single-line call" (Style Guide):
            // `Err(anyhow!(` / `...` / `))` rather than `Err(` / `anyhow!(...),` / `)` in as many
            // lines; the broken call is kept when it takes fewer lines, except after a constructor
            // or around an array (see `is_preferred_hug`)
            let overflow = if is_last {
                hug.overflow(overflow)
            } else {
                Overflow::Never
            };
            item.map(|_| frag.clone()).with_overflow(overflow)
        })
        .collect::<Vec<ListItem>>();
    // A single argument that cannot overflow is not limited by `fn_call_width`, nor is one with width
    // rules of its own, a chain with elements or a closure (`ExprItem::own_width`). Another one that
    // can (e.g. a call) is: when it does not fit, it is broken instead. Nor is one with no layout
    // spanning more lines, which has nothing to overflow with (`f(a::B::new())`, a macro call copied
    // unchanged). All as rustfmt does; the Style Guide leaves what a small call is to the tool.
    let unlimited = match items.as_slice() {
        [item] if exprs.len() == 1 => {
            hug == Hug::Never || own_width || !fmt.has_line_break(&item.frag)
        }
        _ => false,
    };
    let flat = if unlimited {
        FlatLimit::MaxWidth
    } else {
        FlatLimit::Within(width)
    };
    // A string spanning lines, also under an operator (`&"a⏎b"`), is not hugged: "If ... any
    // argument ... is multi-line, then format the call across multiple lines" (Style Guide; a
    // literal is not combinable), as in rustfmt
    let multi_line = items.iter().any(|item| fmt.has_multi_line_text(&item.frag));
    let layout = ListLayout::tall(flat)
        .fill_if(is_fillable_list(fmt, &exprs, &items))
        .broken_if(multi_line);
    delimited_list(fmt, delims, &items, refine(layout))
}

pub(super) fn format_arg_list(fmt: &Formatter, arg_list: ArgList) -> Frag {
    format_expr_list(
        fmt,
        arg_list.syntax(),
        &Delimiters::parens_of(arg_list.syntax()),
        fmt.config().fn_call_width,
        |layout| layout,
    )
}

/// `(a, b)`, or `(a,)` with a single field; laid out as call arguments
pub(super) fn format_tuple_expr(fmt: &Formatter, expr: TupleExpr) -> Frag {
    format_expr_list(
        fmt,
        expr.syntax(),
        &Delimiters::parens_of(expr.syntax()),
        fmt.config().fn_call_width,
        |layout| layout.trailing_separator(single_item_separator(expr.fields().count())),
    )
}

/// The trailing separator of a tuple: `(a,)` is a tuple, `(a)` just a parenthesized expression
pub(super) fn single_item_separator(items: usize) -> TrailingSeparator {
    if items == 1 {
        TrailingSeparator::Required
    } else {
        TrailingSeparator::IfVertical
    }
}

/// `[a, b, c]` laid out as call arguments within `array_width`, or `[value; len]`
pub(super) fn format_array_expr(fmt: &Formatter, expr: ArrayExpr) -> Frag {
    let Some(semicolon) = expr.semicolon_token() else {
        return format_expr_list(
            fmt,
            expr.syntax(),
            &Delimiters::brackets_of(expr.syntax()),
            fmt.config().array_width,
            |layout| layout,
        );
    };
    let mut exprs = expr.exprs();
    let value = format_expr(fmt, exprs.next().unwrap());
    let len = format_expr(fmt, exprs.next().unwrap());
    format_repeat(fmt, expr.syntax(), value, semicolon, len)
}

#[cfg(test)]
mod tests {
    use super::{Hug, last_item_hug};
    use ra_ap_syntax::ast::Expr;
    use ra_ap_syntax::{AstNode, Edition, SourceFile, SyntaxKind};

    /// The hug of the last item of the first list of expressions (arguments, tuple fields, array
    /// elements) in `code`
    fn hug(code: &str) -> Hug {
        let source = format!("fn f() {{ {code}; }}");
        let parse = SourceFile::parse(&source, Edition::Edition2024);
        let list = (parse.syntax_node().descendants())
            .find(|n| {
                matches!(
                    n.kind(),
                    SyntaxKind::ARG_LIST | SyntaxKind::TUPLE_EXPR | SyntaxKind::ARRAY_EXPR
                )
            })
            .unwrap();
        let exprs = list.children().filter_map(Expr::cast).collect::<Vec<_>>();
        last_item_hug(&list, &exprs)
    }

    #[test]
    fn last_item_hugs() {
        use Hug::{Allowed, Never, Preferred, WeakPreference};
        for (code, expected) in [
            ("f(a)", Never),
            ("f(a.b)", Never),
            ("f(a + b)", Never),
            ("f(a, b)", Never),
            ("f(if a { b } else { c })", Allowed),
            ("f(match a {})", Allowed),
            ("f(|x| x)", Allowed),
            ("f(a.b())", Allowed),
            ("f(g(a).await)", Allowed),
            ("f(a.await)", Never),
            ("f(a, |x| x)", Allowed),
            ("f(a, &|x| x)", Never),
            ("f(|x| x, |y| y)", Never),
            ("f(a, unsafe { b })", Allowed),
            ("f(g(a))", WeakPreference),
            ("f(g(a, b))", WeakPreference),
            ("f(&g(a, b)?)", WeakPreference),
            ("Ok(g(a))", WeakPreference),
            ("Ok(g(a, b))", Preferred),
            ("Ok(g(h(a)))", Preferred),
            ("Ok(S { a, b })", Preferred),
            ("Ok(S { a })", WeakPreference),
            ("Ok(format!(\"{}\", a))", Preferred),
            ("f(&[a, b])", Preferred),
            ("f(vec![a, b])", Preferred),
            ("f([a; n])", WeakPreference),
            ("x.f(&[a, b])", Preferred),
            ("(g(a, b),)", WeakPreference),
            ("[g(a, b)]", WeakPreference),
        ] {
            assert_eq!(hug(code), expected, "{code}");
        }
    }
}
