//! The files of a module tree: a crate root and the files its `mod name;` declarations pull in, found
//! the way rustc finds them. Every existing file is included, whatever its `cfg`, as rustfmt does.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ra_ap_parser::LexedStr;
use ra_ap_syntax::ast::{self, HasAttrs, HasModuleItem, HasName};
use ra_ap_syntax::{Edition, SourceFile, SyntaxKind};

use crate::comments::is_code_kind;
use crate::utils::canonical_path;

/// The files of a module tree, and the problems found while following it
#[derive(Debug, Default)]
pub struct ModuleFiles {
    /// The crate root first, then the files in the order found, each once, with its canonical path
    pub files: Vec<(PathBuf, String)>,
    /// A module whose file does not exist (e.g. for another platform), a file that cannot be read
    pub warnings: Vec<String>,
}

struct ModuleFile {
    path: PathBuf,
    /// A crate root, a `mod.rs` or a `#[path]` file has its submodules in its directory, `foo.rs`
    /// in `foo/`
    mod_rs: bool,
}

/// The files of the module tree whose root is `root` (`src/lib.rs`, `src/main.rs`, ...)
pub fn module_files(root: &Path, edition: Edition) -> ModuleFiles {
    let mut result = ModuleFiles::default();
    let mut seen = BTreeSet::new();
    let mut queue = vec![ModuleFile {
        path: root.to_path_buf(),
        mod_rs: true,
    }];
    while let Some(file) = queue.pop() {
        let canonical = canonical_path(&file.path);
        if !seen.insert(canonical.clone()) {
            continue;
        }
        let source = match std::fs::read_to_string(&file.path) {
            Ok(source) => source,
            Err(error) => {
                result
                    .warnings
                    .push(format!("cannot read {}: {error}", file.path.display()));
                continue;
            }
        };
        // Without a declaration, the file is not parsed: lexing is much cheaper
        if !may_declare_module_file(&source, edition) {
            result.files.push((canonical, source));
            continue;
        }
        // A file with syntax errors still has a tree; formatting it reports the errors
        let tree = SourceFile::parse(&source, edition).tree();
        let file_dir = file.path.parent().unwrap_or(Path::new("")).to_path_buf();
        let module_dir = if file.mod_rs {
            file_dir.clone()
        } else {
            file_dir.join(file.path.file_stem().unwrap_or_default())
        };
        let mut children = Vec::new();
        let parent = Parent {
            file: &file.path,
            file_dir: &file_dir,
            module_dir: &module_dir,
            inline: false,
        };
        collect_children(tree.items(), &parent, &mut children, &mut result.warnings);
        result.files.push((canonical, source));
        // Depth first, in the source order
        queue.extend(children.into_iter().rev());
    }
    result
}

/// Whether `source` may declare `mod name;`: the tokens `mod`, a name and `;` in a row; a false
/// positive only costs a parse
fn may_declare_module_file(source: &str, edition: Edition) -> bool {
    if !may_contain_mod_name_semicolon(source) {
        return false;
    }
    let lexed = LexedStr::new(edition, source);
    let code: Vec<SyntaxKind> = (0..lexed.len())
        .map(|i| lexed.kind(i))
        .filter(|kind| is_code_kind(*kind))
        .collect();
    code.windows(3)
        .any(|tokens| tokens == [SyntaxKind::MOD_KW, SyntaxKind::IDENT, SyntaxKind::SEMICOLON])
}

/// A byte scan cheaper than lexing, `false` only without `mod name;`; a comment or a non-ASCII byte
/// between the words counts as a match
fn may_contain_mod_name_semicolon(source: &str) -> bool {
    let bytes = source.as_bytes();
    let is_ascii_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    // The index after the ASCII whitespace from `i`; `None` for a comment or a non-ASCII byte
    let skip_space = |mut i: usize| {
        while let Some(&b) = bytes.get(i) {
            match b {
                b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c' => i += 1,
                b'/' => return None,
                _ if !b.is_ascii() => return None,
                _ => break,
            }
        }
        Some(i)
    };
    for (start, _) in source.match_indices("mod") {
        let after = start + 3;
        if start > 0 && is_ascii_word(bytes[start - 1])
            || bytes.get(after).is_some_and(|&b| is_ascii_word(b))
        {
            continue;
        }
        let Some(mut name_start) = skip_space(after) else {
            return true;
        };
        if source[name_start..].starts_with("r#") {
            name_start += 2;
        }
        let name_end = (name_start..bytes.len())
            .find(|&i| !is_ascii_word(bytes[i]))
            .unwrap_or(bytes.len());
        match skip_space(name_end) {
            None => return true,
            Some(i) if name_end > name_start && bytes.get(i) == Some(&b';') => return true,
            Some(_) => {}
        }
    }
    false
}

