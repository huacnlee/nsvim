//! Tag objects `it` and `at` for HTML and XML: the pair of tags around the cursor, nested
//! pairs matched by name, across lines.

use crate::buffer::Buffer;
use crate::buffer::Pos;

use super::textobj::ObjRange;

/// Tags are looked for this many chars around the cursor at most.
const WINDOW: usize = 200_000;

/// HTML elements that never have a closing tag.
const VOID: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];

#[derive(Clone, Debug)]
struct Tag {
    start: usize,
    end: usize,
    name: String,
    close: bool,
}

/// The tags in `text`, with char offsets from `base`. Comments, doctypes and processing
/// instructions are skipped, and so are self-closing and void tags.
fn scan(text: &[char], base: usize) -> Vec<Tag> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if text[i] != '<' {
            i += 1;
            continue;
        }
        let rest = &text[i + 1..];
        if rest.starts_with(&['!', '-', '-']) {
            let mut j = i + 4;
            while j + 2 < text.len() && !(text[j] == '-' && text[j + 1] == '-' && text[j + 2] == '>') {
                j += 1;
            }
            i = j + 3;
            continue;
        }
        if matches!(rest.first(), Some('!' | '?')) {
            while i < text.len() && text[i] != '>' {
                i += 1;
            }
            continue;
        }
        let close = rest.first() == Some(&'/');
        let mut j = i + 1 + usize::from(close);
        let name_start = j;
        while j < text.len() && (text[j].is_alphanumeric() || matches!(text[j], ':' | '_' | '-' | '.')) {
            j += 1;
        }
        if j == name_start {
            i += 1;
            continue;
        }
        let name: String = text[name_start..j].iter().collect();
        let mut quote: Option<char> = None;
        while j < text.len() {
            match (quote, text[j]) {
                (None, '"' | '\'') => quote = Some(text[j]),
                (Some(q), c) if c == q => quote = None,
                (None, '>') => break,
                _ => {}
            }
            j += 1;
        }
        if j >= text.len() {
            break;
        }
        let self_close = text[j - 1] == '/' || VOID.contains(&name.to_ascii_lowercase().as_str());
        if !self_close || close {
            out.push(Tag { start: base + i, end: base + j + 1, name, close });
        }
        i = j + 1;
    }
    out
}

/// Pairs each closing tag with the nearest open tag of its name; unmatched tags are dropped.
fn pairs(tags: &[Tag]) -> Vec<(Tag, Tag)> {
    let mut stack: Vec<Tag> = Vec::new();
    let mut out = Vec::new();
    for t in tags {
        if !t.close {
            stack.push(t.clone());
            continue;
        }
        if let Some(k) = stack.iter().rposition(|o| o.name == t.name) {
            let open = stack[k].clone();
            stack.truncate(k);
            out.push((open, t.clone()));
        }
    }
    out
}

/// `it` and `at` around `cur`; `count` 2 takes the pair around that one.
pub(crate) fn object(buf: &Buffer, cur: Pos, inner: bool, count: usize) -> Option<ObjRange> {
    let at = buf.idx(cur);
    let a = at.saturating_sub(WINDOW);
    let b = (at + WINDOW).min(buf.len_chars());
    let text: Vec<char> = buf.slice(a, b).chars().collect();
    let mut around: Vec<(Tag, Tag)> = pairs(&scan(&text, a)).into_iter().filter(|(o, c)| o.start <= at && at < c.end).collect();
    // Innermost first.
    around.sort_by_key(|p| std::cmp::Reverse(p.0.start));
    let (open, close) = around.get(count.max(1) - 1)?;
    Some(if inner { ObjRange::Chars(open.end, close.start) } else { ObjRange::Chars(open.start, close.end) })
}
