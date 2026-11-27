//! The functions, methods and classes of a file, from its tree: Go to Symbol in File when no
//! language server runs.

use tree_sitter::Node;

use super::objects::{Doc, collect, field, is_class, is_function};
use crate::buffer::Pos;

/// A named place in a file: what Go to Symbol in File lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    /// What it is, as the picker shows it: `Function`, `Method`, `Struct`.
    pub kind: &'static str,
    /// Where its name starts.
    pub pos: Pos,
    /// How many symbols it sits in: 0 at the top level, 1 for a method in a class.
    pub depth: usize,
}

/// Fields of a parent that name the function in it: `const f = () => {}`, `f: function () {}`.
const NAMING_FIELDS: &[&str] = &["name", "left", "key", "pattern"];

pub(super) fn list(doc: &Doc) -> Vec<Symbol> {
    let mut out = Vec::new();
    for n in collect(doc.root, |n| is_function(doc, n) || is_class(doc, n)) {
        let Some((name, at)) = name_of(doc, n) else { continue };
        let mut depth = 0;
        let mut in_class = None;
        let mut p = n.parent();
        while let Some(q) = p {
            let class = is_class(doc, q);
            if class || is_function(doc, q) {
                depth += 1;
                in_class.get_or_insert(class);
            }
            p = q.parent();
        }
        let kind = if is_class(doc, n) {
            class_kind(n.kind())
        } else if in_class == Some(true) {
            "Method"
        } else {
            "Function"
        };
        out.push(Symbol { name, kind, pos: doc.buf.pos(at), depth });
    }
    out
}

fn class_kind(node: &str) -> &'static str {
    for (part, kind) in [("struct", "Struct"), ("enum", "Enum"), ("union", "Union"), ("trait", "Trait"), ("interface", "Interface")] {
        if node.contains(part) {
            return kind;
        }
    }
    match node {
        "impl_item" => "Impl",
        "type_alias_declaration" | "type_declaration" => "Type",
        "module" | "mod_item" => "Module",
        _ => "Class",
    }
}

/// A short, one-line name.
fn plain(doc: &Doc, n: Node) -> Option<String> {
    let t = doc.text(n);
    let t = t.trim();
    (!t.is_empty() && !t.contains('\n') && t.chars().count() <= 80).then(|| t.to_string())
}

/// The symbol's name and the char where it starts. `None` for a closure with no name.
fn name_of(doc: &Doc, n: Node) -> Option<(String, usize)> {
    if n.kind() == "impl_item" {
        let ty = field(n, "type")?;
        let name = match field(n, "trait").and_then(|t| plain(doc, t)) {
            Some(t) => format!("impl {t} for {}", plain(doc, ty)?),
            None => format!("impl {}", plain(doc, ty)?),
        };
        return Some((name, doc.start(n)));
    }
    if n.kind() == "type_declaration"
        && let Some(spec) = collect(n, |c| c.kind() == "type_spec").into_iter().next()
    {
        let id = field(spec, "name")?;
        return Some((plain(doc, id)?, doc.start(id)));
    }
    if let Some(id) = field(n, "name") {
        return Some((plain(doc, id)?, doc.start(id)));
    }
    // C and C++: the name sits at the end of a chain of declarators.
    if let Some(mut d) = field(n, "declarator") {
        while let Some(inner) = field(d, "declarator") {
            d = inner;
        }
        return Some((plain(doc, d)?, doc.start(d)));
    }
    // A function with no name of its own takes the name it is given.
    let parent = n.parent()?;
    let id = NAMING_FIELDS.iter().filter_map(|f| field(parent, f)).find(|c| c.id() != n.id())?;
    Some((plain(doc, id)?, doc.start(id)))
}
