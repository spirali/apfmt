//! Lists of expressions: call arguments, tuples and arrays, and how their last item hugs them

use super::chain::{is_chain_call, operand_chain, operator_chain_parts};
use super::closure::closure_layouts;
use super::expr::{format_expr, unary_operand};
use super::macros::{
    is_array_macro, is_copied_macro, is_list_macro, macro_argument_count,
};
use crate::comments::{prev_code_token, trivia_from};
use crate::doc::WeakPreference;
use crate::format::common::format_repeat;
use crate::formatter::{Formatter, Frag};
use crate::layout::chain::ChainLayouts;
use crate::layout::list::TrailingSeparator;
use crate::layout::list::{
    Delimiters, FlatLimit, Hug, ListItem, ListLayout, Overflow, delimited_list, list_items_cast,
};
use crate::syntax_utils::{has_attrs, is_brace_macro};
use crate::utils::with_is_last;
use ra_ap_syntax::ast::{ArgList, ArrayExpr, CallExpr, Expr, HasArgList, LiteralKind, TupleExpr};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode};

/// A single argument that can start on the line of `(` and end on the line of `)`: the Style
/// Guide's combinable expressions, also behind operators as rustfmt does (`f(&[` / `...` / `])`)
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
        | Expr::MatchExpr(_)
        | Expr::LoopExpr(_)
        | Expr::WhileExpr(_)
        | Expr::ForExpr(_) => true,
        // `g(...).await`: the root breaks, `.await` follows its closing delimiter
        Expr::AwaitExpr(e) => e.expr().is_some_and(|e| is_combinable_expr(&e)),
        // `m! { ... }` like a block: `Ok(quote! {` / `...` / `})`
        expr => is_list_macro(&expr) || is_brace_macro(&expr),
    }
}

/// A call, a list-like macro call, a struct literal or an array, also behind operators; not a call
/// of a chain's result (`a.b()(x)`)
fn is_delimited_list_expr(expr: &Expr) -> bool {
    match without_operators(expr) {
        expr @ Expr::CallExpr(_) => !is_chain_call(&expr),
        Expr::ArrayExpr(_) | Expr::RecordExpr(_) => true,
        expr => is_list_macro(&expr),
    }
}

/// Whether nested hugs end in a closure, a chain or control flow (`Ok(X(Box::new(move |x| {`): its
/// first line has width rules of its own (see `ExprItem::own_width`)
fn ends_in_own_width_hug(expr: &Expr) -> bool {
    match without_operators(expr) {
        Expr::ClosureExpr(_)
        | Expr::MethodCallExpr(_)
        | Expr::AwaitExpr(_)
        | Expr::IfExpr(_)
        | Expr::MatchExpr(_)
        | Expr::BlockExpr(_)
        | Expr::LoopExpr(_)
        | Expr::WhileExpr(_)
        | Expr::ForExpr(_) => true,
        expr if is_chain_call(&expr) => true,
        Expr::CallExpr(call) => (call.arg_list().and_then(|args| args.args().last()))
            .is_some_and(|last| ends_in_own_width_hug(&last)),
        expr => is_brace_macro(&expr),
    }
}

/// How many calls are nested as the only argument of each other: 2 for `X(Y(z(a, b)))`, 0 for `x(a,
/// b)`
fn nested_hug_depth(expr: &Expr) -> u32 {
    let Expr::CallExpr(call) = without_operators(expr) else {
        return 0;
    };
    let mut args = call.arg_list().into_iter().flat_map(|args| args.args());
    match (args.next(), args.next()) {
        (Some(arg), None) if is_delimited_list_expr(&arg) => 1 + nested_hug_depth(&arg),
        _ => 0,
    }
}

/// More than a single simple item: hugged, it puts more on lines of its own than one short argument
/// (`Err(custom(` / `span,` / `"..."` / `))`, not `Compact(new_units(` / `value,` / `))`)
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
            list.fields().count() > 1 || values.is_some_and(|values| has_content(&values))
        }),
        expr => macro_argument_count(&expr).is_some_and(|n| n > 1),
    }
}

