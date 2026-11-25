//! Syntax colours from tree-sitter. Each grammar's own highlight query runs over the cached
//! tree of the buffer, only for the bytes of the lines on screen. The core names a kind for
//! each run. The app picks the colour.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::path::Path;

use streaming_iterator::StreamingIterator;
use tree_sitter::{Node, Query, QueryCursor};

use super::langs::{Lang, lang_for};
use super::with_tree;
use crate::buffer::Buffer;

/// What a run of text is drawn as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Keyword,
    Function,
    Type,
    String,
    Number,
    Comment,
    Macro,
    Property,
    Punct,
    Text,
}

/// The coloured runs of one line: char ranges and their kinds, in order, with no overlap.
pub type LineRuns = Vec<(Range<usize>, Kind)>;

/// The kind for a capture name of a highlight query: `keyword`, `function.method`, ...
pub fn kind(name: &str) -> Kind {
    let head = name.split('.').next().unwrap_or(name);
    match head {
        "keyword" if name == "keyword.operator" => Kind::Punct,
        "keyword" | "tag" | "conditional" | "repeat" | "include" | "exception" => Kind::Keyword,
        "function" | "method" => Kind::Function,
        "type" | "constructor" | "namespace" | "module" => Kind::Type,
        "string" | "escape" | "character" => Kind::String,
        "number" | "float" | "boolean" | "constant" | "label" => Kind::Number,
        "comment" => Kind::Comment,
        "attribute" | "preproc" | "define" => Kind::Macro,
        "property" | "field" => Kind::Property,
        "punctuation" | "operator" | "delimiter" => Kind::Punct,
        "variable" if name == "variable.builtin" => Kind::Keyword,
        _ => Kind::Text,
    }
}

/// The highlight query source of a language. TypeScript extends JavaScript's query, and C++
/// extends C's, as their grammars say.
fn source(lang: Lang) -> String {
    match lang {
        Lang::Rust => tree_sitter_rust::HIGHLIGHTS_QUERY.into(),
        Lang::Python => tree_sitter_python::HIGHLIGHTS_QUERY.into(),
        Lang::JavaScript => [tree_sitter_javascript::HIGHLIGHT_QUERY, tree_sitter_javascript::JSX_HIGHLIGHT_QUERY].concat(),
        Lang::TypeScript => [tree_sitter_typescript::HIGHLIGHTS_QUERY, tree_sitter_javascript::HIGHLIGHT_QUERY].concat(),
        Lang::Tsx => {
            [tree_sitter_typescript::HIGHLIGHTS_QUERY, tree_sitter_javascript::JSX_HIGHLIGHT_QUERY, tree_sitter_javascript::HIGHLIGHT_QUERY]
                .concat()
        }
        Lang::Go => tree_sitter_go::HIGHLIGHTS_QUERY.into(),
        #[cfg(not(target_arch = "wasm32"))]
        Lang::C => tree_sitter_c::HIGHLIGHT_QUERY.into(),
        #[cfg(not(target_arch = "wasm32"))]
        Lang::Cpp => [tree_sitter_cpp::HIGHLIGHT_QUERY, tree_sitter_c::HIGHLIGHT_QUERY].concat(),
        // The web has no C grammar: C and C++ use the C++ grammar's colours.
        #[cfg(target_arch = "wasm32")]
        Lang::C | Lang::Cpp => tree_sitter_cpp::HIGHLIGHT_QUERY.into(),
        Lang::Java => tree_sitter_java::HIGHLIGHTS_QUERY.into(),
        Lang::Ruby => tree_sitter_ruby::HIGHLIGHTS_QUERY.into(),
        Lang::Bash => tree_sitter_bash::HIGHLIGHT_QUERY.into(),
        Lang::Json => tree_sitter_json::HIGHLIGHTS_QUERY.into(),
        #[cfg(not(target_arch = "wasm32"))]
        Lang::Lua => tree_sitter_lua::HIGHLIGHTS_QUERY.into(),
        #[cfg(target_arch = "wasm32")]
        Lang::Lua => String::new(),
    }
}

