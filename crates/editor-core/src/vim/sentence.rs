//! Sentences, by Vim's rules: one ends at `.`, `!` or `?` followed by the end of the line, a
//! space or a tab, with any `)`, `]`, `"` or `'` between. An empty line is a boundary too. Used
//! by `(`, `)`, `is` and `as`.

use crate::buffer::{Buffer, Pos};

use super::motion::{para_back, para_fwd};
use super::textobj::ObjRange;

/// Sentence starts on lines `first..=last` as char indices, sorted, and the index where the
/// region ends.
fn starts(buf: &Buffer, first: usize, last: usize) -> (Vec<usize>, usize) {
    let mut out = Vec::new();
    let mut need = true;
    for l in first..=last {
        let base = buf.idx(Pos::new(l, 0));
        let chars: Vec<char> = buf.line(l).chars().collect();
        if chars.is_empty() {
            out.push(base);
            need = true;
            continue;
        }
        for (i, &c) in chars.iter().enumerate() {
            if need && c != ' ' && c != '\t' {
                out.push(base + i);
                need = false;
            }
            if matches!(c, '.' | '!' | '?') {
                let mut j = i + 1;
                while j < chars.len() && matches!(chars[j], ')' | ']' | '"' | '\'') {
                    j += 1;
                }
                if j == chars.len() || chars[j] == ' ' || chars[j] == '\t' {
                    need = true;
                }
            }
        }
    }
    let end = buf.idx(Pos::new(last, buf.line_len(last)));
    (out, end)
}

/// The lines `count` paragraphs around `line`, so every sentence a command can reach is in.
fn region(buf: &Buffer, line: usize, count: usize) -> (usize, usize) {
    let mut a = line;
    let mut b = line;
    for _ in 0..=count {
        a = para_back(buf, a.saturating_sub(1)).line;
        b = para_fwd(buf, (b + 1).min(buf.last_line())).line;
    }
    (a, b.min(buf.last_line()))
}

/// `)` (`forward`) and `(`: the start of the `count`th next or previous sentence.
pub(crate) fn motion(buf: &Buffer, cur: Pos, forward: bool, count: usize) -> Pos {
    let (a, b) = region(buf, cur.line, count);
    let (st, end) = starts(buf, a, b);
    let mut i = buf.idx(cur);
    for _ in 0..count.max(1) {
        i = if forward {
            st.iter().copied().find(|&s| s > i).unwrap_or(end)
        } else {
            st.iter().rev().copied().find(|&s| s < i).unwrap_or(0)
        };
    }
    buf.pos(i)
}

/// `is` and `as`: the sentence under the cursor, without or with its trailing white space,
/// `count` sentences long.
pub(crate) fn object(buf: &Buffer, cur: Pos, inner: bool, count: usize) -> Option<ObjRange> {
    if buf.line_len(cur.line) == 0 {
        return Some(ObjRange::Lines(cur.line, cur.line));
    }
    let (a, b) = region(buf, cur.line, count);
    let (mut st, end) = starts(buf, a, b);
    st.push(end);
    let at = buf.idx(cur);
    let k = st.iter().rposition(|&s| s <= at)?;
    // The end of sentence `i`'s text, before the white space that follows it.
    let text_end = |i: usize| -> usize {
        let (s, e) = (st[i], st.get(i + 1).copied().unwrap_or(end));
        let body = buf.slice(s, e);
        s + body.trim_end().chars().count()
    };
    let white = |c: Option<char>| matches!(c, Some(' ' | '\t' | '\n'));
    let k_end = text_end(k);
    if at >= k_end {
        // On the white space between two sentences.
        let next = st.get(k + 1).copied().unwrap_or(end);
        if inner {
            return Some(ObjRange::Chars(k_end, next));
        }
        let last = (k + count).min(st.len().saturating_sub(2));
        return Some(ObjRange::Chars(k_end, text_end(last).max(next)));
    }
    let last = (k + count.max(1) - 1).min(st.len().saturating_sub(2));
    let e = text_end(last);
    if inner {
        return Some(ObjRange::Chars(st[k], e));
    }
    let next = st.get(last + 1).copied().unwrap_or(end);
    if next > e && white(buf.char_idx(e)) && next != end {
        return Some(ObjRange::Chars(st[k], next));
    }
    // No white space after it (the paragraph ends): take the white space before it.
    let mut s = st[k];
    while s > 0 && matches!(buf.char_idx(s - 1), Some(' ' | '\t')) {
        s -= 1;
    }
    Some(ObjRange::Chars(s, e))
}