/// A block, a `loop`, a brace macro call, or a `match`/`if`/`while`/`for` with a simple head. As
/// the only argument it hugs whenever it fits, as a closure does, as rustfmt does.
fn is_block_with_simple_head(expr: &Expr) -> bool {
    let is_simple = |e: Option<Expr>| e.is_some_and(|e| is_simple_expr(&e));
    match without_operators(expr) {
        Expr::BlockExpr(_) | Expr::LoopExpr(_) => true,
        Expr::MatchExpr(e) => is_simple(e.expr()),
        Expr::IfExpr(e) => is_simple(e.condition()),
        Expr::WhileExpr(e) => is_simple(e.condition()),
        Expr::ForExpr(e) => is_simple(e.iterable()),
        expr => is_brace_macro(&expr),
    }
}

/// A path callee whose last segment starts with an uppercase letter (`Ok`, `Some`, `Error::Io`)
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

/// Also behind operators; not a repeat (`[a; n]`)
fn is_array_like_expr(expr: &Expr) -> bool {
    match without_operators(expr) {
        Expr::ArrayExpr(array) => array.semicolon_token().is_none(),
        expr => is_array_macro(&expr),
    }
}

/// A single argument that hugs whenever it fits: a delimited list with content (`has_list_content`)
/// after a constructor (`Ok(X::new(` / `a,` / `))`), or an array (`f(&[` / `a,` / `])`)
fn is_preferred_hug(node: &SyntaxNode, expr: &Expr) -> bool {
    let after_constructor =
        || (node.parent().and_then(CallExpr::cast)).is_some_and(|call| is_constructor_call(&call));
    ArgList::can_cast(node.kind())
        && is_delimited_list_expr(expr)
        && has_list_content(expr)
        && (is_array_like_expr(expr) || after_constructor())
}

/// E.g. `a.b()` of `&*a.b()? as T`
fn without_operators(expr: &Expr) -> Expr {
    let operand = match expr {
        Expr::TryExpr(e) => e.expr(),
        expr => unary_operand(expr),
    };
    operand.map_or_else(|| expr.clone(), |e| without_operators(&e))
}

/// A multi-line string literal starting on the line of the token before it (`f(r#"` / `...` /
/// `"#)`): as a single item, it hugs the list as in the source (as Prettier and Ruff)
fn is_hugging_string(expr: &Expr) -> bool {
    let Expr::Literal(literal) = without_refs(expr) else {
        return false;
    };
    let is_string = matches!(
        literal.kind(),
        LiteralKind::String(_) | LiteralKind::ByteString(_) | LiteralKind::CString(_)
    );
    is_string && literal.token().text().contains('\n') && starts_on_line_of_previous_token(expr)
}

/// Only spaces between it and the code token before it
fn starts_on_line_of_previous_token(expr: &Expr) -> bool {
    let previous = (expr.syntax().first_token()).and_then(|t| prev_code_token(&t));
    trivia_from(previous.and_then(|t| t.next_token()))
        .all(|t| t.kind() == SyntaxKind::WHITESPACE && !t.text().contains('\n'))
}

/// A multi-line copied macro call as the last of more items, hugging the list in the source
/// (`f(a, json!({` / `...` / `}))`): its lines cannot be broken, so it hugs if breaking overflows
fn is_hugging_copied_macro(fmt: &Formatter, exprs: &[Expr]) -> bool {
    let [rest @ .., last] = exprs else {
        return false;
    };
    !rest.is_empty()
        && last.syntax().text().contains_char('\n')
        && starts_on_line_of_previous_token(last)
        && is_copied_macro(fmt, &without_refs(last))
}

/// E.g. `|x| x` of `&&mut |x| x`
fn without_refs(expr: &Expr) -> Expr {
    match expr {
        Expr::RefExpr(e) => e.expr().map_or_else(|| expr.clone(), |e| without_refs(&e)),
        _ => expr.clone(),
    }
}

/// rustfmt's `is_simple_expr`: literals and names, possibly after a unary operator, `&` or `&mut`,
/// followed by a field, `?` or `as Type`, indexed by a simple expression, or repeated
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

/// E.g. `x`, `!x`, `-1`
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

