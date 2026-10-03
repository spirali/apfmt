use super::formatter::Edge::{BraceClose, BraceOpen, Close, Comma, Open, Operator, Tight, Word};
use crate::formatter::{Builder, Formatter, Frag};
use crate::list::{list_comma_separated, list_nl_separated, list_operator_separated};
use ra_ap_syntax::ast::{
    ArgList, AssocTypeArg, BinExpr, BlockExpr, CallExpr, ElseBranch, Expr, Fn as AstFn, GenericArg,
    GenericArgList, HasArgList, HasGenericArgs, HasModuleItem, HasName, HasTypeBounds, IdentPat,
    IfExpr, Item, LetExpr, LetStmt, Literal, Name, NameRef, ParamList, ParenExpr,
    ParenthesizedArgList, Pat, Path, PathExpr, PathSegment, PrefixExpr, RefType, RetType, Stmt,
    StmtList, TuplePat, TupleStructPat, Type, TypeAnchor,
};
use ra_ap_syntax::{
    AstNode, Direction, PreorderWithTokens, RustLanguage, SourceFile, SyntaxElement, SyntaxKind,
    SyntaxNode, SyntaxToken, WalkEvent, ast,
};

fn format_path_expr(fmt: &Formatter, path_expr: PathExpr) -> Frag {
    format_path(fmt, path_expr.path().unwrap())
}

/// `a::b::c`; the path is left-recursive: `(a::b)::c`
fn format_path(fmt: &Formatter, path: Path) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(qualifier) = path.qualifier() {
        builder.push(format_path(fmt, qualifier));
        builder.push_token(path.coloncolon_token().unwrap(), Tight);
    }
    builder.push(format_path_segment(fmt, path.segment().unwrap()));
    builder.to_frag()
}

/// `name`, `name<args>`, `name::<args>`, `Fn(args) -> ret` or `<T as Trait>`; with a leading `::` in `::std`
fn format_path_segment(fmt: &Formatter, segment: PathSegment) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = segment.coloncolon_token() {
        builder.push_token(token, Tight);
    }
    if let Some(name_ref) = segment.name_ref() {
        builder.push(format_name_ref(fmt, name_ref));
    }
    if let Some(anchor) = segment.type_anchor() {
        builder.push(format_type_anchor(fmt, anchor));
    }
    if let Some(args) = segment.generic_arg_list() {
        builder.push(format_generic_arg_list(fmt, args));
    }
    if let Some(args) = segment.parenthesized_arg_list() {
        builder.push(format_parenthesized_arg_list(fmt, args));
    }
    if let Some(ret_type) = segment.ret_type() {
        builder.push(format_ret_type(fmt, ret_type));
    }
    if let Some(x) = segment.return_type_syntax() {
        panic!("{x:?}");
    }
    builder.to_frag()
}

/// An identifier or one of `self`, `super`, `crate`, `Self`
fn format_name_ref(fmt: &Formatter, name_ref: NameRef) -> Frag {
    fmt.token(name_ref.syntax().first_token().unwrap(), Word)
}

/// `<T>` or `<T as Trait>` in `<T as Trait>::f`
fn format_type_anchor(fmt: &Formatter, anchor: TypeAnchor) -> Frag {
    let mut inner = Builder::new(fmt);
    inner.push(format_type(fmt, anchor.ty().unwrap()));
    if let Some(as_token) = anchor.as_token() {
        inner.push_token(as_token, Word);
        // `path_type()` returns the first path type, which is `T` in `<T as Trait>` (rust-analyzer#15778)
        let trait_ty = anchor
            .syntax()
            .children()
            .filter_map(Type::cast)
            .nth(1)
            .unwrap();
        inner.push(format_type(fmt, trait_ty));
    }
    format_parenthesized(
        fmt,
        anchor.l_angle_token().unwrap(),
        inner.to_frag(),
        anchor.r_angle_token().unwrap(),
    )
}

/// `<A, B>`, or the turbofish `::<A, B>`
fn format_generic_arg_list(fmt: &Formatter, args: GenericArgList) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = args.coloncolon_token() {
        builder.push_token(token, Tight);
    }
    let items = args
        .generic_args()
        .map(|a| format_generic_arg(fmt, a))
        .collect::<Vec<Frag>>();
    let inner = list_comma_separated(fmt, &items, None);
    builder.push(format_parenthesized(
        fmt,
        args.l_angle_token().unwrap(),
        inner,
        args.r_angle_token().unwrap(),
    ));
    builder.to_frag()
}