struct Parent<'a> {
    file: &'a Path,
    file_dir: &'a Path,
    /// The directory of the submodules of the current module
    module_dir: &'a Path,
    /// In an inline module (`mod a { ... }`): `#[path]` is relative to `module_dir`, not `file_dir`
    inline: bool,
}

fn collect_children(
    items: impl Iterator<Item = ast::Item>,
    parent: &Parent,
    children: &mut Vec<ModuleFile>,
    warnings: &mut Vec<String>,
) {
    for item in items {
        let ast::Item::Module(module) = item else {
            continue;
        };
        let Some(name) = module.name() else {
            continue;
        };
        let name = name.text();
        let name = name.strip_prefix("r#").unwrap_or(name);
        let path_attr = path_attribute(&module);
        if let Some(item_list) = module.item_list() {
            // `mod a { mod b; }`: `b` is in `a/` (or in the directory given by `#[path]`)
            let module_dir = parent.module_dir.join(path_attr.as_deref().unwrap_or(name));
            let inline = Parent {
                module_dir: &module_dir,
                inline: true,
                ..*parent
            };
            collect_children(item_list.items(), &inline, children, warnings);
            continue;
        }
        if let Some(path) = path_attr {
            let base = if parent.inline {
                parent.module_dir
            } else {
                parent.file_dir
            };
            children.push(ModuleFile {
                path: base.join(path),
                mod_rs: true,
            });
            continue;
        }
        let candidates = [
            (parent.module_dir.join(format!("{name}.rs")), false),
            (parent.module_dir.join(name).join("mod.rs"), true),
        ];
        match candidates.iter().find(|(path, _)| path.is_file()) {
            Some((path, mod_rs)) => children.push(ModuleFile {
                path: path.clone(),
                mod_rs: *mod_rs,
            }),
            None => warnings.push(format!(
                "{}: no file for module `{name}` (neither {} nor {})",
                parent.file.display(),
                candidates[0].0.display(),
                candidates[1].0.display()
            )),
        }
    }
}

