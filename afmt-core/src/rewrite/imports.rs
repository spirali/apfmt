//! Sorting and normalization of imports (Style Guide: "Imports"). Each `use` item of a group and each
//! name of a list import moves together with its comments, so the formatter places them as in any other
//! source.

use super::line_if_alone;
use super::sort::version_cmp;
use crate::comments::{
    TriviaStop, comments_ending_line_end, count_newlines, first_code_token, has_comments,
    is_comment_kind, is_line_comment, own_line_comment_follows, post_comments_end, trivia_end,
    trivia_from,
};
use crate::layout::list::list_items_cast;
use crate::syntax_utils::has_attrs;
use ra_ap_syntax::ast::{HasName, Path, Use, UseTree, UseTreeList};
use ra_ap_syntax::{AstNode, SourceFile, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TextSize};
use std::cmp::Ordering;

/// The edits that sort and normalize the imports of the file, one per group of imports that changes
pub(super) fn import_edits(file: &SourceFile, source: &str) -> Vec<(TextRange, String)> {
    let mut edits = Vec::new();
    for node in file.syntax().descendants() {
        if matches!(
            node.kind(),
            SyntaxKind::SOURCE_FILE | SyntaxKind::STMT_LIST | SyntaxKind::ITEM_LIST
        ) {
            for group in use_groups(&node) {
                let (range, text) = group_text(source, &group);
                if source[range] != text {
                    edits.push((range, text));
                }
            }
        }
    }
    edits
}

/// A tree that imports nothing, `a::{}` or `a::{b::{}}`, without comments; it is removed (Style Guide:
/// "Normalisation")
fn is_empty_tree(tree: &UseTree) -> bool {
    tree.use_tree_list().is_some_and(|list| {
        !has_comments(list.syntax()) && list.use_trees().all(|t| is_empty_tree(&t))
    })
}

/// `self` or `self as name`
fn is_self(tree: &UseTree) -> bool {
    tree.use_tree_list().is_none()
        && tree.path().is_some_and(|path| {
            path.qualifier().is_none()
                && path.segment().is_some_and(|s| {
                    s.coloncolon_token().is_none()
                        && s.name_ref().is_some_and(|n| n.self_token().is_some())
                })
        })
}

/// The only item of `list` when the braces are dropped, `a::{b}` -> `a::b` (Style Guide:
/// "Normalisation"), unless the list has comments. `a::{self}` keeps them: it imports only the module
/// `a`, `a` would also import a function or a macro `a`.
fn unwrapped_item(list: &UseTreeList) -> Option<UseTree> {
    if has_comments(list.syntax()) {
        return None;
    }
    let mut trees = list.use_trees().filter(|tree| !is_empty_tree(tree));
    match (trees.next(), trees.next()) {
        (Some(tree), None) if !is_self(&tree) => Some(tree),
        _ => None,
    }
}

/// A name compared by version sorting; without the raw prefix: `r#type` sorts as `type`
#[derive(PartialEq, Eq)]
struct VersionName(String);

impl VersionName {
    fn new(text: &str) -> Self {
        VersionName(text.trim_start_matches("r#").to_string())
    }
}

impl Ord for VersionName {
    fn cmp(&self, other: &Self) -> Ordering {
        version_cmp(&self.0, &other.0)
    }
}

impl PartialOrd for VersionName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A segment of a use tree for sorting; the variants are in their order (Style Guide: "Ordering list
/// import": `self` and `super` first, globs and lists last; `crate` and the leading `::`, an empty name,
/// follow `super` as in rustfmt)
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Segment {
    SelfKw,
    Super,
    Crate,
    Name(VersionName),
    Glob,
    List(Vec<UseKey>),
}

/// A use tree in its normalized form for sorting: its segments (`a::b::{c, d}` is `a`, `b`, `{c, d}`
/// with the list sorted) and the alias of the last one. A shorter tree goes first when it is a prefix
/// of the other one, the alias decides last: `a`, `a as b`, `a::b`.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct UseKey {
    segments: Vec<Segment>,
    alias: Option<VersionName>,
}

