use ra_ap_syntax::{ast, PreorderWithTokens, RustLanguage, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, WalkEvent, AstNode, SourceFile, Direction};
use ra_ap_syntax::ast::{ArgList, BinExpr, BlockExpr, CallExpr, ElseBranch, Expr, IdentPat, IfExpr, LetExpr, LetStmt, Literal, Name, ParamList, ParenExpr, Pat, Path, PathExpr, PrefixExpr, RefType, Stmt, StmtList, TuplePat, TupleStructPat, Type, Fn as AstFn, Item, HasArgList, HasName, HasModuleItem};
use crate::doc::{DocId, DocNode, NEWLINE_DOC, NEWLINE_OR_SPACE_DOC};
use crate::formatter::{Edge, Formatter, Frag, Builder};
use crate::list::{list_comma_separated, list_nl_separated};
use super::formatter::Edge::{BraceClose, BraceOpen, Close, Comma, Open, Operator, Tight, Word};


fn format_path_expr(fmt: &Formatter, path_expr: PathExpr) -> Frag {
    format_path(fmt, path_expr.path().unwrap())
}

fn format_path(fmt: &Formatter, path: Path) -> Frag {
    let doc_id = fmt.make_node(DocNode::Text(path.syntax().text().to_string()));
    Frag::new_simple(doc_id, Word, path.syntax().last_token())
}

fn format_token(fmt: &Formatter, token: SyntaxToken, edge: Edge) -> Frag {
    fmt.token(token, edge)
}

fn format_name(fmt: &Formatter, name: Name) -> Frag {
    format_token(fmt, name.ident_token().unwrap(), Word)
}

/// Formats just the token; the caller is responsible for the tokens that follow it
fn format_token_no_follows(fmt: &Formatter, token: SyntaxToken, edge: Edge) -> Frag {
    Frag::new_simple(fmt.make_node(DocNode::Token(token)), edge, None)
}

fn is_combinable_expr(expr: &Expr) -> bool {
    // TODO: Add other ...
    matches!(expr, Expr::CallExpr(_))
}