/// The value of `#[path = "..."]` on a module; `#[cfg_attr(..., path = "...")]` is not followed
fn path_attribute(module: &ast::Module) -> Option<String> {
    module.attrs().find_map(|attr| {
        let ast::Meta::KeyValueMeta(meta) = attr.meta()? else {
            return None;
        };
        if attr.simple_name()? != "path" {
            return None;
        }
        let ast::Expr::Literal(literal) = meta.expr()? else {
            return None;
        };
        let ast::LiteralKind::String(string) = literal.kind() else {
            return None;
        };
        string.value().ok().map(|value| value.into_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TempDir;

    fn files(dir: &Path, root: &str) -> (Vec<String>, Vec<String>) {
        let tree = module_files(&dir.join(root), Edition::CURRENT);
        let dir = canonical_path(dir);
        let files = tree
            .files
            .iter()
            .map(|(file, _)| file.strip_prefix(&dir).unwrap().display().to_string())
            .collect();
        (files, tree.warnings)
    }

    #[test]
    fn module_file_declarations() {
        let declares = |source| super::may_declare_module_file(source, Edition::CURRENT);
        for source in [
            "mod a;",
            "pub(crate) mod a;",
            "mod r#type;",
            "mod /* c */ a\n;",
            "#[path = \"x.rs\"]\nmod a;",
            "mod a {\n    mod b;\n}",
        ] {
            assert!(declares(source), "{source}");
        }
        for source in [
            "mod a {}",
            "let model = module;",
            "// mod a;",
            "const S: &str = \"mod a;\";",
        ] {
            assert!(!declares(source), "{source}");
        }
    }

    #[test]
    fn mod_name_semicolon_scan() {
        for source in [
            "mod a;",
            "pub(crate) mod a ;",
            "mod r#type;",
            "mod\ta\n;",
            "x;mod a;",
            // A comment or a non-ASCII byte may hide a declaration
            "mod /* c */ a\n;",
            "mod a // c\n;",
            "mod\u{2028}a;",
            "\u{2028}mod a;",
            "mod é;",
            // Comments and strings are not told apart from code
            "// mod a;",
            "const S: &str = \"mod a;\";",
        ] {
            assert!(may_contain_mod_name_semicolon(source), "{source:?}");
        }
        for source in [
            "",
            "fn f() {}",
            "mod a {}",
            "mod a\n{\n}",
            "model;",
            "a_mod b;",
            "let model = module;",
            "mod;",
            "mod",
        ] {
            assert!(!may_contain_mod_name_semicolon(source), "{source:?}");
        }
    }

    #[test]
    fn the_text_of_each_file() {
        // `a.rs` declares no module and is not parsed
        let dir = TempDir::new(
            "modules-text",
            &[("src/lib.rs", "mod a;\n"), ("src/a.rs", "fn f() {}\n")],
        );
        let tree = module_files(&dir.join("src/lib.rs"), Edition::CURRENT);
        let texts: Vec<_> = tree.files.iter().map(|(_, text)| text.as_str()).collect();
        assert_eq!(texts, ["mod a;\n", "fn f() {}\n"]);
    }

    #[test]
    fn module_tree() {
        let dir = TempDir::new(
            "tree",
            &[
                ("src/lib.rs", "mod a;\npub mod b;\nmod r#type;\nfn f() {}\n"),
                ("src/a.rs", "mod c;\n"),
                ("src/a/c.rs", ""),
                ("src/b/mod.rs", "mod d;\n"),
                ("src/b/d.rs", ""),
                ("src/type.rs", ""),
            ],
        );
        let (files, warnings) = files(&dir, "src/lib.rs");
        assert_eq!(
            files,
            [
                "src/lib.rs",
                "src/a.rs",
                "src/a/c.rs",
                "src/b/mod.rs",
                "src/b/d.rs",
                "src/type.rs",
            ]
        );
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn inline_modules_and_path_attributes() {
        let dir = TempDir::new(
            "inline",
            &[
                (
                    "src/main.rs",
                    "mod a { mod b; mod c { mod d; } }\n#[path = \"other/x.rs\"]\nmod x;\n\
                     mod e { #[path = \"f.rs\"] mod f; }\n",
                ),
                ("src/a/b.rs", ""),
                ("src/a/c/d.rs", ""),
                // a file given by `#[path]` has its submodules in its own directory
                ("src/other/x.rs", "mod y;\n"),
                ("src/other/y.rs", ""),
                // `#[path]` in an inline module is relative to its directory
                ("src/e/f.rs", ""),
            ],
        );
        let (files, warnings) = files(&dir, "src/main.rs");
        assert_eq!(
            files,
            [
                "src/main.rs",
                "src/a/b.rs",
                "src/a/c/d.rs",
                "src/other/x.rs",
                "src/other/y.rs",
                "src/e/f.rs",
            ]
        );
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn missing_module_is_a_warning() {
        let dir = TempDir::new(
            "missing",
            &[
                ("src/lib.rs", "#[cfg(windows)]\nmod windows;\nmod found;\n"),
                ("src/found.rs", ""),
            ],
        );
        let (files, warnings) = files(&dir, "src/lib.rs");
        assert_eq!(files, ["src/lib.rs", "src/found.rs"]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("`windows`"), "{warnings:?}");
    }

    #[test]
    fn a_file_is_listed_once() {
        let dir = TempDir::new(
            "once",
            &[
                (
                    "src/lib.rs",
                    "mod a;\n#[path = \"a.rs\"]\nmod again;\n#[path = \"lib.rs\"]\nmod me;\n",
                ),
                ("src/a.rs", ""),
            ],
        );
        let (files, _) = files(&dir, "src/lib.rs");
        assert_eq!(files, ["src/lib.rs", "src/a.rs"]);
    }
}