/// A use tree (or a `use` item) sorted and normalized: its sort key and its text
struct Normalized {
    key: UseKey,
    text: String,
}

fn push_path_segments(path: &Path, segments: &mut Vec<Segment>) {
    if let Some(qualifier) = path.qualifier() {
        push_path_segments(&qualifier, segments);
    }
    let segment = path.segment().unwrap();
    if segment.coloncolon_token().is_some() {
        // The leading `::` in `::std`
        segments.push(Segment::Name(VersionName::new("")));
    }
    segments.push(match segment.name_ref() {
        Some(name) if name.self_token().is_some() => Segment::SelfKw,
        Some(name) if name.super_token().is_some() => Segment::Super,
        Some(name) if name.crate_token().is_some() => Segment::Crate,
        Some(name) => Segment::Name(VersionName::new(&name.to_string())),
        None => Segment::Name(VersionName::new(&segment.syntax().text().to_string())),
    });
}

/// `source[outer]` with its part `inner` replaced by `replacement`
fn splice(source: &str, outer: TextRange, inner: TextRange, replacement: &str) -> String {
    let before = &source[TextRange::new(outer.start(), inner.start())];
    let after = &source[TextRange::new(inner.end(), outer.end())];
    format!("{before}{replacement}{after}")
}

/// The tree with its list sorted and normalized (`a::{b}` is `a::b`), with its sort key
fn normalize(source: &str, tree: &UseTree) -> Normalized {
    let range = tree.syntax().text_range();
    let mut segments = Vec::new();
    let mut alias = tree.rename().map(|rename| match rename.name() {
        Some(name) => VersionName::new(&name.to_string()),
        None => VersionName::new("_"),
    });
    match tree.path() {
        Some(path) => push_path_segments(&path, &mut segments),
        // `::{a, b}`
        None if tree.coloncolon_token().is_some() => {
            segments.push(Segment::Name(VersionName::new("")))
        }
        None => {}
    }
    if tree.star_token().is_some() {
        segments.push(Segment::Glob);
    }
    let text = match tree.use_tree_list() {
        None => source[range].to_string(),
        Some(list) => {
            let list_text = match unwrapped_item(&list) {
                Some(item) => {
                    let item = normalize(source, &item);
                    segments.extend(item.key.segments);
                    alias = item.key.alias;
                    item.text
                }
                None => {
                    let (keys, text) = sorted_list(source, &list);
                    segments.push(Segment::List(keys));
                    text
                }
            };
            splice(source, range, list.syntax().text_range(), &list_text)
        }
    };
    Normalized {
        key: UseKey { segments, alias },
        text,
    }
}

/// An item of a list import with the source around it: its leading trivia from `start`, and its
/// separator with the post comments up to `end`
struct ListEntry {
    tree: Normalized,
    start: TextSize,
    range: TextRange,
    end: TextSize,
    has_comma: bool,
    /// The start of the line comment ending the post comments, which has to end the line
    line_comment: Option<TextSize>,
}