fn format_generic_arg(fmt: &Formatter, arg: GenericArg) -> Frag {
    match arg {
        GenericArg::TypeArg(arg) => format_type(fmt, arg.ty().unwrap()),
        GenericArg::LifetimeArg(arg) => fmt.token(arg.lifetime().unwrap().lifetime_ident_token().unwrap(), Word),
        GenericArg::ConstArg(arg) => format_expr(fmt, arg.expr().unwrap()),
        GenericArg::AssocTypeArg(arg) => format_assoc_type_arg(fmt, arg),
    }
}

/// `Item = u32` in `Iterator<Item = u32>`
fn format_assoc_type_arg(fmt: &Formatter, arg: AssocTypeArg) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_name_ref(fmt, arg.name_ref().unwrap()));
    if let Some(args) = arg.generic_arg_list() {
        builder.push(format_generic_arg_list(fmt, args));
    }
    let (Some(eq_token), None) = (arg.eq_token(), arg.type_bound_list()) else {
        panic!("{arg:?}");
    };
    builder.push_token(eq_token, Operator);
    match (arg.ty(), arg.const_arg()) {
        (Some(ty), _) => builder.push(format_type(fmt, ty)),
        (None, Some(arg)) => builder.push(format_expr(fmt, arg.expr().unwrap())),
        (None, None) => panic!("{arg:?}"),
    }
    builder.to_frag()
}

/// `(A, B)` in `Fn(A, B) -> C`
fn format_parenthesized_arg_list(fmt: &Formatter, args: ParenthesizedArgList) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = args.coloncolon_token() {
        builder.push_token(token, Tight);
    }
    let items = args
        .type_args()
        .map(|a| format_type(fmt, a.ty().unwrap()))
        .collect::<Vec<Frag>>();
    let inner = list_comma_separated(fmt, &items, None);
    builder.push(format_parenthesized(
        fmt,
        args.l_paren_token().unwrap(),
        inner,
        args.r_paren_token().unwrap(),
    ));
    builder.to_frag()
}

/// `-> T`
fn format_ret_type(fmt: &Formatter, ret_type: RetType) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(ret_type.thin_arrow_token().unwrap(), Operator);
    builder.push(format_type(fmt, ret_type.ty().unwrap()));
    builder.to_frag()
}

fn format_name(fmt: &Formatter, name: Name) -> Frag {
    fmt.token(name.ident_token().unwrap(), Word)
}

/// Whether a single argument can overflow, i.e. start on the line of `(` and end on the line of `)`:
/// `f(if a {\n ...\n})`. Follows the combinable expressions of the style guide.
// TODO: closures, `match`, loops, struct literals, arrays, tuples, method calls and macros once they are supported
fn is_combinable_expr(expr: &Expr) -> bool {
    match expr {
        Expr::CallExpr(_) | Expr::IfExpr(_) | Expr::BlockExpr(_) => true,
        // `f(!g(\n...\n))`
        Expr::PrefixExpr(expr) => expr.expr().is_some_and(|e| is_combinable_expr(&e)),
        _ => false,
    }
}

fn format_arg_list(fmt: &Formatter, arg_list: ArgList) -> Frag {
    let mut builder = Builder::new(fmt);
    let l_paren = arg_list.l_paren_token().unwrap();

    let args = arg_list
        .args()
        .map(|a| format_expr(fmt, a.clone()))
        .collect::<Vec<Frag>>();
    if args.is_empty() {
        builder.push_token(l_paren, Open);
    } else {
        // The comments following `(` belong to the list, so each layout places them itself
        builder.push(fmt.token_no_follows(l_paren.clone(), Open));
        let own_line_comment =
            Builder::with_token(fmt, Some(l_paren.clone())).has_own_line_comment();
        let mut variants = list_nl_separated(fmt, &args, Some(l_paren.clone()));
        // An own-line comment after `(` keeps the list vertical
        if !own_line_comment {
            let flat = list_comma_separated(fmt, &args, Some(l_paren.clone()));
            // A single argument is not limited by `fn_call_width`
            let flat = if args.len() == 1 {
                Some(fmt.flat(flat))
            } else {
                fmt.flat_within(flat, fmt.config().fn_call_width)
            };
            variants = fmt.alt_opt(variants, flat);
        }
        if !own_line_comment
            && args.len() == 1
            && let Some(expr) = arg_list.args().next()
            && is_combinable_expr(&expr)
        {
            let mut overflow = Builder::with_token(fmt, Some(l_paren));
            overflow.flush_trivia(false);
            overflow.push(args[0].clone());
            overflow.skip_delimiter(SyntaxKind::COMMA);
            overflow.flush_trivia(false);
            variants = fmt.alt_frag(variants, overflow.to_frag());
        }
        builder.push(variants);
    }
    builder.push_token(arg_list.r_paren_token().unwrap(), Close);
    builder.to_frag()
}

