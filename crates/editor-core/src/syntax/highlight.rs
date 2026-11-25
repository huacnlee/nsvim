//! Syntax colours from tree-sitter. Each grammar's own highlight query runs over the cached
//! tree of the buffer, only for the bytes of the lines on screen. The core names a kind for
//! each run. The app picks the colour.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::path::Path;

use streaming_iterator::StreamingIterator;
use tree_sitter::{Node, Parser, Query, QueryCursor};

use super::langs::{Lang, lang_for, lang_named};
use super::{parse_str, with_tree};
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
        // Markdown: headings, bold, italics, code, links.
        "text" => match name {
            "text.title" | "text.strong" => Kind::Keyword,
            "text.emphasis" => Kind::Type,
            "text.literal" => Kind::String,
            "text.reference" => Kind::Function,
            "text.uri" => Kind::Comment,
            _ => Kind::Text,
        },
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
        Lang::C => tree_sitter_c::HIGHLIGHT_QUERY.into(),
        Lang::Cpp => [tree_sitter_cpp::HIGHLIGHT_QUERY, tree_sitter_c::HIGHLIGHT_QUERY].concat(),
        Lang::Java => tree_sitter_java::HIGHLIGHTS_QUERY.into(),
        Lang::Ruby => tree_sitter_ruby::HIGHLIGHTS_QUERY.into(),
        Lang::Bash => tree_sitter_bash::HIGHLIGHT_QUERY.into(),
        Lang::Json => tree_sitter_json::HIGHLIGHTS_QUERY.into(),
        Lang::Lua => tree_sitter_lua::HIGHLIGHTS_QUERY.into(),
        Lang::Markdown => tree_sitter_md::HIGHLIGHT_QUERY_BLOCK.into(),
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
    /// Markdown's inline grammar and its query, for the text of paragraphs and headings.
    static INLINE: RefCell<Option<Option<(Parser, Compiled)>>> = const { RefCell::new(None) };
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
        if lang == Lang::Markdown {
            inline_captures(doc.root, rope, a..b, &mut out);
            code_captures(doc.root, rope, a..b, &mut out);
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

/// The nodes of `kind` under `root` that touch `range`. It does not look inside them.
fn nodes_of<'t>(root: Node<'t>, kind: &str, range: &Range<usize>) -> Vec<Node<'t>> {
    let mut nodes = Vec::new();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if n.end_byte() <= range.start || n.start_byte() >= range.end {
            continue;
        }
        if n.kind() == kind {
            nodes.push(n);
            continue;
        }
        let mut c = n.walk();
        stack.extend(n.children(&mut c));
    }
    nodes
}

/// The colours of the code in Markdown's fenced code blocks, in the language after the fence.
/// Blocks in other languages, or with none, keep the code block's colour.
fn code_captures(root: Node, rope: &ropey::Rope, range: Range<usize>, out: &mut Vec<(Range<usize>, usize, Kind)>) {
    for block in nodes_of(root, "fenced_code_block", &range) {
        let mut c = block.walk();
        let kids: Vec<Node> = block.children(&mut c).collect();
        let info = kids.iter().find(|n| n.kind() == "info_string").and_then(|n| n.child(0));
        let Some(lang) = info.and_then(|n| lang_named(&rope.byte_slice(n.byte_range()).to_string())) else { continue };
        let Some(content) = kids.iter().find(|n| n.kind() == "code_fence_content") else { continue };
        if lang == Lang::Markdown {
            continue;
        }
        let Some(q) = compiled(lang) else { continue };
        let base = content.start_byte();
        let text = rope.byte_slice(content.byte_range()).to_string();
        let Some(tree) = parse_str(lang, &text) else { continue };
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&q.query, tree.root_node(), text.as_bytes());
        while let Some(m) = matches.next() {
            for c in m.captures {
                let r = c.node.byte_range();
                out.push((base + r.start..base + r.end, m.pattern_index, q.kinds[c.index as usize]));
            }
        }
        // Code the query leaves alone shows as plain text.
        out.push((content.byte_range(), usize::MAX, Kind::Text));
    }
}

/// Markdown's inline captures (bold, italics, code spans, links) in the `inline` nodes of the
/// block tree that touch `range`. Each node's text is parsed alone with the inline grammar.
fn inline_captures(root: Node, rope: &ropey::Rope, range: Range<usize>, out: &mut Vec<(Range<usize>, usize, Kind)>) {
    let nodes = nodes_of(root, "inline", &range);
    INLINE.with(|inline| {
        let mut inline = inline.borrow_mut();
        let Some((parser, q)) = inline
            .get_or_insert_with(|| {
                let lang: tree_sitter::Language = tree_sitter_md::INLINE_LANGUAGE.into();
                let mut parser = Parser::new();
                parser.set_language(&lang).ok()?;
                let query = Query::new(&lang, tree_sitter_md::HIGHLIGHT_QUERY_INLINE).ok()?;
                let kinds = query.capture_names().iter().map(|n| kind(n)).collect();
                Some((parser, Compiled { query, kinds }))
            })
            .as_mut()
        else {
            return;
        };
        for n in nodes {
            let base = n.start_byte();
            let text = rope.byte_slice(n.byte_range()).to_string();
            let Some(tree) = parser.parse(&text, None) else { continue };
            let mut cursor = QueryCursor::new();
            let mut matches = cursor.matches(&q.query, tree.root_node(), text.as_bytes());
            while let Some(m) = matches.next() {
                for c in m.captures {
                    let k = q.kinds[c.index as usize];
                    if k != Kind::Text {
                        let r = c.node.byte_range();
                        out.push((base + r.start..base + r.end, m.pattern_index, k));
                    }
                }
            }
        }
    });
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
