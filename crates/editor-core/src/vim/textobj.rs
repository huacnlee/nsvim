//! Text objects: `iw` `aw` `iW` `aW` `is` `as` `ip` `ap`, brackets (`i(` `a{` ...), quotes
//! (`i"` ...), tags (`it` `at`) and code structure from tree-sitter (`if` `af` `ic` `ac` `ia`
//! `aa` `i/` `a/`).

use crate::buffer::{Buffer, Pos};
use crate::editor::{Editor, KeyInput};
use crate::syntax::{self, Obj};

use super::motion::{char_cls, match_close, unmatched_open};
use super::{Parse, ch};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObjKind {
    Word(bool),
    Sentence,
    Para,
    Pair(char, char),
    Quote(char),
    Tag,
    Syntax(Obj),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextObj {
    pub inner: bool,
    pub kind: ObjKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObjRange {
    /// Chars `[a, b)`.
    Chars(usize, usize),
    /// Whole lines.
    Lines(usize, usize),
}

/// Reads `i` or `a` and the object's char.
pub(crate) fn parse(keys: &[KeyInput]) -> Parse<(TextObj, usize)> {
    let inner = match keys.first().and_then(ch) {
        Some('i') => true,
        Some('a') => false,
        _ => return Parse::Bad,
    };
    let Some(k) = keys.get(1) else { return Parse::More };
    let kind = match ch(k) {
        Some('w') => ObjKind::Word(false),
        Some('W') => ObjKind::Word(true),
        Some('p') => ObjKind::Para,
        Some('s') => ObjKind::Sentence,
        Some('t') => ObjKind::Tag,
        Some('f') => ObjKind::Syntax(Obj::Function),
        Some('c') => ObjKind::Syntax(Obj::Class),
        Some('a') => ObjKind::Syntax(Obj::Parameter),
        Some('/') => ObjKind::Syntax(Obj::Comment),
        Some('(' | ')' | 'b') => ObjKind::Pair('(', ')'),
        Some('[' | ']') => ObjKind::Pair('[', ']'),
        Some('{' | '}' | 'B') => ObjKind::Pair('{', '}'),
        Some('<' | '>') => ObjKind::Pair('<', '>'),
        Some(q @ ('"' | '\'' | '`')) => ObjKind::Quote(q),
        _ => return Parse::Bad,
    };
    Parse::Done((TextObj { inner, kind }, 2))
}

/// The text object around `cur`, `count` times.
pub(crate) fn select(buf: &Buffer, cur: Pos, obj: TextObj, count: usize) -> Option<ObjRange> {
    let count = count.max(1);
    match obj.kind {
        ObjKind::Word(big) => Some(word(buf, cur, obj.inner, big, count)),
        ObjKind::Para => Some(para(buf, cur.line, obj.inner, count)),
        ObjKind::Pair(open, close) => pair(buf, cur, obj.inner, open, close, count),
        ObjKind::Quote(q) => quote(buf, cur, obj.inner, q),
        ObjKind::Sentence => super::sentence::object(buf, cur, obj.inner, count),
        ObjKind::Tag => super::tags::object(buf, cur, obj.inner, count),
        ObjKind::Syntax(o) => {
            let (a, b, linewise) = syntax::object(buf, buf.path.as_deref(), cur, o, obj.inner, count)?;
            if linewise && b > a { Some(ObjRange::Lines(buf.pos(a).line, buf.pos(b - 1).line)) } else { Some(ObjRange::Chars(a, b)) }
        }
    }
}

/// The name a tree-sitter object goes by in messages.
pub(crate) fn kind_name(o: Obj) -> &'static str {
    match o {
        Obj::Function => "function",
        Obj::Class => "class",
        Obj::Parameter => "parameter",
        Obj::Comment => "comment",
    }
}

/// Says why a text object found nothing, when the reason isn't plain. Code objects need a
/// grammar for the file and a node around the cursor.
pub(crate) fn missing(ed: &mut Editor, obj: TextObj) {
    match obj.kind {
        ObjKind::Syntax(o) => {
            let path = ed.buf().path.clone();
            if syntax::supported(path.as_deref()) {
                ed.error(format!("E: no {} at the cursor", kind_name(o)));
            } else {
                ed.error("E: no syntax tree for this file");
            }
        }
        ObjKind::Tag => ed.error("E: no tag around the cursor"),
        _ => {}
    }
}

/// `iw` and `aw` on the cursor's line.
fn word(buf: &Buffer, cur: Pos, inner: bool, big: bool, count: usize) -> ObjRange {
    let start = buf.idx(Pos::new(cur.line, 0));
    let line: Vec<char> = buf.line(cur.line).chars().collect();
    let len = line.len();
    if len == 0 {
        return ObjRange::Chars(start, start);
    }
    let col = cur.col.min(len - 1);
    let c = |i: usize| char_cls(line[i], big);
    let run_end = |i: usize| {
        let k = c(i);
        let mut j = i;
        while j < len && c(j) == k {
            j += 1;
        }
        j
    };
    let run_start = |i: usize| {
        let k = c(i);
        let mut j = i;
        while j > 0 && c(j - 1) == k {
            j -= 1;
        }
        j
    };
    let mut s = run_start(col);
    let mut e = run_end(col);
    if !inner {
        if c(col) == 0 {
            if e < len {
                e = run_end(e);
            }
        } else if e < len && c(e) == 0 {
            e = run_end(e);
        } else if s > 0 && c(s - 1) == 0 {
            s = run_start(s - 1);
        }
    }
    for _ in 1..count {
        if e >= len {
            break;
        }
        e = run_end(e);
        if !inner && e < len && c(e) == 0 {
            e = run_end(e);
        }
    }
    ObjRange::Chars(start + s, start + e)
}

fn blank_line(buf: &Buffer, line: usize) -> bool {
    buf.line(line).trim().is_empty()
}

/// `ip` and `ap`: a run of non-blank lines (or blank ones), and for `ap` the blank lines after.
fn para(buf: &Buffer, line: usize, inner: bool, count: usize) -> ObjRange {
    let last = buf.last_line();
    let run = |from: usize| {
        let b = blank_line(buf, from);
        let mut l = from;
        while l < last && blank_line(buf, l + 1) == b {
            l += 1;
        }
        l
    };
    let b = blank_line(buf, line);
    let mut first = line;
    while first > 0 && blank_line(buf, first - 1) == b {
        first -= 1;
    }
    let mut end = run(line);
    if !inner {
        if !b {
            if end < last {
                end = run(end + 1);
            } else {
                while first > 0 && blank_line(buf, first - 1) {
                    first -= 1;
                }
            }
        } else if end < last {
            end = run(end + 1);
        }
    }
    for _ in 1..count {
        if end >= last {
            break;
        }
        end = run(end + 1);
        if !inner && end < last && blank_line(buf, end + 1) {
            end = run(end + 1);
        }
    }
    ObjRange::Lines(first, end)
}

/// `i(` `a(` and the other brackets, across lines.
fn pair(buf: &Buffer, cur: Pos, inner: bool, open: char, close: char, count: usize) -> Option<ObjRange> {
    let at = buf.idx(cur);
    let here = buf.char_idx(at);
    let mut start = if here == Some(open) { Some(at) } else { unmatched_open(buf, at, open, close) }?;
    for _ in 1..count {
        start = unmatched_open(buf, start, open, close)?;
    }
    let end = match_close(buf, start, open, close)?;
    if !inner {
        return Some(ObjRange::Chars(start, end + 1));
    }
    let mut a = start + 1;
    let mut b = end;
    if buf.char_idx(a) == Some('\n') && a < b {
        a += 1;
    }
    let inside = buf.slice(a, b);
    if let Some(nl) = inside.rfind('\n') {
        let tail = &inside[nl + 1..];
        if tail.chars().all(|c| c == ' ' || c == '\t') {
            b = a + inside[..nl + 1].chars().count();
        }
    }
    Some(ObjRange::Chars(a.min(b), b))
}

/// `i"` `a"` and the other quotes, on the cursor's line. Quotes pair up left to right.
fn quote(buf: &Buffer, cur: Pos, inner: bool, q: char) -> Option<ObjRange> {
    let start = buf.idx(Pos::new(cur.line, 0));
    let line: Vec<char> = buf.line(cur.line).chars().collect();
    let qs: Vec<usize> = (0..line.len()).filter(|&i| line[i] == q && (i == 0 || line[i - 1] != '\\')).collect();
    let col = cur.col;
    let mut found = None;
    for pair in qs.chunks(2) {
        if let [a, b] = *pair {
            if col >= a && col <= b {
                found = Some((a, b));
                break;
            }
            if a > col && found.is_none() {
                found = Some((a, b));
                break;
            }
        }
    }
    let (a, b) = found?;
    if inner {
        return Some(ObjRange::Chars(start + a + 1, start + b));
    }
    let mut e = b + 1;
    let mut s = a;
    let blank = |c: char| c == ' ' || c == '\t';
    if e < line.len() && blank(line[e]) {
        while e < line.len() && blank(line[e]) {
            e += 1;
        }
    } else {
        while s > 0 && blank(line[s - 1]) {
            s -= 1;
        }
    }
    Some(ObjRange::Chars(start + s, start + e))
}
