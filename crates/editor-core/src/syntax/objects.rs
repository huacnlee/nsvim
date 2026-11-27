//! Finding functions, classes, parameters and comments in a tree, as text objects and as
//! motion targets. Ranges are char indices of the buffer.

use tree_sitter::Node;

use super::Obj;
use super::langs::Lang;
use crate::buffer::{Buffer, Pos};

/// A parsed buffer.
pub struct Doc<'a> {
    pub buf: &'a Buffer,
    pub lang: Lang,
    pub root: Node<'a>,
}

impl<'a> Doc<'a> {
    pub(super) fn ch(&self, byte: usize) -> usize {
        self.buf.rope().byte_to_char(byte.min(self.buf.len_bytes()))
    }

    pub(super) fn start(&self, n: Node) -> usize {
        self.ch(n.start_byte())
    }

    /// The node's end, before any line end it takes in.
    fn end(&self, n: Node) -> usize {
        let a = self.start(n);
        let mut b = self.ch(n.end_byte());
        while b > a && self.buf.char_idx(b - 1) == Some('\n') {
            b -= 1;
        }
        b
    }

    pub(super) fn text(&self, n: Node) -> String {
        let r = self.buf.rope();
        r.byte_slice(n.start_byte().min(r.len_bytes())..n.end_byte().min(r.len_bytes())).to_string()
    }

    fn line(&self, i: usize) -> usize {
        self.buf.pos(i).line
    }

    /// Whether only blanks come before char `i` on its line.
    fn starts_line(&self, i: usize) -> bool {
        let p = self.buf.pos(i);
        self.buf.line(p.line).chars().take(p.col).all(|c| c == ' ' || c == '\t')
    }

    /// Whether only blanks come after char `i` (an exclusive end) on its line.
    fn ends_line(&self, i: usize) -> bool {
        let p = self.buf.pos(i);
        self.buf.line(p.line).chars().skip(p.col).all(|c| c == ' ' || c == '\t')
    }

    /// `[a, b)` as whole lines: the first line's start to the last line's end.
    fn lines(&self, first: usize, last: usize) -> (usize, usize, bool) {
        let a = self.buf.idx(Pos::new(first, 0));
        let b = self.buf.idx(Pos::new(last, self.buf.line_len(last)));
        (a, b, true)
    }

    /// `[a, b)`, as whole lines when it starts and ends with its lines.
    fn span(&self, a: usize, b: usize) -> (usize, usize, bool) {
        if b > a && self.starts_line(a) && self.ends_line(b) { self.lines(self.line(a), self.line(b)) } else { (a, b, false) }
    }
}

/// Every node `keep` accepts, in document order.
pub(super) fn collect<'a>(root: Node<'a>, keep: impl Fn(Node<'a>) -> bool) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    let mut c = root.walk();
    loop {
        let n = c.node();
        if keep(n) {
            out.push(n);
        }
        if c.goto_first_child() {
            continue;
        }
        loop {
            if c.goto_next_sibling() {
                break;
            }
            if !c.goto_parent() {
                return out;
            }
        }
    }
}

pub(super) fn field<'a>(n: Node<'a>, name: &str) -> Option<Node<'a>> {
    n.child_by_field_name(name)
}

pub(super) fn is_function(doc: &Doc, n: Node) -> bool {
    doc.lang.functions().contains(&n.kind())
}

pub(super) fn is_class(doc: &Doc, n: Node) -> bool {
    if !doc.lang.classes().contains(&n.kind()) {
        return false;
    }
    match (doc.lang, n.kind()) {
        // `struct S s;` names a type; only a definition has a body.
        (Lang::C | Lang::Cpp, _) => field(n, "body").is_some(),
        (_, "type_alias_declaration") => field(n, "value").is_some_and(|v| v.kind() == "object_type"),
        _ => true,
    }
}

fn is_comment(doc: &Doc, n: Node) -> bool {
    doc.lang.comments().contains(&n.kind())
}