fn format_call_expr(fmt: &Formatter, call: CallExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_expr(fmt, call.expr().unwrap()));
    builder.push(format_arg_list(fmt, call.arg_list().unwrap()));
    builder.to_frag()
}

fn format_literal(fmt: &Formatter, literal: Literal) -> Frag {
    fmt.token(literal.token(), Word)
}

/// Splits a chain of the same binary operator, e.g. `a && b && c`, into the first operand
/// (the return value) and the (operator, operand) pairs that follow it
fn collect_bin_chain(expr: BinExpr, rest: &mut Vec<(SyntaxToken, Expr)>) -> Expr {
    let lhs = expr.lhs().unwrap();
    let first = match &lhs {
        Expr::BinExpr(inner) if inner.op_kind() == expr.op_kind() => {
            collect_bin_chain(inner.clone(), rest)
        }
        _ => lhs,
    };
    rest.push((expr.op_token().unwrap(), expr.rhs().unwrap()));
    first
}

fn format_bin_expr(fmt: &Formatter, expr: BinExpr) -> Frag {
    let mut rest = Vec::new();
    let first = format_expr(fmt, collect_bin_chain(expr, &mut rest));
    let rest = rest
        .into_iter()
        .map(|(op, e)| (fmt.token(op, Operator), format_expr(fmt, e)))
        .collect::<Vec<_>>();
    list_operator_separated(fmt, first, &rest)
}

fn format_prefix_expr(fmt: &Formatter, expr: PrefixExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(expr.op_token().unwrap(), Tight);
    builder.push(format_expr(fmt, expr.expr().unwrap()));
    builder.to_frag()
}

/// `(inner)`, `[inner]` or `<inner>`
fn format_parenthesized(
    fmt: &Formatter,
    open: SyntaxToken,
    inner: Frag,
    close: SyntaxToken,
) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(open, Open);
    builder.push(inner);
    builder.push_token(close, Close);
    builder.to_frag()
}

/// `{ inner }` of a block or a struct literal
fn format_braced(fmt: &Formatter, open: SyntaxToken, inner: Frag, close: SyntaxToken) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(open, BraceOpen);
    builder.push(inner);
    builder.push_token(close, BraceClose);
    builder.to_frag()
}

fn format_paren_expr(fmt: &Formatter, expr: ParenExpr) -> Frag {
    let inner = format_expr(fmt, expr.expr().unwrap());
    format_parenthesized(
        fmt,
        expr.l_paren_token().unwrap(),
        inner,
        expr.r_paren_token().unwrap(),
    )
}

/// `let pat = expr` in a condition
fn format_let_expr(fmt: &Formatter, expr: LetExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_keyword(expr.let_token().unwrap());
    builder.push(format_pat(fmt, expr.pat().unwrap()));
    builder.push_token(expr.eq_token().unwrap(), Operator);
    builder.push(format_expr(fmt, expr.expr().unwrap()));
    builder.to_frag()
}

/// Whether a multi-line layout of the condition ends with an indented line, e.g. a broken `a && b`;
/// then `{` has to go on its own line, so it is not mistaken for a part of the condition
fn is_hanging_condition(expr: &Expr) -> bool {
    match expr {
        Expr::BinExpr(_) => true,
        Expr::LetExpr(expr) => expr.expr().is_some_and(|e| is_hanging_condition(&e)),
        Expr::PrefixExpr(expr) => expr.expr().is_some_and(|e| is_hanging_condition(&e)),
        _ => false,
    }
}

/// `if cond {`, or `if cond` + `{` on the next line when the condition does not fit or is broken
fn format_if_head(fmt: &Formatter, expr: &IfExpr, condition: Frag, then_branch: Frag) -> Frag {
    let hanging = is_hanging_condition(&expr.condition().unwrap());
    let mut builder = Builder::new(fmt);
    builder.push_keyword(expr.if_token().unwrap());

    let mut same_line = builder.clone();
    same_line.push(if hanging {
        fmt.flat(condition.clone())
    } else {
        condition.clone()
    });
    same_line.push(then_branch.clone());

    let mut next_line = builder.clone();
    next_line.push(condition);
    next_line.push(fmt.on_next_line(then_branch));
    fmt.alt_frag(same_line.to_frag(), next_line.to_frag())
}