/// `{a, b}` with the items sorted and the empty ones removed (if the list has no comments), and the sort
/// keys of the items. Each item moves with its leading comments (everything after the previous item's
/// post comments) and its post comments; the comments ending the line of `{` and the comments in front
/// of `}` stay.
fn sorted_list(source: &str, list: &UseTreeList) -> (Vec<UseKey>, String) {
    let open = list.l_curly_token().unwrap();
    let close = list.r_curly_token().unwrap();
    let drop_empty = !has_comments(list.syntax());
    let mut entries = Vec::new();
    let mut before = comments_ending_line_end(&open);
    let opener = TextRange::new(open.text_range().end(), before.text_range().end());
    // Whether the text so far ends with a line comment, which has to end the line
    let mut line_comment = is_line_comment(&before);
    for item in list_items_cast(list.syntax(), |t: UseTree| t) {
        let tree = item.frag;
        let end = post_comments_end(&tree.syntax().last_token().unwrap(), SyntaxKind::COMMA);
        let start = std::mem::replace(&mut before, end.clone())
            .text_range()
            .end();
        if drop_empty && is_empty_tree(&tree) {
            continue;
        }
        entries.push(ListEntry {
            tree: normalize(source, &tree),
            start,
            range: tree.syntax().text_range(),
            end: end.text_range().end(),
            has_comma: item.comma.is_some(),
            line_comment: is_line_comment(&end).then(|| end.text_range().start()),
        });
    }
    let closer = TextRange::new(before.text_range().end(), close.text_range().start());
    entries.sort_by(|a, b| a.tree.key.cmp(&b.tree.key));

    let mut out = format!("{{{}", &source[opener]);
    let mut keys = Vec::new();
    let count = entries.len();
    for (i, entry) in entries.into_iter().enumerate() {
        let mut text = format!(
            "{}{}",
            &source[TextRange::new(entry.start, entry.range.start())],
            entry.tree.text
        );
        let post = &source[TextRange::new(entry.range.end(), entry.end)];
        // The item last in the source has no comma: it goes behind its comments, in front of a line
        // comment
        let add_comma = !entry.has_comma && i + 1 < count;
        let split = entry
            .line_comment
            .filter(|_| add_comma)
            .map_or(post.len(), |start| usize::from(start - entry.range.end()));
        text.push_str(&post[..split]);
        if add_comma {
            text.push(',');
        }
        text.push_str(&post[split..]);
        push_after(&mut out, &text, line_comment);
        line_comment = entry.line_comment.is_some();
        keys.push(entry.tree.key);
    }
    push_after(&mut out, &source[closer], line_comment);
    out.push('}');
    (keys, out)
}

/// Appends `text` to `out`, on a new line if `out` ends with a line comment
fn push_after(out: &mut String, text: &str, line_comment: bool) {
    let indent = text.len() - text.trim_start().len();
    if line_comment && !text[..indent].contains('\n') {
        out.push('\n');
    }
    out.push_str(text);
}

/// The `use` item, if `node` is one that can be sorted with others (attributes, e.g. `#[macro_use]`,
/// prevent it: Style Guide)
fn sortable_use(node: &SyntaxNode) -> Option<Use> {
    Use::cast(node.clone()).filter(|item| !has_attrs(item))
}

/// Whether the next `use` item follows `prev` in the same group of imports: on the next line, or on the
/// same line without comments in between. A blank line or a line with only comments starts a new group
/// (as in rustfmt).
fn is_same_group(prev: &Use) -> bool {
    let semicolon = prev.syntax().last_token().unwrap();
    let trivia = || trivia_from(semicolon.next_token());
    match trivia().map(|t| count_newlines(&t)).sum::<u32>() {
        0 => !trivia().any(|t| is_comment_kind(t.kind())),
        1 => !own_line_comment_follows(&semicolon),
        _ => false,
    }
}

/// The groups of imports among the children of `node`; a `use` item with attributes is a group alone
fn use_groups(node: &SyntaxNode) -> Vec<Vec<Use>> {
    let mut groups: Vec<Vec<Use>> = Vec::new();
    let mut children = node.children().peekable();
    while let Some(child) = children.next() {
        let Some(item) = Use::cast(child) else {
            continue;
        };
        let sortable = !has_attrs(&item);
        let mut group = vec![item];
        while sortable
            && let Some(next) = children.peek().and_then(sortable_use)
            && is_same_group(group.last().unwrap())
        {
            group.push(next);
            children.next();
        }
        groups.push(group);
    }
    groups
}

/// The source range of `item` from its first code token (rust-analyzer attaches the comments in front
/// of it to it) to the comments ending the line of its `;`, which move with it
fn use_range(item: &Use) -> TextRange {
    let start = first_code_token(item.syntax()).text_range().start();
    let end = comments_ending_line_end(&semicolon(item))
        .text_range()
        .end();
    TextRange::new(start, end)
}

fn semicolon(item: &Use) -> SyntaxToken {
    item.syntax().last_token().unwrap()
}