/// The node a function or class sits in that belongs with it: a decorated definition or an
/// export.
fn outer(n: Node) -> Node {
    match n.parent() {
        Some(p) if p.kind() == "decorated_definition" => p,
        Some(p) if p.kind() == "export_statement" && field(p, "declaration") == Some(n) => p,
        Some(p) if p.kind() == "template_declaration" => p,
        // `struct point { ... };` and `typedef struct { ... } T;`: the `;` and the comment above
        // belong to the declaration around the type.
        Some(p) if matches!(p.kind(), "declaration" | "type_definition") && field(p, "type") == Some(n) => p,
        _ => n,
    }
}

/// `af` and `ac`: the item with the doc comments and attributes right above it.
fn around(doc: &Doc, n: Node) -> (usize, usize, bool) {
    let top = outer(n);
    // A comment can sit one level up: Ruby puts it in the class, beside the body that starts
    // with the method.
    let mut anchor = top;
    while anchor.prev_named_sibling().is_none() {
        match anchor.parent() {
            Some(p) if p.start_byte() == anchor.start_byte() => anchor = p,
            _ => break,
        }
    }
    let mut first = top;
    let mut prev = anchor.prev_named_sibling();
    while let Some(s) = prev {
        let attached = doc.lang.attachments().contains(&s.kind()) || (is_comment(doc, s) && doc.lang.is_doc(&doc.text(s)));
        let touching = doc.line(doc.end(s)) + 1 >= doc.line(doc.start(first));
        if !attached || !touching || !doc.starts_line(doc.start(s)) {
            break;
        }
        first = s;
        prev = s.prev_named_sibling();
    }
    // `struct point { ... };` in C and C++: the `;` after the type ends the item.
    let c_type = matches!(doc.lang, Lang::C | Lang::Cpp) && top.kind().ends_with("_specifier");
    let end = match top.next_sibling() {
        Some(semi) if c_type && semi.kind() == ";" && doc.line(doc.start(semi)) == doc.line(doc.end(top)) => doc.end(semi),
        _ => doc.end(top),
    };
    doc.span(doc.start(first), end)
}

/// What sits inside a function's or class's braces (or its indented block).
fn body<'a>(doc: &Doc, n: Node<'a>) -> Option<Node<'a>> {
    match (doc.lang, n.kind()) {
        (Lang::Go, "type_declaration") => {
            let spec = collect(n, |c| c.kind() == "type_spec").into_iter().next()?;
            let ty = field(spec, "type")?;
            match ty.kind() {
                "struct_type" => collect(ty, |c| c.kind() == "field_declaration_list").into_iter().next(),
                "interface_type" => Some(ty),
                _ => None,
            }
        }
        (_, "type_alias_declaration") => field(n, "value"),
        _ => field(n, "body"),
    }
}

/// `if` and `ic`: the body without its braces, as whole lines when the braces sit on their
/// own lines.
fn inner(doc: &Doc, n: Node) -> Option<(usize, usize, bool)> {
    let b = body(doc, n)?;
    let count = b.child_count();
    let open = b.child(0).filter(|c| matches!(c.kind(), "{" | "(" | "["));
    let close = count.checked_sub(1).and_then(|i| b.child(i)).filter(|c| matches!(c.kind(), "}" | ")" | "]"));
    let mut cur = b.walk();
    let named: Vec<Node> = b.named_children(&mut cur).collect();
    match (open, close) {
        (Some(o), Some(c)) if o.id() != c.id() => {
            let (ol, cl) = (doc.line(doc.start(o)), doc.line(doc.start(c)));
            if cl >= ol + 2 && doc.ends_line(doc.ch(o.end_byte())) && doc.starts_line(doc.start(c)) {
                return Some(doc.lines(ol + 1, cl - 1));
            }
            match (named.first(), named.last()) {
                (Some(f), Some(l)) => Some((doc.start(*f), doc.end(*l), false)),
                _ => {
                    let p = doc.ch(o.end_byte());
                    Some((p, p, false))
                }
            }
        }
        _ => {
            let (a, e) = (doc.start(b), doc.end(b));
            if doc.line(a) > doc.line(doc.start(n)) && doc.starts_line(a) && doc.ends_line(e) {
                Some(doc.lines(doc.line(a), doc.line(e)))
            } else {
                Some((a, e, false))
            }
        }
    }
}

