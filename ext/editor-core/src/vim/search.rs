//! `/`, `?`, `n`, `N`, `*` and `#`, and the matches the view highlights.

use crate::buffer::{Buffer, Pos};
use crate::editor::{Editor, Search};

use super::regex::Regex;

/// The char ranges of `pattern`'s matches on lines `first_line..=last_line`, for highlighting.
/// Empty matches are left out. A pattern that doesn't compile matches nothing. `case` is
/// `Editor::case`.
pub fn search_matches(buf: &Buffer, pattern: &str, case: Option<bool>, first_line: usize, last_line: usize) -> Vec<(usize, usize)> {
    if pattern.is_empty() {
        return Vec::new();
    }
    let Ok(re) = Regex::with_case(pattern, case) else { return Vec::new() };
    let mut out = Vec::new();
    let mut chars = Vec::new();
    for l in first_line..=last_line.min(buf.last_line()) {
        let start = buf.idx(Pos::new(l, 0));
        buf.line_chars(l, &mut chars);
        for m in re.find_all(&chars) {
            if m.end > m.start {
                out.push((start + m.start, start + m.end));
            }
        }
    }
    out
}

/// The char ranges of the matches on line `l`. `chars` is scratch space, reused across lines.
fn line_matches(buf: &Buffer, re: &Regex, l: usize, chars: &mut Vec<char>) -> Vec<(usize, usize)> {
    buf.line_chars(l, chars);
    re.find_all(chars).into_iter().map(|m| (m.start, m.end)).collect()
}

/// The next match after (or before) `from`, wrapping around the text. Returns its char range
/// and whether the search wrapped.
pub(crate) fn find(buf: &Buffer, re: &Regex, from: Pos, backward: bool) -> Option<((usize, usize), bool)> {
    let n = buf.last_line() + 1;
    let mut chars = Vec::new();
    for step in 0..=n {
        let (l, wrapped) =
            if backward { ((from.line + n - step % n) % n, step > from.line) } else { ((from.line + step) % n, from.line + step > n - 1) };
        let ms = line_matches(buf, re, l, &mut chars);
        let hit = if backward {
            if step == 0 {
                ms.iter().rev().find(|m| m.0 < from.col).copied()
            } else if step == n {
                ms.iter().rev().find(|m| m.0 >= from.col).copied()
            } else {
                ms.last().copied()
            }
        } else if step == 0 {
            ms.iter().find(|m| m.0 > from.col).copied()
        } else if step == n {
            ms.iter().find(|m| m.0 <= from.col).copied()
        } else {
            ms.first().copied()
        };
        if let Some((a, b)) = hit {
            let start = buf.idx(Pos::new(l, 0));
            return Some(((start + a, start + b), wrapped));
        }
    }
    None
}

/// Searches `count` times for the last pattern, in its direction or the other way when
/// `reverse`. Sets the message and returns the match's char range.
pub(crate) fn step_match(ed: &mut Editor, reverse: bool, count: usize) -> Option<(usize, usize)> {
    let pat = ed.search.pattern.clone();
    if pat.is_empty() {
        ed.error("E35: No previous regular expression");
        return None;
    }
    let re = match Regex::with_case(&pat, ed.case()) {
        Ok(re) => re,
        Err(e) => {
            ed.error(e);
            return None;
        }
    };
    let backward = ed.search.backward != reverse;
    let buf = &ed.doc().buf;
    let mut p = ed.doc().view.cursor;
    let mut wrapped = false;
    let mut found = None;
    for _ in 0..count.max(1) {
        match find(buf, &re, p, backward) {
            Some(((a, b), w)) => {
                p = buf.pos(a);
                wrapped |= w;
                found = Some((a, b));
            }
            None => {
                ed.error(format!("E486: Pattern not found: {pat}"));
                return None;
            }
        }
    }
    ed.vim.hlsearch = true;
    if wrapped {
        ed.message(if backward { "search hit TOP, continuing at BOTTOM" } else { "search hit BOTTOM, continuing at TOP" });
    } else {
        ed.message(format!("{}{pat}", if ed.search.backward { '?' } else { '/' }));
    }
    found
}

/// [`step_match`]'s start.
pub(crate) fn step(ed: &mut Editor, reverse: bool, count: usize) -> Option<Pos> {
    let (a, _) = step_match(ed, reverse, count)?;
    Some(ed.buf().pos(a))
}

/// `*` and `#`: searches for the keyword under or after the cursor. `whole` adds `\<` `\>`.
pub(crate) fn star(ed: &mut Editor, backward: bool, whole: bool, count: usize) -> Option<Pos> {
    let cur = ed.cursor();
    let line: Vec<char> = ed.buf().line(cur.line).chars().collect();
    let kw = |c: char| c.is_alphanumeric() || c == '_';
    let mut s = cur.col.min(line.len());
    while s < line.len() && !kw(line[s]) {
        s += 1;
    }
    if s >= line.len() {
        ed.error("E348: No string under cursor");
        return None;
    }
    while s > 0 && kw(line[s - 1]) {
        s -= 1;
    }
    let mut e = s;
    while e < line.len() && kw(line[e]) {
        e += 1;
    }
    let word: String = line[s..e].iter().collect();
    let pattern = if whole { format!("\\<{word}\\>") } else { word };
    ed.search = Search { pattern, backward };
    // Start from the word's start, so the first match is the next occurrence.
    ed.doc_mut().view.cursor = Pos::new(cur.line, s);
    let found = step(ed, false, count);
    if found.is_none() {
        ed.doc_mut().view.cursor = cur;
    }
    found
}

/// `gn` and `dgn`: the match under the cursor, else the next one (the one before for `gN`),
/// wrapping around the text. Its char range.
pub(crate) fn match_here(ed: &mut Editor, forward: bool) -> Option<(usize, usize)> {
    let pat = ed.search.pattern.clone();
    if pat.is_empty() {
        ed.error("E35: No previous regular expression");
        return None;
    }
    let re = match Regex::with_case(&pat, ed.case()) {
        Ok(re) => re,
        Err(e) => {
            ed.error(e);
            return None;
        }
    };
    let cur = ed.cursor();
    let buf = &ed.doc().buf;
    let start = buf.idx(Pos::new(cur.line, 0));
    let ms = line_matches(buf, &re, cur.line, &mut Vec::new());
    let here = if forward {
        ms.iter().find(|m| m.1 > cur.col || (m.0 == cur.col && m.1 == m.0))
    } else {
        ms.iter().rev().find(|m| m.0 <= cur.col)
    };
    if let Some(&(a, b)) = here {
        ed.vim.hlsearch = true;
        return Some((start + a, start + b));
    }
    match find(buf, &re, cur, !forward) {
        Some((m, _)) => {
            ed.vim.hlsearch = true;
            Some(m)
        }
        None => {
            ed.error(format!("E486: Pattern not found: {pat}"));
            None
        }
    }
}