/// The expression of a block that can be written as `{ expr }`: no statements and no comments
fn single_line_block_expr(block: &BlockExpr) -> Option<Expr> {
    let stmt_list = block.stmt_list()?;
    let has_comment = stmt_list
        .syntax()
        .children_with_tokens()
        .any(|e| e.kind() == SyntaxKind::COMMENT);
    if stmt_list.statements().next().is_some() || has_comment || block.label().is_some() {
        return None;
    }
    stmt_list.tail_expr()
}

/// `{ inner }`, where `inner` is the expression from `single_line_block_expr`; the caller restricts it to a single line
fn format_block_single_line(fmt: &Formatter, block: &BlockExpr, inner: Frag) -> Frag {
    let stmt_list = block.stmt_list().unwrap();
    format_braced(
        fmt,
        stmt_list.l_curly_token().unwrap(),
        inner,
        stmt_list.r_curly_token().unwrap(),
    )
}

/// The expressions of the blocks of `if a { b } else { c }` that can be put on a single line:
/// a single `else` and blocks containing just an expression. Only a syntactic check, nothing is formatted.
fn single_line_if_exprs(expr: &IfExpr) -> Option<(Expr, Expr)> {
    let Some(ElseBranch::Block(else_block)) = expr.else_branch() else {
        return None;
    };
    Some((single_line_block_expr(&expr.then_branch()?)?, single_line_block_expr(&else_block)?))
}

/// `if a { b } else { c }` on a single line; `then_expr` and `else_expr` are the formatted
/// expressions from `single_line_if_exprs`. It is allowed only in an expression context
/// (not for a standalone statement).
fn format_if_single_line(fmt: &Formatter, expr: &IfExpr, condition: Frag, then_expr: Frag, else_expr: Frag) -> Option<Frag> {
    let Some(ElseBranch::Block(else_block)) = expr.else_branch() else {
        unreachable!("checked by `single_line_if_exprs`");
    };
    let mut builder = Builder::new(fmt);
    builder.push_keyword(expr.if_token()?);
    builder.push(condition);
    builder.push(format_block_single_line(fmt, &expr.then_branch()?, then_expr));
    builder.push_keyword(expr.else_token()?);
    builder.push(format_block_single_line(fmt, &else_block, else_expr));
    fmt.flat_within(
        builder.to_frag(),
        fmt.config().single_line_if_else_max_width,
    )
}

fn format_if_expr(fmt: &Formatter, expr: IfExpr, allow_single_line: bool) -> Frag {
    // The condition and the block expressions are formatted once and shared by the multi-line
    // and the single-line layout; otherwise nested `if`s would be formatted exponentially many times
    let condition = format_expr(fmt, expr.condition().unwrap());
    let block_exprs = allow_single_line
        .then(|| single_line_if_exprs(&expr))
        .flatten()
        .map(|(then_expr, else_expr)| (format_expr(fmt, then_expr), format_expr(fmt, else_expr)));
    let (then_tail, else_tail) = block_exprs.clone().unzip();

    let mut builder = Builder::new(fmt);
    builder.push(format_if_head(
        fmt,
        &expr,
        condition.clone(),
        format_block_expr_with_tail(fmt, expr.then_branch().unwrap(), then_tail),
    ));
    if let Some(else_branch) = expr.else_branch() {
        // `else` is not a continuation of the block: own-line comments in front of it keep the indentation of `}`
        builder.flush_trivia(false);
        builder.push_keyword(expr.else_token().unwrap());
        builder.push(match else_branch {
            ElseBranch::Block(block) => format_block_expr_with_tail(fmt, block, else_tail),
            ElseBranch::IfExpr(expr) => format_if_expr(fmt, expr, false),
        });
    }
    let frag = builder.to_frag();
    fmt.alt_opt(
        frag,
        block_exprs.and_then(|(then_expr, else_expr)| {
            format_if_single_line(fmt, &expr, condition, then_expr, else_expr)
        }),
    )
}