/// A `use` item that imports nothing (`use a::{};`) is removed, unless it has attributes, or comments in
/// it or after it on its line (comments in front of it stay)
fn is_removed(item: &Use) -> bool {
    let semicolon = semicolon(item);
    let start = first_code_token(item.syntax()).text_range().start();
    !has_attrs(item)
        && item.use_tree().is_some_and(|tree| is_empty_tree(&tree))
        && trivia_end(&semicolon, None, TriviaStop::OwnLine) == semicolon
        && !item
            .syntax()
            .descendants_with_tokens()
            .any(|e| is_comment_kind(e.kind()) && e.text_range().start() >= start)
}

/// The source range of the group (see `use_range`) and its sorted, normalized text: the items on their
/// own lines. If all of them are removed and the group is alone on its line, the line goes too.
fn group_text(source: &str, group: &[Use]) -> (TextRange, String) {
    let mut range = TextRange::new(
        use_range(&group[0]).start(),
        use_range(group.last().unwrap()).end(),
    );
    let mut items: Vec<Normalized> = group
        .iter()
        .filter(|item| !is_removed(item))
        .map(|item| {
            let tree = item.use_tree().unwrap();
            let normalized = normalize(source, &tree);
            let text = splice(
                source,
                use_range(item),
                tree.syntax().text_range(),
                &normalized.text,
            );
            Normalized { text, ..normalized }
        })
        .collect();
    if items.is_empty() {
        range = line_if_alone(source, range);
    }
    items.sort_by(|a, b| a.key.cmp(&b.key));
    let lines: Vec<String> = items.into_iter().map(|item| item.text).collect();
    (range, lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use crate::rewrite::rewrite_source;
    use ra_ap_syntax::{Edition, SourceFile};

    fn rewrite(source: &str) -> Option<String> {
        let file = SourceFile::parse(source, Edition::CURRENT);
        assert!(file.errors().is_empty());
        rewrite_source(&file.tree(), source)
    }

    #[test]
    fn sorted_source_is_kept() {
        assert_eq!(rewrite("use a;\nuse b::{c, d};\n\nuse a;\n"), None);
    }

    #[test]
    fn group_sorted_with_trailing_comments() {
        assert_eq!(
            rewrite("// first\nuse c; // c\nuse b;\n\nuse a;\n").unwrap(),
            "// first\nuse b;\nuse c; // c\n\nuse a;\n"
        );
    }

    #[test]
    fn comma_added_behind_block_comment() {
        assert_eq!(
            rewrite("use x::{b, a /* a */};").unwrap(),
            "use x::{ a /* a */,b,};"
        );
    }

    #[test]
    fn line_comment_ends_the_line() {
        assert_eq!(
            rewrite("use x::{b, a // a\n};").unwrap(),
            "use x::{ a ,// a\nb,\n};"
        );
        assert_eq!(
            rewrite("use x::{c, // c\n b, a};").unwrap(),
            "use x::{ a,\n b,c, // c\n};"
        );
    }

    #[test]
    fn normalized() {
        assert_eq!(rewrite("use a::{b::{c}};").unwrap(), "use a::b::c;");
        assert_eq!(rewrite("use a::{c, b::{}};").unwrap(), "use a::c;");
        assert_eq!(rewrite("use a::{self};"), None);
        assert_eq!(rewrite("use a::{ /* c */ b};"), None);
    }

    #[test]
    fn removed_line_goes() {
        assert_eq!(
            rewrite("use a;\nuse b::{};\n\nfn f() {}\n").unwrap(),
            "use a;\n\nfn f() {}\n"
        );
        assert_eq!(rewrite("use b::{}; // kept\n"), None);
        // With its indentation
        assert_eq!(
            rewrite("fn f() {\n    use a::{};\n    g();\n}\n").unwrap(),
            "fn f() {\n    g();\n}\n"
        );
        // Code in front of it on its line: the line stays, the comment below stays on its own line
        assert_eq!(
            rewrite("fn f() {\n    g(); use a::{};\n    // c\n}\n").unwrap(),
            "fn f() {\n    g(); \n    // c\n}\n"
        );
    }
}