fn format_arg_list(fmt: &Formatter, arg_list: ArgList) -> Frag {
    let mut builder = Builder::new(fmt);
    let l_paren = arg_list.l_paren_token().unwrap();

    let args = arg_list.args().map(|a| format_expr(fmt, a.clone())).collect::<Vec<Frag>>();
    if args.is_empty() {
        builder.push_token(l_paren, Open);
    } else {
        // The comments following `(` belong to the list, so each layout places them itself
        builder.push(format_token_no_follows(fmt, l_paren.clone(), Open));
        let own_line_comment = Builder::with_token(fmt, Some(l_paren.clone())).has_own_line_comment();
        let mut variants = list_nl_separated(fmt, &args, Some(l_paren.clone()));
        let flat = list_comma_separated(fmt, &args, Some(l_paren.clone()));
        // An own-line comment after `(` keeps the list vertical
        if !own_line_comment && (args.len() == 1 || fmt.simple_length_frag(&flat).is_some_and(|s| s as u32 <= fmt.config().fn_call_width)) {
            variants = fmt.alt_frag(variants, fmt.flat(flat));
        }
        if !own_line_comment && args.len() == 1 && let Some(expr) = arg_list.args().next() && is_combinable_expr(&expr) {
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
    builder.finish_node(arg_list.syntax())
}

fn format_call_expr(fmt: &Formatter, call: CallExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_expr(fmt, call.expr().unwrap()));
    builder.push(format_arg_list(fmt, call.arg_list().unwrap()));
    builder.finish_node(call.syntax())
}

fn format_literal(fmt: &Formatter, literal: Literal) -> Frag {
    format_token(fmt, literal.token(), Word)
}

/// Operands of a chain of the same binary operator, e.g. `a && b && c`, with the operator in front
/// of each operand except the first
fn collect_bin_chain(expr: BinExpr, out: &mut Vec<(Option<SyntaxToken>, Expr)>) {
    let lhs = expr.lhs().unwrap();
    match &lhs {
        Expr::BinExpr(inner) if inner.op_kind() == expr.op_kind() => collect_bin_chain(inner.clone(), out),
        _ => out.push((None, lhs)),
    }
    out.push((expr.op_token(), expr.rhs().unwrap()));
}

fn format_bin_expr(fmt: &Formatter, expr: BinExpr) -> Frag {
    let mut operands = Vec::new();
    collect_bin_chain(expr.clone(), &mut operands);
    let operands = operands
        .into_iter()
        .map(|(op, e)| (op.map(|op| format_token(fmt, op, Operator)), format_expr(fmt, e)))
        .collect::<Vec<_>>();

    // Everything on one line
    let mut flat = Builder::new(fmt);
    for (op, operand) in &operands {
        if let Some(op) = op {
            flat.push(op.clone());
        }
        flat.push(operand.clone());
    }

    // Breaks before every operator; the continuation lines are indented
    let mut vertical = Builder::new(fmt);
    for (op, operand) in &operands {
        let Some(op) = op else {
            vertical.push(operand.clone());
            continue;
        };
        let mut line = Builder::new(fmt);
        line.push(op.clone());
        line.push(operand.clone());
        vertical.push(fmt.indented_line(line.to_frag()));
    }
    fmt.alt_frag(fmt.flat(flat.to_frag()), vertical.to_frag())
}

fn format_prefix_expr(fmt: &Formatter, expr: PrefixExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(expr.op_token().unwrap(), Tight);
    builder.push(format_expr(fmt, expr.expr().unwrap()));
    builder.finish_node(expr.syntax())
}

fn format_paren_expr(fmt: &Formatter, expr: ParenExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(expr.l_paren_token().unwrap(), Open);
    builder.push(format_expr(fmt, expr.expr().unwrap()));
    builder.push_token(expr.r_paren_token().unwrap(), Close);
    builder.finish_node(expr.syntax())
}

/// `let pat = expr` in a condition
fn format_let_expr(fmt: &Formatter, expr: LetExpr) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_keyword(expr.let_token().unwrap());
    builder.push(format_pat(fmt, expr.pat().unwrap()));
    builder.push_token(expr.eq_token().unwrap(), Operator);
    builder.push(format_expr(fmt, expr.expr().unwrap()));
    builder.finish_node(expr.syntax())
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
fn format_if_head(fmt: &Formatter, expr: &IfExpr, then_branch: Frag) -> Frag {
    let condition = expr.condition().unwrap();
    let hanging = is_hanging_condition(&condition);
    let condition = format_expr(fmt, condition);
    let mut builder = Builder::new(fmt);
    builder.push_keyword(expr.if_token().unwrap());

    let mut same_line = builder.clone();
    same_line.push(if hanging { fmt.flat(condition.clone()) } else { condition.clone() });
    same_line.push(then_branch.clone());

    let mut next_line = builder.clone();
    next_line.push(condition);
    next_line.push(fmt.on_next_line(then_branch));
    fmt.alt_frag(same_line.to_frag(), next_line.to_frag())
}

/// `{ expr }` on a single line; only for blocks that contain just an expression and no comments
fn format_block_single_line(fmt: &Formatter, block: &BlockExpr) -> Option<Frag> {
    let stmt_list = block.stmt_list()?;
    let expr = stmt_list.tail_expr()?;
    let has_comment = stmt_list.syntax().children_with_tokens().any(|e| e.kind() == SyntaxKind::COMMENT);
    if stmt_list.statements().next().is_some() || has_comment || block.label().is_some() {
        return None;
    }
    let mut builder = Builder::new(fmt);
    builder.push_token(stmt_list.l_curly_token()?, BraceOpen);
    builder.push(format_expr(fmt, expr));
    builder.push_token(stmt_list.r_curly_token()?, BraceClose);
    Some(fmt.flat(builder.finish_node(stmt_list.syntax())))
}

/// `if a { b } else { c }` on a single line. It is allowed only in an expression context
/// (not for a standalone statement), with a single `else` and with blocks containing just an expression.
fn format_if_single_line(fmt: &Formatter, expr: &IfExpr) -> Option<Frag> {
    let Some(ElseBranch::Block(else_block)) = expr.else_branch() else {
        return None;
    };
    let then_branch = format_block_single_line(fmt, &expr.then_branch()?)?;
    let else_branch = format_block_single_line(fmt, &else_block)?;
    let mut builder = Builder::new(fmt);
    builder.push_keyword(expr.if_token()?);
    builder.push(format_expr(fmt, expr.condition()?));
    builder.push(then_branch);
    builder.push_keyword(expr.else_token()?);
    builder.push(else_branch);
    let frag = fmt.flat(builder.finish_node(expr.syntax()));
    fmt.simple_length_frag(&frag)
        .is_some_and(|size| size as u32 <= fmt.config().single_line_if_else_max_width)
        .then_some(frag)
}

fn format_if_expr(fmt: &Formatter, expr: IfExpr, allow_single_line: bool) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_if_head(fmt, &expr, format_block_expr(fmt, expr.then_branch().unwrap())));
    if let Some(else_branch) = expr.else_branch() {
        // `else` is not a continuation of the block: own-line comments in front of it keep the indentation of `}`
        builder.flush_trivia(false);
        builder.push_keyword(expr.else_token().unwrap());
        builder.push(match else_branch {
            ElseBranch::Block(block) => format_block_expr(fmt, block),
            ElseBranch::IfExpr(expr) => format_if_expr(fmt, expr, false),
        });
    }
    let frag = builder.finish_node(expr.syntax());
    match allow_single_line.then(|| format_if_single_line(fmt, &expr)).flatten() {
        Some(single_line) => fmt.alt_frag(single_line, frag),
        None => frag,
    }
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
            panic!{
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
    builder.finish_node(pat.syntax())
}