fn format_expr(fmt: &Formatter, expr: Expr) -> Frag {
    match expr {
        Expr::Literal(expr) => format_literal(fmt, expr),
        Expr::CallExpr(expr) => format_call_expr(fmt, expr),
        Expr::PathExpr(expr) => format_path_expr(fmt, expr),
        Expr::BinExpr(expr) => format_bin_expr(fmt, expr),
        Expr::PrefixExpr(expr) => format_prefix_expr(fmt, expr),
        Expr::ParenExpr(expr) => format_paren_expr(fmt, expr),
        Expr::LetExpr(expr) => format_let_expr(fmt, expr),
        Expr::IfExpr(expr) => format_if_expr(fmt, expr, true),
        Expr::BlockExpr(expr) => format_block_expr(fmt, expr),
        x => {
            panic! {
                "{x:?}"
            }
        }
    }
}

fn format_ident_pat(fmt: &Formatter, pat: IdentPat) -> Frag {
    let mut builder = Builder::new(fmt);
    if let Some(token) = pat.ref_token() {
        builder.push_token(token, Word);
    }
    if let Some(token) = pat.mut_token() {
        builder.push_token(token, Word);
    }
    builder.push(format_name(fmt, pat.name().unwrap()));
    if let Some(token) = pat.at_token() {
        builder.push_token(token, Operator);
        builder.push(format_pat(fmt, pat.pat().unwrap()));
    }
    builder.to_frag()
}

fn format_tuple_pat(fmt: &Formatter, pat: TuplePat) -> Frag {
    let fields = pat
        .fields()
        .map(|p| format_pat(fmt, p))
        .collect::<Vec<Frag>>();
    let inner = if fields.len() == 1 {
        // `(a,)` is a tuple, `(a)` is just a parenthesized pattern
        let mut builder = Builder::new(fmt);
        builder.push(fields[0].clone());
        builder.push_delimiter(SyntaxKind::COMMA);
        builder.to_frag()
    } else {
        list_comma_separated(fmt, &fields, None)
    };
    format_parenthesized(
        fmt,
        pat.l_paren_token().unwrap(),
        inner,
        pat.r_paren_token().unwrap(),
    )
}

fn format_tuple_struct_pat(fmt: &Formatter, pat: TupleStructPat) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_path(fmt, pat.path().unwrap()));
    let fields = pat
        .fields()
        .map(|p| format_pat(fmt, p))
        .collect::<Vec<Frag>>();
    let inner = list_comma_separated(fmt, &fields, None);
    builder.push(format_parenthesized(
        fmt,
        pat.l_paren_token().unwrap(),
        inner,
        pat.r_paren_token().unwrap(),
    ));
    builder.to_frag()
}

fn format_pat(fmt: &Formatter, pat: Pat) -> Frag {
    match pat {
        Pat::IdentPat(pat) => format_ident_pat(fmt, pat),
        Pat::WildcardPat(pat) => fmt.token(pat.underscore_token().unwrap(), Word),
        Pat::TuplePat(pat) => format_tuple_pat(fmt, pat),
        Pat::TupleStructPat(pat) => format_tuple_struct_pat(fmt, pat),
        x => {
            panic!("{x:?}")
        }
    }
}

fn format_ref_type(fmt: &Formatter, ty: RefType) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(ty.amp_token().unwrap(), Tight);
    if let Some(lifetime) = ty.lifetime() {
        builder.push_token(lifetime.lifetime_ident_token().unwrap(), Word);
    }
    if let Some(token) = ty.mut_token() {
        builder.push_token(token, Word);
    }
    builder.push(format_type(fmt, ty.ty().unwrap()));
    builder.to_frag()
}

fn format_type(fmt: &Formatter, ty: Type) -> Frag {
    match ty {
        Type::PathType(ty) => format_path(fmt, ty.path().unwrap()),
        Type::RefType(ty) => format_ref_type(fmt, ty),
        x => {
            panic!("{x:?}")
        }
    }
}

fn format_let_stmt(fmt: &Formatter, stmt: LetStmt) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_keyword(stmt.let_token().unwrap());
    builder.push(format_pat(fmt, stmt.pat().unwrap()));
    if let Some(ty) = stmt.ty() {
        builder.push_token(stmt.colon_token().unwrap(), Comma);
        builder.push(format_type(fmt, ty));
    }
    if let Some(init) = stmt.initializer() {
        builder.push_token(stmt.eq_token().unwrap(), Operator);
        builder.push(fmt.same_or_indented_line(format_expr(fmt, init)));
    }
    if let Some(let_else) = stmt.let_else() {
        builder.push_token(let_else.else_token().unwrap(), Word);
        builder.push(format_block_expr(fmt, let_else.block_expr().unwrap()));
    }
    builder.push_delimiter(SyntaxKind::SEMICOLON);
    builder.to_frag()
}