/// A compiled query and the kind of each of its captures.
struct Compiled {
    query: Query,
    kinds: Vec<Kind>,
}

thread_local! {
    /// Compiled queries per language. `None` when a query does not compile.
    static QUERIES: RefCell<HashMap<Lang, Option<std::rc::Rc<Compiled>>>> = RefCell::new(HashMap::new());
}

fn compiled(lang: Lang) -> Option<std::rc::Rc<Compiled>> {
    QUERIES.with(|q| {
        q.borrow_mut()
            .entry(lang)
            .or_insert_with(|| {
                let query = Query::new(&lang.language(), &source(lang)).ok()?;
                let kinds = query.capture_names().iter().map(|n| kind(n)).collect();
                Some(std::rc::Rc::new(Compiled { query, kinds }))
            })
            .clone()
    })
}

/// Whether a highlight query covers this file.
pub fn highlighted(path: Option<&Path>) -> bool {
    lang_for(path).and_then(compiled).is_some()
}

/// The coloured runs of lines `first..first + count` of `buf`. Lines past the end, files with
/// no grammar and files too big to parse give empty runs.
pub fn runs(buf: &Buffer, path: Option<&Path>, first: usize, count: usize) -> Vec<LineRuns> {
    let empty = || vec![Vec::new(); count];
    let Some(lang) = lang_for(path) else { return empty() };
    let Some(q) = compiled(lang) else { return empty() };
    let rope = buf.rope();
    let first = first.min(rope.len_lines());
    let end = (first + count).min(rope.len_lines());
    let (a, b) = (rope.line_to_byte(first), rope.line_to_byte(end));
    // Captures as byte ranges, the pattern that made each, and its kind.
    let found = with_tree(buf, path, |doc| {
        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(a..b);
        let text = |node: Node| rope.byte_slice(node.byte_range()).chunks().map(str::as_bytes);
        let mut out: Vec<(Range<usize>, usize, Kind)> = Vec::new();
        let mut matches = cursor.matches(&q.query, doc.root, text);
        while let Some(m) = matches.next() {
            for c in m.captures {
                let k = q.kinds[c.index as usize];
                if k != Kind::Text {
                    out.push((c.node.byte_range(), m.pattern_index, k));
                }
            }
        }
        out
    });
    let Some(mut found) = found else { return empty() };
    // Paint wide nodes first, so a node inside another (an escape in a string) wins. For the
    // same node, the earlier pattern wins, as tree-sitter's own highlighter does.
    found.sort_by_key(|x| std::cmp::Reverse((x.0.len(), x.1)));
    let mut out = Vec::with_capacity(count);
    for line in first..first + count {
        if line >= end {
            out.push(Vec::new());
            continue;
        }
        let start = rope.line_to_byte(line);
        let stop = rope.line_to_byte(line + 1);
        let chars = rope.line(line).len_chars();
        let mut paint: Vec<Option<Kind>> = vec![None; chars];
        let base = rope.byte_to_char(start);
        for (r, _, k) in &found {
            let (s, e) = (r.start.max(start), r.end.min(stop));
            if s >= e {
                continue;
            }
            let (cs, ce) = (rope.byte_to_char(s) - base, (rope.byte_to_char(e) - base).min(chars));
            paint[cs..ce].fill(Some(*k));
        }
        out.push(runs_of_paint(&paint));
    }
    out
}

/// Joins painted chars into runs.
fn runs_of_paint(paint: &[Option<Kind>]) -> LineRuns {
    let mut runs: LineRuns = Vec::new();
    for (i, p) in paint.iter().enumerate() {
        let Some(k) = *p else { continue };
        match runs.last_mut() {
            Some((r, last)) if r.end == i && *last == k => r.end = i + 1,
            _ => runs.push((i..i + 1, k)),
        }
    }
    runs
}

#[cfg(test)]
#[path = "highlight_tests.rs"]
mod tests;
