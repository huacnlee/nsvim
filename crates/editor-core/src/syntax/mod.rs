//! Code structure from tree-sitter, for text objects (`af`, `if`, `ac`, `ic`, `aa`, `ia`,
//! `a/`, `i/`) and motions (`]f`, `[f`, `]c`, `[c`).
//!
//! Each call parses the buffer once per version and keeps a few trees per thread, so repeated
//! text objects on an unchanged buffer cost only a walk of the tree. Files over 4 MiB and
//! languages without a grammar give `None`.

pub mod highlight;
mod langs;
mod objects;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tree_sitter::{Parser, Tree};

use crate::buffer::{Buffer, Pos};
use langs::{Lang, lang_for};

/// A kind of code structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Obj {
    /// Functions, methods and closures.
    Function,
    /// Classes, structs, enums, traits, interfaces and impl blocks.
    Class,
    /// A parameter or argument in a list.
    Parameter,
    /// A comment, or a run of line comments.
    Comment,
}

/// Files bigger than this aren't parsed.
const MAX_BYTES: usize = 4 * 1024 * 1024;

/// Trees kept per thread.
const KEEP: usize = 4;

/// The `obj` around `pos` as a char range `[a, b)`, and whether it covers whole lines.
/// When it does, `a` is the start of the first line and `b` the end of the last line, before
/// its line end, so the lines are `buf.pos(a).line..=buf.pos(b).line`.
///
/// `inner` is the body without its signature and braces (a parameter without its comma, a
/// comment without its markers). `count` 2 picks the enclosing one, and so on. `None` when
/// the language has no grammar or `pos` is in no such node.
pub fn object(buf: &Buffer, path: Option<&Path>, pos: Pos, obj: Obj, inner: bool, count: usize) -> Option<(usize, usize, bool)> {
    with_tree(buf, path, |doc| objects::object(doc, pos, obj, inner, count.max(1)))?
}

/// Where the `count`th next (or previous) `obj` starts, or ends when `end` is true. When
/// fewer than `count` remain, the farthest one.
pub fn jump(buf: &Buffer, path: Option<&Path>, pos: Pos, obj: Obj, forward: bool, end: bool, count: usize) -> Option<Pos> {
    with_tree(buf, path, |doc| objects::jump(doc, pos, obj, forward, end, count.max(1)))?
}

/// The language name of a file for the status line, or `text` when no grammar covers it.
pub fn language_name(path: Option<&Path>) -> &'static str {
    lang_for(path).map_or("text", Lang::name)
}

/// Whether a grammar covers this file.
pub fn supported(path: Option<&Path>) -> bool {
    lang_for(path).is_some()
}

struct Entry {
    path: Option<PathBuf>,
    version: u64,
    len: usize,
    lang: Lang,
    tree: Tree,
}

thread_local! {
    static TREES: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
    static PARSERS: RefCell<HashMap<Lang, Parser>> = RefCell::new(HashMap::new());
}

/// Runs `f` on the buffer's tree, parsing it first when the cache has no tree for this
/// version.
fn with_tree<R>(buf: &Buffer, path: Option<&Path>, f: impl FnOnce(&objects::Doc) -> R) -> Option<R> {
    let lang = lang_for(path)?;
    if buf.len_bytes() > MAX_BYTES {
        return None;
    }
    TREES.with(|trees| {
        let mut trees = trees.borrow_mut();
        let key_path = path.map(Path::to_path_buf);
        let hit = trees.iter().position(|e| e.path == key_path && e.version == buf.version() && e.len == buf.len_chars() && e.lang == lang);
        let i = match hit {
            Some(i) => i,
            None => {
                let tree = parse(buf, lang)?;
                trees.retain(|e| e.path != key_path);
                if trees.len() >= KEEP {
                    trees.remove(0);
                }
                trees.push(Entry { path: key_path, version: buf.version(), len: buf.len_chars(), lang, tree });
                trees.len() - 1
            }
        };
        let e = &trees[i];
        Some(f(&objects::Doc { buf, lang, root: e.tree.root_node() }))
    })
}

/// Parses a short text, such as a code block in Markdown, with the parser of `lang`.
fn parse_str(lang: Lang, text: &str) -> Option<Tree> {
    PARSERS.with(|parsers| {
        let mut parsers = parsers.borrow_mut();
        let parser = match parsers.entry(lang) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                let mut p = Parser::new();
                p.set_language(&lang.language()).ok()?;
                v.insert(p)
            }
        };
        parser.parse(text, None)
    })
}

/// Parses the rope chunk by chunk, with no copy of the text.
fn parse(buf: &Buffer, lang: Lang) -> Option<Tree> {
    PARSERS.with(|parsers| {
        let mut parsers = parsers.borrow_mut();
        let parser = match parsers.entry(lang) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                let mut p = Parser::new();
                p.set_language(&lang.language()).ok()?;
                v.insert(p)
            }
        };
        let rope = buf.rope();
        let len = rope.len_bytes();
        let mut read = |byte: usize, _: tree_sitter::Point| -> &[u8] {
            if byte >= len {
                return &[];
            }
            let (chunk, start, _, _) = rope.chunk_at_byte(byte);
            &chunk.as_bytes()[byte - start..]
        };
        parser.parse_with_options(&mut read, None, None)
    })
}

#[cfg(test)]
mod tests;