/// How the last of `exprs` may hug the list `node`: a single combinable expression, a closure or a
/// block after other expressions (`f(a, |x| {` / `...` / `})`)
fn last_item_hug(node: &SyntaxNode, exprs: &[Expr]) -> Hug {
    match exprs {
        [single] if is_preferred_hug(node, single) || is_block_with_simple_head(single) => {
            Hug::Preferred
        }
        [single] if is_delimited_list_expr(single) => Hug::WeakPreference,
        // Not a tuple: its comma would follow the string (`(r#"` / `...` / `"#,)`)
        [single] if is_hugging_string(single) && !TupleExpr::can_cast(node.kind()) => {
            Hug::Preferred
        }
        [single] if is_combinable_expr(single) => Hug::Allowed,
        _ if is_overflowing_last(exprs) => Hug::Allowed,
        _ => Hug::Never,
    }
}

/// `f(g(a, b))`: a call with a single argument that may hug its list; not one with attributes,
/// which are copied with it
pub(super) fn has_single_hugging_argument(call: &CallExpr) -> bool {
    let Some(args) = call.arg_list() else {
        return false;
    };
    let exprs = args.args().collect::<Vec<_>>();
    matches!(exprs.as_slice(), [arg] if !has_attrs(arg))
        && last_item_hug(args.syntax(), &exprs) != Hug::Never
}

/// All items simple and at most `short_array_element_width_threshold` wide: filled as in rustfmt,
/// an exception to the Style Guide
fn is_fillable_list(fmt: &Formatter, exprs: &[Expr], items: &[ListItem]) -> bool {
    let threshold = fmt.config().short_array_element_width_threshold;
    threshold > 0
        && exprs.iter().all(is_simple_expr)
        && items
            .iter()
            .all(|i| fmt.min_flat_width(&i.frag).is_some_and(|w| w <= threshold))
}

/// A closure if no other one is (Style Guide), or a block (as rustfmt), also after `&` or `&mut`
fn is_overflowing_last(exprs: &[Expr]) -> bool {
    let is_closure = |e: &Expr| matches!(without_refs(e), Expr::ClosureExpr(_));
    match exprs {
        [rest @ .., last] if !rest.is_empty() => {
            matches!(without_refs(last), Expr::BlockExpr(_))
                || is_closure(last) && !rest.iter().any(is_closure)
        }
        _ => false,
    }
}

pub(super) struct ExprItem {
    pub frag: Frag,
    pub overflow: Overflow,
    /// As the only item, its single line is limited by its own width rules, not `fn_call_width`: a
    /// chain (`chain_width`), a closure or a block with a simple head, as in rustfmt
    pub own_width: bool,
}