/// Whether `i` is in `r`, counting a whole-lines range by its lines.
fn contains(doc: &Doc, r: (usize, usize, bool), i: usize) -> bool {
    if r.2 {
        let l = doc.line(i);
        return l >= doc.line(r.0) && l <= doc.line(r.1);
    }
    i >= r.0 && i < r.1.max(r.0 + 1)
}

pub fn object(doc: &Doc, pos: Pos, obj: Obj, inner_only: bool, count: usize) -> Option<(usize, usize, bool)> {
    let i = doc.buf.idx(pos);
    match obj {
        Obj::Function | Obj::Class => {
            let items = collect(doc.root, |n| if obj == Obj::Function { is_function(doc, n) } else { is_class(doc, n) });
            let mut hits: Vec<(Node, (usize, usize, bool))> =
                items.iter().map(|n| (*n, around(doc, *n))).filter(|(_, r)| contains(doc, *r, i)).collect();
            hits.sort_by_key(|(_, r)| r.1 - r.0);
            hits.dedup_by_key(|(_, r)| (r.0, r.1));
            let n = match hits.get(count - 1) {
                Some((n, _)) => *n,
                None if hits.is_empty() && count == 1 => *items.iter().find(|n| doc.line(doc.start(**n)) == pos.line)?,
                None => return None,
            };
            if inner_only { inner(doc, n) } else { Some(around(doc, n)) }
        }
        Obj::Parameter => parameter(doc, i, inner_only, count),
        Obj::Comment => comment(doc, pos, i, inner_only),
    }
}

/// Every parameter or argument: the named, non-comment children of list nodes.
fn params<'a>(doc: &Doc<'a>) -> Vec<Node<'a>> {
    let lists = doc.lang.lists();
    collect(doc.root, |n| n.is_named() && !is_comment(doc, n) && n.parent().is_some_and(|p| lists.contains(&p.kind())))
}

fn parameter(doc: &Doc, i: usize, inner_only: bool, count: usize) -> Option<(usize, usize, bool)> {
    let all = params(doc);
    let mut hits: Vec<Node> = all.iter().copied().filter(|n| doc.start(*n) <= i && i < doc.end(*n).max(doc.start(*n) + 1)).collect();
    hits.sort_by_key(|n| doc.end(*n) - doc.start(*n));
    let n = match hits.get(count - 1) {
        Some(n) => *n,
        None if hits.is_empty() && count == 1 => {
            // On a comma or a space in a list: the parameter after the cursor, else the last.
            let lists = doc.lang.lists();
            let list = collect(doc.root, |n| lists.contains(&n.kind()) && doc.start(n) <= i && i < doc.end(n)).into_iter().last()?;
            let kids: Vec<Node> = all.iter().copied().filter(|n| n.parent() == Some(list)).collect();
            *kids.iter().find(|n| doc.start(**n) >= i).or(kids.last())?
        }
        None => return None,
    };
    let (a, b) = (doc.start(n), doc.end(n));
    if inner_only {
        return Some((a, b, false));
    }
    let siblings: Vec<Node> = all.iter().copied().filter(|s| s.parent() == n.parent()).collect();
    let at = siblings.iter().position(|s| *s == n)?;
    if let Some(next) = siblings.get(at + 1) {
        Some((a, doc.start(*next), false))
    } else if let Some(prev) = at.checked_sub(1).and_then(|p| siblings.get(p)) {
        Some((doc.end(*prev), b, false))
    } else {
        Some((a, b, false))
    }
}

fn comment(doc: &Doc, pos: Pos, i: usize, inner_only: bool) -> Option<(usize, usize, bool)> {
    let all = collect(doc.root, |n| is_comment(doc, n));
    let c = all
        .iter()
        .copied()
        .find(|n| doc.start(*n) <= i && i < doc.end(*n).max(doc.start(*n) + 1))
        .or_else(|| all.iter().copied().find(|n| doc.line(doc.start(*n)) == pos.line))?;
    if inner_only {
        return Some(comment_inner(doc, c));
    }
    let (first, last) = run(doc, c);
    Some(doc.span(doc.start(first), doc.end(last)))
}

