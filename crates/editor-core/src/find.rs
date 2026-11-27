//! The find bar's search. A query in plain text or as a regular expression, with match case and
//! whole word, becomes a search pattern. The view highlights it as it does a `/` search. This
//! module counts the matches, goes to the next one, and replaces one or all.
//!
//! The regular expression syntax is the common one (`(a|b)+`, `\d{2}`, `(?:x)`). It becomes the
//! Vim pattern that the search engine reads. In a replacement, `$1` is a group and `$&` the match.

use crate::buffer::Pos;
use crate::editor::{Editor, Keymap, Mode};
use crate::vim::ex::expand;
use crate::vim::regex::Regex;

/// The find bar's toggles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindOpts {
    /// Match case. Off: case does not matter.
    pub case: bool,
    /// Match whole words only.
    pub word: bool,
    /// The query is a regular expression.
    pub regex: bool,
}

/// The search pattern for `query`. An empty query gives an empty pattern.
pub fn pattern(query: &str, o: FindOpts) -> String {
    if query.is_empty() {
        return String::new();
    }
    let body = if o.regex { from_regex(query) } else { literal(query) };
    let body = if o.word { format!("\\<\\%({body}\\)\\>") } else { body };
    format!("{}{body}", if o.case { "\\C" } else { "\\c" })
}

/// The Vim replacement text for `with`. Plain text stays as typed. With `regex`, `$1`-`$9` are
/// groups, `$&` and `$0` the match, `$$` a dollar, and `\n` and `\t` a line break and a tab.
pub fn replacement(with: &str, regex: bool) -> String {
    let mut out = String::new();
    let mut it = with.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\\' if !regex => out.push_str("\\\\"),
            '&' => out.push_str("\\&"),
            '$' if regex => match it.peek().copied() {
                Some(d @ '1'..='9') => {
                    it.next();
                    out.push('\\');
                    out.push(d);
                }
                Some('&' | '0') => {
                    it.next();
                    out.push('&');
                }
                Some('$') => {
                    it.next();
                    out.push('$');
                }
                _ => out.push('$'),
            },
            '\\' => match it.next() {
                Some(e @ ('n' | 't' | '\\')) => {
                    out.push('\\');
                    out.push(e);
                }
                Some('&') => out.push_str("\\&"),
                Some(e) => out.push(e),
                None => out.push_str("\\\\"),
            },
            c => out.push(c),
        }
    }
    out
}