/// An expression as a list item, and how it may overflow as the last item
pub(super) fn format_expr_item(fmt: &Formatter, expr: Expr) -> ExprItem {
    match expr {
        // A chain hugs only with its elements on one line and the last arguments broken (`f(a.b(|x|
        // {`); a single element may follow a broken root (`f(g(` / `...` / `).into())`)
        expr if !has_attrs(&expr)
            && let Some(chain) = operand_chain(&expr).filter(|chain| {
                (matches!(
                    chain,
                    Expr::MethodCallExpr(_) | Expr::TryExpr(_) | Expr::AwaitExpr(_)
                ) || is_chain_call(chain))
                    && is_combinable_expr(chain)
            }) =>
        {
            let own_width = matches!(
                without_operators(&chain),
                Expr::MethodCallExpr(_) | Expr::AwaitExpr(_)
            ) || is_chain_call(&without_operators(&chain));
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
            let combinable = is_combinable_expr(&expr) || is_hugging_string(&expr);
            // A closure behind operators or with attributes, a block with a simple head, or an
            // array, tuple or struct literal: its single line has width rules of its own, as in
            // rustfmt
            let own_width = matches!(
                without_operators(&expr),
                Expr::ClosureExpr(_)
                    | Expr::ArrayExpr(_)
                    | Expr::TupleExpr(_)
                    | Expr::RecordExpr(_)
            ) || is_block_with_simple_head(&expr);
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

/// Call arguments, tuple fields or array elements: flat within `width`, otherwise one per line or
/// filled
pub(super) fn format_expr_list(
    fmt: &Formatter,
    node: &SyntaxNode,
    delims: &Delimiters,
    width: u32,
    refine: impl FnOnce(ListLayout) -> ListLayout,
) -> Frag {
    let exprs = node.children().filter_map(Expr::cast).collect::<Vec<_>>();
    let hug = last_item_hug(node, &exprs);
    let hugging_copied_macro = hug == Hug::Never && is_hugging_copied_macro(fmt, &exprs);
    let items = list_items_cast(node, |e| format_expr_item(fmt, e));
    let own_width = matches!(items.as_slice(), [item] if item.frag.own_width);
    let mut items = with_is_last(&items)
        .map(|(item, is_last)| {
            let ExprItem { frag, overflow, .. } = &item.frag;
            // Style Guide: a single multi-line argument is hugged (`Err(anyhow!(` / `...` / `))`)
            // unless the broken call takes fewer lines, except where `is_preferred_hug`
            let overflow = if !is_last {
                Overflow::Never
            } else if hugging_copied_macro {
                Overflow::With(fmt.dispreferred(frag.clone()))
            } else {
                hug.overflow(overflow)
            };
            item.map(|_| frag.clone()).with_overflow(overflow)
        })
        .collect::<Vec<ListItem>>();
    // A single argument that cannot overflow or has width rules of its own is not limited by
    // `fn_call_width`; another one is, and it is broken instead, as in rustfmt
    let flat = if own_width {
        FlatLimit::MaxWidth
    } else {
        FlatLimit::of_arguments(fmt, &items, hug, width)
    };
    // The limits of nested hugs add up, as in rustfmt: `Ok(A(b(` / `x,` / `)))` only while
    // `Ok(A(b(` fits `width`. The outer calls are preferred to hug (Style Guide).
    if let (FlatLimit::Within(width), [expr], [item]) =
        (flat, exprs.as_slice(), items.as_mut_slice())
        && is_delimited_list_expr(expr)
        && !ends_in_own_width_hug(expr)
        && !fmt.has_multi_line_text(&item.frag)
    {
        let depth = nested_hug_depth(expr);
        item.overflow = item.overflow.map(|layouts| {
            let layouts = fmt.first_line_within(layouts.clone(), width);
            if depth > 0 {
                fmt.weak_preference(layouts, WeakPreference::preferred_by(depth))
            } else {
                layouts
            }
        });
    }
    let layout = ListLayout::tall(flat).fill_if(is_fillable_list(fmt, &exprs, &items));
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

/// `(a,)` is a tuple, `(a)` just a parenthesized expression
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
            ("f(if a { b } else { c })", Preferred),
            ("f(match a {})", Preferred),
            ("f(unsafe { a })", Preferred),
            ("f(&match a {})", Preferred),
            ("f(match a.b() {})", Allowed),
            ("f(if let Some(a) = b { c } else { d })", Allowed),
            ("f(|x| x)", Allowed),
            ("f(a.b())", Allowed),
            ("f(g(a).await)", Allowed),
            ("f(a.await)", Never),
            ("f(a, |x| x)", Allowed),
            ("f(a, &|x| x)", Allowed),
            ("f(a, &mut move |x| x)", Allowed),
            ("f(&|x| x, &mut |y| y)", Never),
            ("f(|x| x, &|y| y)", Never),
            ("f(a, &{ b })", Allowed),
            ("f(a, &mut unsafe { b })", Allowed),
            ("f(|x| x, |y| y)", Never),
            ("f(a, unsafe { b })", Allowed),
            ("Poll::Ready(loop {})", Preferred),
            ("f('a: loop {})", Preferred),
            ("f(&loop {})", Preferred),
            ("f(while a {})", Preferred),
            ("f(while a.b() {})", Allowed),
            ("f(for x in y {})", Preferred),
            ("f(for x in y.iter() {})", Allowed),
            ("f(a, loop {})", Never),
            ("Ok(m! { a })", Preferred),
            ("f(&m! { a })", Preferred),
            ("f(a, m! { b })", Never),
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
            ("f(r#\"\na\n\"#)", Preferred),
            ("[ &\"a\nb\"]", Preferred),
            ("f(\n    \"a\nb\",\n)", Never),
            ("f(/* c */ \"a\nb\")", Never),
            ("f(\"a\")", Never),
            ("f(a, \"a\nb\")", Never),
            ("(\"a\nb\",)", Never),
        ] {
            assert_eq!(hug(code), expected, "{code}");
        }
    }
}