fn format_stmt(fmt: &Formatter, stmt: Stmt) -> Frag {
    match stmt {
        Stmt::ExprStmt(expr) => {
            let mut builder = Builder::new(fmt);

            let inner = match expr.expr().unwrap() {
                // A standalone `if` is never put on a single line
                Expr::IfExpr(if_expr) => format_if_expr(fmt, if_expr, false),
                e => format_expr(fmt, e),
            };
            builder.push(inner);
            // Block-like expressions (`if`, ...) may stand without `;`
            if expr.semicolon_token().is_some() {
                builder.push_delimiter(SyntaxKind::SEMICOLON);
            }
            builder.to_frag()
        }
        Stmt::Item(_) => {
            todo!()
        }
        Stmt::LetStmt(stmt) => format_let_stmt(fmt, stmt),
    }
}

/// `tail` is the already formatted tail expression, when it is shared with another layout
fn format_stmt_list(fmt: &Formatter, stmt_list: StmtList, tail: Option<Frag>) -> Frag {
    let l_curly = stmt_list.l_curly_token().unwrap();
    let mut followers = l_curly
        .siblings_with_tokens(Direction::Next)
        .skip(1)
        .peekable();

    // Blank lines right after `{` are dropped; only whether the first element starts a new line matters
    let mut on_new_line = false;
    while let Some(ws) = followers.next_if(|e| e.kind() == SyntaxKind::WHITESPACE) {
        on_new_line |= ws.as_token().is_some_and(|t| t.text().contains('\n'));
    }

    // Tokens following `{` belong to the body, so they are nested with it
    let mut builder = Builder::with_token(fmt, Some(l_curly.clone()));
    // The cursor is kept, so `push_on_new_line` can tell whether the last comment belongs to the first statement
    builder.consume_comments();
    for child in stmt_list.statements() {
        builder.push_on_new_line(format_stmt(fmt, child));
    }
    if let Some(expr) = stmt_list.tail_expr() {
        builder.push_on_new_line(tail.unwrap_or_else(|| format_expr(fmt, expr)));
    }

    // Blank lines before `}` are dropped
    builder.flush_trivia(false);
    let body_frag = if !builder.is_empty() {
        // A comment on the line of `{` stays there if it fits
        let comment_on_brace_line = !on_new_line
            && followers
                .peek()
                .is_some_and(|e| e.kind() == SyntaxKind::COMMENT);
        builder.push_newline();
        let body = builder.to_frag();
        fmt.nest(if comment_on_brace_line { fmt.on_same_or_next_line(body) } else { fmt.on_next_line(body) })
    } else {
        Frag::empty()
    };
    let mut builder = Builder::new(fmt);
    builder.push(fmt.token_no_follows(l_curly, BraceOpen));
    builder.push(body_frag);
    builder.push_token(stmt_list.r_curly_token().unwrap(), BraceClose);
    builder.to_frag()
}

fn format_block_expr(fmt: &Formatter, body: BlockExpr) -> Frag {
    format_block_expr_with_tail(fmt, body, None)
}

/// See `format_stmt_list` for `tail`
fn format_block_expr_with_tail(fmt: &Formatter, body: BlockExpr, tail: Option<Frag>) -> Frag {
    format_stmt_list(fmt, body.stmt_list().unwrap(), tail)
}

fn format_param_list(fmt: &Formatter, param_list: ParamList) -> Frag {
    // Parameters are not supported yet
    format_parenthesized(
        fmt,
        param_list.l_paren_token().unwrap(),
        Frag::empty(),
        param_list.r_paren_token().unwrap(),
    )
}

fn format_fn(fmt: &Formatter, item_fn: AstFn) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(item_fn.fn_token().unwrap(), Word);
    builder.push(format_name(fmt, item_fn.name().unwrap()));
    builder.push(format_param_list(fmt, item_fn.param_list().unwrap()));
    builder.push(format_block_expr(fmt, item_fn.body().unwrap()));
    builder.to_frag()
}

fn format_item(fmt: &Formatter, item: Item) -> Frag {
    let mut builder = Builder::new(fmt);
    match item {
        Item::Fn(item) => builder.push(format_fn(fmt, item)),
        x => {
            panic!("{x:?}");
        }
    }
    builder.to_frag()
}

pub(crate) fn format_source_file(fmt: &Formatter, node: SourceFile) -> Frag {
    let mut builder = Builder::new(fmt);
    for child in node.items() {
        builder.push_on_new_line(format_item(fmt, child));
    }
    builder.flush_trivia(false);
    assert!(builder.get_token().is_none());
    builder.to_frag()
}