/// `query` as plain text: every char that the pattern syntax reads gets a backslash.
fn literal(query: &str) -> String {
    let mut out = String::new();
    for c in query.chars() {
        if matches!(c, '\\' | '.' | '*' | '[' | ']' | '^' | '$' | '~' | '/') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// A common regular expression as a Vim magic pattern.
fn from_regex(re: &str) -> String {
    let s: Vec<char> = re.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        i += 1;
        match c {
            '\\' => match s.get(i).copied() {
                Some(e @ ('(' | ')' | '|' | '+' | '?' | '{' | '}')) => {
                    i += 1;
                    out.push(e);
                }
                Some(e) => {
                    i += 1;
                    out.push('\\');
                    out.push(e);
                }
                None => out.push_str("\\\\"),
            },
            '[' => {
                // A set reads the same in both: copy it to its `]`.
                out.push('[');
                let mut first = true;
                while i < s.len() {
                    let d = s[i];
                    i += 1;
                    out.push(d);
                    if d == '\\' && i < s.len() {
                        out.push(s[i]);
                        i += 1;
                    } else if d == ']' && !first {
                        break;
                    }
                    first = d == '^' && first;
                }
            }
            '(' if s.get(i) == Some(&'?') && s.get(i + 1) == Some(&':') => {
                i += 2;
                out.push_str("\\%(");
            }
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '|' => out.push_str("\\|"),
            '{' => out.push_str("\\{"),
            '*' | '+' | '?' if s.get(i) == Some(&'?') => {
                // A lazy repeat.
                i += 1;
                out.push_str(match c {
                    '*' => "\\{-}",
                    '+' => "\\{-1,}",
                    _ => "\\{-0,1}",
                });
            }
            '+' => out.push_str("\\+"),
            '?' => out.push_str("\\="),
            '~' | '/' => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
}

/// Compiles `pat`, or gives the error for a pattern that does not compile.
pub fn compile(pat: &str) -> Result<Regex, String> {
    Regex::with_case(pat, None)
}

/// Every match in the active file, as char ranges in order. Empty matches are left out.
fn all(ed: &Editor, re: &Regex) -> Vec<(usize, usize)> {
    let buf = &ed.doc().buf;
    let mut out = Vec::new();
    let mut chars = Vec::new();
    for l in 0..=buf.last_line() {
        buf.line_chars(l, &mut chars);
        let start = buf.idx(Pos::new(l, 0));
        out.extend(re.find_all(&chars).into_iter().filter(|m| m.end > m.start).map(|m| (start + m.start, start + m.end)));
    }
    out
}

/// The start of the selection, else the cursor, as a char index.
fn here(ed: &Editor) -> usize {
    let v = &ed.doc().view;
    let p = v.anchor.map_or(v.cursor, |a| a.min(v.cursor));
    ed.doc().buf.idx(p)
}

/// The match count, and the number (from 1) of the match at the cursor, if one starts there.
pub fn count(ed: &Editor, pat: &str) -> (Option<usize>, usize) {
    if pat.is_empty() || !ed.has_doc() {
        return (None, 0);
    }
    let Ok(re) = compile(pat) else { return (None, 0) };
    let ms = all(ed, &re);
    let at = here(ed);
    (ms.iter().position(|m| m.0 == at).map(|i| i + 1), ms.len())
}

/// Shows match `(a, b)`: the classic keymap selects it, the Neovim keymap puts the cursor on its
/// start. Leaves Visual and Insert mode in the Neovim keymap.
fn show(ed: &mut Editor, (a, b): (usize, usize)) {
    let buf = &ed.doc().buf;
    let (pa, pb) = (buf.pos(a), buf.pos(b));
    let classic = ed.keymap == Keymap::Classic;
    if !classic {
        ed.mode = Mode::Normal;
    }
    let view = &mut ed.doc_mut().view;
    if classic {
        view.anchor = Some(pa);
        view.cursor = pb;
    } else {
        view.anchor = None;
        view.cursor = pa;
    }
    view.want_col = view.cursor.col;
}

/// Goes to the first match that starts at `from` or after it, wrapping around. For the search
/// while the user types: the match under the cursor stays. False when nothing matches.
pub fn first_from(ed: &mut Editor, pat: &str, from: Pos) -> bool {
    let Ok(re) = compile(pat) else { return false };
    if pat.is_empty() || !ed.has_doc() {
        return false;
    }
    let at = ed.doc().buf.idx(ed.doc().buf.clamp(from, true));
    let ms = all(ed, &re);
    let Some(&m) = ms.iter().find(|m| m.0 >= at).or(ms.first()) else { return false };
    show(ed, m);
    true
}

/// Goes to the next match after the cursor, or the one before it when `back`, wrapping around.
/// False when nothing matches.
pub fn step(ed: &mut Editor, pat: &str, back: bool) -> bool {
    let Ok(re) = compile(pat) else { return false };
    if pat.is_empty() || !ed.has_doc() {
        return false;
    }
    let at = here(ed);
    let ms = all(ed, &re);
    let m = if back { ms.iter().rev().find(|m| m.0 < at).or(ms.last()) } else { ms.iter().find(|m| m.0 > at).or(ms.first()) };
    let Some(&m) = m else { return false };
    show(ed, m);
    true
}

/// The match on its own line at char `a`, with the line's chars and the line's start index.
fn match_at(ed: &Editor, re: &Regex, a: usize) -> Option<(Vec<char>, usize, crate::vim::regex::Match)> {
    let buf = &ed.doc().buf;
    let l = buf.pos(a).line;
    let start = buf.idx(Pos::new(l, 0));
    let mut chars = Vec::new();
    buf.line_chars(l, &mut chars);
    let m = re.find_all(&chars).into_iter().find(|m| start + m.start == a && m.end > m.start)?;
    Some((chars, start, m))
}

/// Replaces the match at the cursor with `rep` (a Vim replacement, see [`replacement`]) and goes
/// to the next match. With no match at the cursor, it only goes to the next one. One undo step.
/// False when nothing matches.
pub fn replace_one(ed: &mut Editor, pat: &str, rep: &str) -> bool {
    let Ok(re) = compile(pat) else { return false };
    if pat.is_empty() || !ed.has_doc() {
        return false;
    }
    let at = here(ed);
    let Some((chars, start, m)) = match_at(ed, &re, at) else { return step(ed, pat, false) };
    let text = expand(rep, &chars, &m);
    let cur = ed.cursor();
    let buf = &mut ed.doc_mut().buf;
    if buf.in_txn() {
        buf.commit(cur);
    }
    buf.begin(cur);
    buf.remove(start + m.start, start + m.end);
    let end = buf.insert_at(start + m.start, &text);
    let after = buf.pos(end);
    buf.commit(after);
    let v = &mut ed.doc_mut().view;
    v.anchor = None;
    v.cursor = after;
    let ms = all(ed, &re);
    if let Some(&m) = ms.iter().find(|m| m.0 >= end).or(ms.first()) {
        show(ed, m);
    }
    true
}

/// Replaces every match in the file with `rep`, as one undo step. The number replaced.
pub fn replace_all(ed: &mut Editor, pat: &str, rep: &str) -> usize {
    let Ok(re) = compile(pat) else { return 0 };
    if pat.is_empty() || !ed.has_doc() {
        return 0;
    }
    let buf = &ed.doc().buf;
    let mut edits = Vec::new();
    let mut chars = Vec::new();
    for l in 0..=buf.last_line() {
        buf.line_chars(l, &mut chars);
        let start = buf.idx(Pos::new(l, 0));
        for m in re.find_all(&chars).into_iter().filter(|m| m.end > m.start) {
            edits.push((start + m.start, start + m.end, expand(rep, &chars, &m)));
        }
    }
    if edits.is_empty() {
        return 0;
    }
    let cur = ed.cursor();
    let buf = &mut ed.doc_mut().buf;
    if buf.in_txn() {
        buf.commit(cur);
    }
    buf.begin(cur);
    for (a, b, text) in edits.iter().rev() {
        buf.replace(*a, *b, text);
    }
    let first = buf.pos(edits[0].0);
    buf.commit(first);
    let v = &mut ed.doc_mut().view;
    v.anchor = None;
    v.cursor = first;
    v.want_col = first.col;
    edits.len()
}

#[cfg(test)]
#[path = "find_tests.rs"]
mod tests;
