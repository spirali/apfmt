use std::path::{Path, PathBuf};

use ra_ap_syntax::ast::{self, HasAttrs};
use ra_ap_syntax::{AstNode, SyntaxKind, SyntaxNode, SyntaxToken, TextSize};

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

pub(crate) fn with_is_last<T>(items: &[T]) -> impl Iterator<Item = (&T, bool)> {
    items
        .iter()
        .enumerate()
        .map(|(idx, item)| (item, idx + 1 == items.len()))
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

/// `path` absolute, with symbolic links and `..` resolved; as far as it can be (`path` itself when
/// it does not exist)
pub fn canonical_path(path: &Path) -> PathBuf {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    path.canonicalize().unwrap_or(path)
}

/// The first of the files `names` that exists in `dir`
pub(crate) fn file_in_dir(dir: &Path, names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}