fn format_tuple_pat(fmt: &Formatter, pat: TuplePat) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(pat.l_paren_token().unwrap(), Open);
    let fields = pat.fields().map(|p| format_pat(fmt, p)).collect::<Vec<Frag>>();
    if fields.len() == 1 {
        // `(a,)` is a tuple, `(a)` is just a parenthesized pattern
        builder.push(fields[0].clone());
        builder.push_delimiter(SyntaxKind::COMMA);
    } else {
        builder.push(list_comma_separated(fmt, &fields, None));
    }
    builder.push_token(pat.r_paren_token().unwrap(), Close);
    builder.finish_node(pat.syntax())
}

fn format_tuple_struct_pat(fmt: &Formatter, pat: TupleStructPat) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_path(fmt, pat.path().unwrap()));
    builder.push_token(pat.l_paren_token().unwrap(), Open);
    let fields = pat.fields().map(|p| format_pat(fmt, p)).collect::<Vec<Frag>>();
    builder.push(list_comma_separated(fmt, &fields, None));
    builder.push_token(pat.r_paren_token().unwrap(), Close);
    builder.finish_node(pat.syntax())
}

fn format_pat(fmt: &Formatter, pat: Pat) -> Frag {
    match pat {
        Pat::IdentPat(pat) => format_ident_pat(fmt, pat),
        Pat::WildcardPat(pat) => format_token(fmt, pat.underscore_token().unwrap(), Word),
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
    builder.finish_node(ty.syntax())
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

fn format_assign_rhs(fmt: &Formatter, rhs: Frag) -> Frag {
    fmt.alt_frag(rhs.clone(), fmt.indented_line(rhs))
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
        builder.push(format_assign_rhs(fmt, format_expr(fmt, init)));
    }
    if let Some(let_else) = stmt.let_else() {
        builder.push_token(let_else.else_token().unwrap(), Word);
        builder.push(format_block_expr(fmt, let_else.block_expr().unwrap()));
    }
    builder.push_delimiter(SyntaxKind::SEMICOLON);
    builder.finish_node(stmt.syntax())
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
            builder.finish_node(expr.syntax())
        }
        Stmt::Item(_) => {
            todo!()
        }
        Stmt::LetStmt(stmt) => format_let_stmt(fmt, stmt),
    }
}

