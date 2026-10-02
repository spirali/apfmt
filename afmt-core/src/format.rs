use ra_ap_syntax::{ast, PreorderWithTokens, RustLanguage, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, WalkEvent, AstNode, SourceFile, Direction};
use ra_ap_syntax::ast::{ArgList, BlockExpr, CallExpr, Expr, IdentPat, LetStmt, Literal, Name, ParamList, Pat, Path, PathExpr, RefType, Stmt, StmtList, TuplePat, Type, Fn as AstFn, Item, HasArgList, HasName, HasModuleItem};
use crate::doc::{DocId, DocNode, NEWLINE_DOC, NEWLINE_OR_SPACE_DOC};
use crate::formatter::{Branch, Edge, Formatter, Frag, Builder};
use crate::list::{list_comma_separated, list_nl_separated};
use super::formatter::Edge::{BraceClose, BraceOpen, Close, Comma, Keyword, Nothing, Open, Operator, Tight, Word};


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

/// Keyword that is always followed by a space, e.g. `let (a, b)`
fn format_keyword(fmt: &Formatter, token: SyntaxToken) -> Frag {
    Frag::new(fmt.make_node(DocNode::Token(token.clone())), Word, Keyword, Some(token))
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
        builder.push(format_token(fmt, l_paren, Open));
    } else {
        // The comments following `(` belong to the list, so each layout places them itself
        builder.push(format_token_no_follows(fmt, l_paren.clone(), Open));
        let own_line_comment = Builder::new_with(fmt, Some(l_paren.clone())).has_own_line_comment();
        let mut variants = list_nl_separated(fmt, &args, Some(l_paren.clone()));
        let flat = list_comma_separated(fmt, &args, Some(l_paren.clone()));
        // An own-line comment after `(` keeps the list vertical
        if !own_line_comment && (args.len() == 1 || fmt.simple_length_frag(&flat).is_some_and(|s| s as u32 <= fmt.config().fn_call_width)) {
            variants = fmt.alt_frag(variants, fmt.flat(flat));
        }
        if !own_line_comment && args.len() == 1 && let Some(expr) = arg_list.args().next() && is_combinable_expr(&expr) {
            let mut overflow = Builder::new_with(fmt, Some(l_paren));
            overflow.flush_trivia(false);
            overflow.push(args[0].clone());
            overflow.skip_delimiter(SyntaxKind::COMMA);
            overflow.flush_trivia(false);
            variants = fmt.alt_frag(variants, overflow.to_frag());
        }
        builder.push(variants);
    }
    builder.push(format_token(fmt, arg_list.r_paren_token().unwrap(), Close));
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

fn format_expr(fmt: &Formatter, expr: Expr) -> Frag {
    match expr {
        Expr::Literal(expr) => format_literal(fmt, expr),
        Expr::CallExpr(expr) => format_call_expr(fmt, expr),
        Expr::PathExpr(expr) => format_path_expr(fmt, expr),
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
        builder.push(format_token(fmt, token, Word));
    }
    if let Some(token) = pat.mut_token() {
        builder.push(format_token(fmt, token, Word));
    }
    builder.push(format_name(fmt, pat.name().unwrap()));
    if let Some(token) = pat.at_token() {
        builder.push(format_token(fmt, token, Operator));
        builder.push(format_pat(fmt, pat.pat().unwrap()));
    }
    builder.finish_node(pat.syntax())
}

fn format_tuple_pat(fmt: &Formatter, pat: TuplePat) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_token(fmt, pat.l_paren_token().unwrap(), Open));
    let fields = pat.fields().map(|p| format_pat(fmt, p)).collect::<Vec<Frag>>();
    if fields.len() == 1 {
        // `(a,)` is a tuple, `(a)` is just a parenthesized pattern
        builder.push(fields[0].clone());
        builder.push_delimiter(SyntaxKind::COMMA);
    } else {
        builder.push(list_comma_separated(fmt, &fields, None));
    }
    builder.push(format_token(fmt, pat.r_paren_token().unwrap(), Close));
    builder.finish_node(pat.syntax())
}