/// The run of whole-line comments with the same marker around `c`, as its first and last.
fn run<'a>(doc: &Doc, c: Node<'a>) -> (Node<'a>, Node<'a>) {
    let text = doc.text(c);
    let Some(marker) = doc.lang.line_marker(&text) else { return (c, c) };
    if !doc.starts_line(doc.start(c)) {
        return (c, c);
    }
    let same = |s: Node, line: usize| {
        is_comment(doc, s)
            && doc.lang.line_marker(&doc.text(s)) == Some(marker)
            && doc.starts_line(doc.start(s))
            && doc.line(doc.start(s)) == line
    };
    let (mut first, mut last) = (c, c);
    while let Some(p) = first.prev_named_sibling() {
        let l = doc.line(doc.start(first));
        if l == 0 || !same(p, l - 1) {
            break;
        }
        first = p;
    }
    while let Some(n) = last.next_named_sibling() {
        if !same(n, doc.line(doc.start(last)) + 1) {
            break;
        }
        last = n;
    }
    (first, last)
}

/// A comment's text without its markers and the blanks around it.
fn comment_inner(doc: &Doc, c: Node) -> (usize, usize, bool) {
    let a = doc.start(c);
    let text: Vec<char> = doc.buf.slice(a, doc.end(c)).chars().collect();
    let s: String = text.iter().collect();
    let (mut lo, mut hi) = (0usize, text.len());
    if let Some(m) = doc.lang.line_marker(&s) {
        lo = s.chars().take_while(|c| c.is_whitespace()).count() + m.chars().count();
    } else {
        for open in ["/**", "/*!", "/*", "--[[", "=begin"] {
            if s.starts_with(open) {
                lo = open.chars().count();
                break;
            }
        }
        for close in ["*/", "]]", "=end"] {
            if s.ends_with(close) && hi >= lo + close.chars().count() {
                hi -= close.chars().count();
                break;
            }
        }
    }
    while lo < hi && text[lo].is_whitespace() {
        lo += 1;
    }
    while hi > lo && text[hi - 1].is_whitespace() {
        hi -= 1;
    }
    (a + lo, a + hi, false)
}

pub fn jump(doc: &Doc, pos: Pos, obj: Obj, forward: bool, end: bool, count: usize) -> Option<Pos> {
    let i = doc.buf.idx(pos);
    let nodes: Vec<(usize, usize)> = match obj {
        Obj::Function => collect(doc.root, |n| is_function(doc, n)).into_iter().map(|n| (doc.start(n), doc.end(n))).collect(),
        Obj::Class => collect(doc.root, |n| is_class(doc, n)).into_iter().map(|n| (doc.start(n), doc.end(n))).collect(),
        Obj::Parameter => params(doc).into_iter().map(|n| (doc.start(n), doc.end(n))).collect(),
        Obj::Comment => {
            let mut runs: Vec<(usize, usize)> = Vec::new();
            for c in collect(doc.root, |n| is_comment(doc, n)) {
                let (f, l) = run(doc, c);
                let r = (doc.start(f), doc.end(l));
                if !runs.contains(&r) {
                    runs.push(r);
                }
            }
            runs
        }
    };
    let mut points: Vec<usize> = nodes.iter().map(|&(a, b)| if end { b.saturating_sub(1).max(a) } else { a }).collect();
    points.sort_unstable();
    points.dedup();
    let pick = if forward {
        let after: Vec<usize> = points.into_iter().filter(|&p| p > i).collect();
        after.get(count - 1).or(after.last()).copied()
    } else {
        let before: Vec<usize> = points.into_iter().filter(|&p| p < i).rev().collect();
        before.get(count - 1).or(before.last()).copied()
    };
    pick.map(|p| doc.buf.pos(p))
}