fn format_stmt_list(fmt: &Formatter, stmt_list: StmtList) -> Frag {
    let l_curly = stmt_list.l_curly_token().unwrap();
    let mut followers = l_curly.siblings_with_tokens(Direction::Next).skip(1).peekable();

    // Blank lines right after `{` are dropped; only whether the first element starts a new line matters
    let mut on_new_line = false;
    while let Some(ws) = followers.next_if(|e| e.kind() == SyntaxKind::WHITESPACE) {
        on_new_line |= ws.as_token().is_some_and(|t| t.text().contains('\n'));
    }

    // Tokens following `{` belong to the body, so they are nested with it
    let mut builder = Builder::with_token(fmt, Some(l_curly.clone()));
    // The cursor is kept, so `ensure_newline` can tell whether the last comment belongs to the first statement
    builder.consume_comments();
    for child in stmt_list.statements() {
        if !builder.is_empty() {
            builder.ensure_newline();
        }
        builder.push(format_stmt(fmt, child));
    }
    if let Some(expr) = stmt_list.tail_expr() {
        if !builder.is_empty() {
            builder.ensure_newline();
        }
        builder.push(format_expr(fmt, expr));
    }

    // Blank lines before `}` are dropped
    builder.flush_trivia(false);
    let body_frag = if !builder.is_empty() {
        // A comment on the line of `{` stays there if it fits
        let separator = if !on_new_line && followers.peek().is_some_and(|e| e.kind() == SyntaxKind::COMMENT) {
            NEWLINE_OR_SPACE_DOC
        } else {
            NEWLINE_DOC
        };
        builder.push_newline();
        fmt.nest(fmt.prefix_break(separator, builder.to_frag()))
    } else {
        Frag::empty()
    };
    let mut builder = Builder::new(fmt);
    builder.push(format_token_no_follows(fmt, l_curly, BraceOpen));
    builder.push(body_frag);
    builder.push_token(stmt_list.r_curly_token().unwrap(), BraceClose);
    builder.finish_node(stmt_list.syntax())
}

fn format_block_expr(fmt: &Formatter, body: BlockExpr) -> Frag {
    format_stmt_list(fmt, body.stmt_list().unwrap())
}

fn format_param_list(fmt: &Formatter, param_list: ParamList) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(param_list.l_paren_token().unwrap(), Open);
    builder.push_token(param_list.r_paren_token().unwrap(), Close);
    builder.finish_node(param_list.syntax())
}

fn format_fn(fmt: &Formatter, item_fn: AstFn) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push_token(item_fn.fn_token().unwrap(), Word);
    builder.push(format_name(fmt, item_fn.name().unwrap()));
    builder.push(format_param_list(fmt, item_fn.param_list().unwrap()));
    builder.push(format_block_expr(fmt, item_fn.body().unwrap()));
    builder.finish_node(item_fn.syntax())
}

fn format_item(fmt: &Formatter, item: Item) -> Frag {
    let mut builder = Builder::new(fmt);
    match item {
        Item::Fn(item) => {
            builder.push(format_fn(fmt, item))
        },
        x => {
            panic!("{x:?}");
        }
    }
    builder.to_frag()
}

pub(crate) fn format_source_file(fmt: &Formatter, node: SourceFile) -> DocId {
    let mut builder = Builder::new(fmt);
    for child in node.items() {
        if !builder.is_empty() {
            builder.ensure_newline();
        }
        builder.push(format_item(fmt, child));
    }
    builder.flush_trivia(false);
    assert!(builder.get_token().is_none());
    fmt.frag_to_doc(&builder.to_frag())
}