fn format_pat(fmt: &Formatter, pat: Pat) -> Frag {
    match pat {
        Pat::IdentPat(pat) => format_ident_pat(fmt, pat),
        Pat::WildcardPat(pat) => format_token(fmt, pat.underscore_token().unwrap(), Word),
        Pat::TuplePat(pat) => format_tuple_pat(fmt, pat),
        x => {
            panic!("{x:?}")
        }
    }
}

fn format_ref_type(fmt: &Formatter, ty: RefType) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_token(fmt, ty.amp_token().unwrap(), Tight));
    if let Some(lifetime) = ty.lifetime() {
        builder.push(format_token(fmt, lifetime.lifetime_ident_token().unwrap(), Word));
    }
    if let Some(token) = ty.mut_token() {
        builder.push(format_token(fmt, token, Word));
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
    let next_line = fmt.map_branches(rhs.clone(), |b| Branch {
        start: Nothing,
        doc_id: fmt.concat(NEWLINE_DOC, b.doc_id),
        ..b
    });
    fmt.alt_frag(rhs, fmt.nest(next_line))
}

fn format_let_stmt(fmt: &Formatter, stmt: LetStmt) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_keyword(fmt, stmt.let_token().unwrap()));
    builder.push(format_pat(fmt, stmt.pat().unwrap()));
    if let Some(ty) = stmt.ty() {
        builder.push(format_token(fmt, stmt.colon_token().unwrap(), Comma));
        builder.push(format_type(fmt, ty));
    }
    if let Some(init) = stmt.initializer() {
        builder.push(format_token(fmt, stmt.eq_token().unwrap(), Operator));
        builder.push(format_assign_rhs(fmt, format_expr(fmt, init)));
    }
    if let Some(let_else) = stmt.let_else() {
        builder.push(format_token(fmt, let_else.else_token().unwrap(), Word));
        builder.push(format_block_expr(fmt, let_else.block_expr().unwrap()));
    }
    builder.push_delimiter(SyntaxKind::SEMICOLON);
    builder.finish_node(stmt.syntax())
}

fn format_stmt(fmt: &Formatter, stmt: Stmt) -> Frag {
    match stmt {
        Stmt::ExprStmt(expr) => {
            let mut builder = Builder::new(fmt);

            builder.push(format_expr(fmt, expr.expr().unwrap()));
            builder.push_delimiter(SyntaxKind::SEMICOLON);
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
    let mut builder = Builder::new_with(fmt, Some(l_curly.clone()));
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
        let frag = fmt.map_branches(builder.to_frag(), |b| Branch {
            start: Nothing,
            doc_id: fmt.concat(separator, b.doc_id),
            ..b
        });
        fmt.nest(frag)
    } else {
        Frag::empty()
    };
    let mut builder = Builder::new(fmt);
    builder.push(format_token_no_follows(fmt, l_curly, BraceOpen));
    builder.push(body_frag);
    builder.push(format_token(fmt, stmt_list.r_curly_token().unwrap(), BraceClose));
    builder.finish_node(stmt_list.syntax())
}

fn format_block_expr(fmt: &Formatter, body: BlockExpr) -> Frag {
    format_stmt_list(fmt, body.stmt_list().unwrap())
}

fn format_param_list(fmt: &Formatter, param_list: ParamList) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_token(fmt, param_list.l_paren_token().unwrap(), Open));
    builder.push(format_token(fmt, param_list.r_paren_token().unwrap(), Close));
    builder.finish_node(param_list.syntax())
}

fn format_fn(fmt: &Formatter, item_fn: AstFn) -> Frag {
    let mut builder = Builder::new(fmt);
    builder.push(format_token(fmt, item_fn.fn_token().unwrap(), Word));
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