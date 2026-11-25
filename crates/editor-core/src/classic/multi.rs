//! Multiple cursors in the classic keymap. The main cursor is `view.cursor` with
//! `view.anchor`, and `Doc.extra` holds the others. Typing, deleting, moving, copying and
//! pasting act at every cursor. Edits run from the last cursor in the text to the first, so
//! each one's char indices stay valid, and one key makes one undo step.

use crate::buffer::{Buffer, Pos};
use crate::editor::{Action, Cursor, Editor, KeyCode, KeyInput};
use crate::vim;

use super::{MAC, arrow_target, letter, stop_typing, typing};

/// Whether more than one cursor is out.
pub(super) fn active(ed: &Editor) -> bool {
    ed.has_doc() && !ed.doc().extra.is_empty()
}

fn has_sel(c: &Cursor) -> bool {
    c.anchor.is_some_and(|a| a != c.pos)
}

/// A cursor's selection, start first, or its position twice.
fn span(c: &Cursor) -> (Pos, Pos) {
    match c.anchor {
        Some(a) if a < c.pos => (a, c.pos),
        Some(a) => (c.pos, a),
        None => (c.pos, c.pos),
    }
}

/// Every cursor, main first.
fn all(ed: &Editor) -> Vec<Cursor> {
    let doc = ed.doc();
    let mut out = vec![Cursor { pos: doc.view.cursor, anchor: doc.view.anchor }];
    out.extend(doc.extra.iter().copied());
    out
}

/// Two cursors that would act on the same text.
fn overlaps(a: &Cursor, b: &Cursor) -> bool {
    let (s1, e1) = span(a);
    let (s2, e2) = span(b);
    s1 == s2 || (s1 < e2 && s2 < e1)
}

/// Stores cursors, the main one first, merging any that land on the same text.
fn set_all(ed: &mut Editor, cs: Vec<Cursor>) {
    let mut kept: Vec<Cursor> = Vec::with_capacity(cs.len());
    for c in cs {
        if !kept.iter().any(|k| overlaps(k, &c)) {
            kept.push(c);
        }
    }
    let main = kept.remove(0);
    let doc = ed.doc_mut();
    doc.view.cursor = main.pos;
    doc.view.anchor = main.anchor;
    doc.extra = kept;
}

/// Drops every cursor but the main one.
pub(super) fn collapse(ed: &mut Editor) {
    if ed.has_doc() {
        ed.doc_mut().extra.clear();
    }
}

/// Scrolls so `line` shows, and keeps that view through the settle that follows.
fn reveal(ed: &mut Editor, line: usize) {
    let rows = ed.rows.max(1);
    let top = ed.doc().view.top;
    if line < top || line >= top + rows {
        ed.doc_mut().view.top = line.saturating_sub(rows / 2);
    }
    ed.vim.keep_view = true;
}

/// Keys that add cursors, and keys while several are out. True when the key was used here.
pub(super) fn key(ed: &mut Editor, k: KeyInput, out: &mut Vec<Action>) -> bool {
    if k.cmd && k.alt && matches!(k.code, KeyCode::Up | KeyCode::Down) {
        add_vertical(ed, k.code == KeyCode::Down);
        return true;
    }
    if k.cmd && k.shift && letter(&k) == Some('l') {
        select_all(ed);
        return true;
    }
    if !active(ed) {
        return false;
    }
    let word = if MAC { k.alt } else { k.cmd };
    let unit = ed.indent_unit();
    match k.code {
        KeyCode::Esc => {
            stop_typing(ed);
            collapse(ed);
            ed.doc_mut().view.anchor = None;
        }
        KeyCode::Char(c) if !k.cmd && !k.ctrl => {
            typing(ed);
            edit_each(ed, |b, s, e, _| put(b, s, e, &c.to_string()));
        }
        KeyCode::Enter => {
            typing(ed);
            edit_each(ed, |b, s, e, _| {
                let p = b.pos(s);
                let lead: String = b.line(p.line).chars().take(p.col).take_while(|c| *c == ' ' || *c == '\t').collect();
                put(b, s, e, &format!("\n{lead}"))
            });
        }
        KeyCode::Backspace => {
            typing(ed);
            edit_each(ed, |b, s, e, _| {
                if s < e {
                    b.remove(s, e);
                    s
                } else if s > 0 {
                    b.remove(s - 1, s);
                    s - 1
                } else {
                    s
                }
            });
        }
        KeyCode::Delete => {
            typing(ed);
            edit_each(ed, |b, s, e, _| {
                if s < e {
                    b.remove(s, e);
                } else if s < b.len_chars() {
                    b.remove(s, s + 1);
                }
                s
            });
        }
        KeyCode::Tab if !k.shift => {
            typing(ed);
            edit_each(ed, |b, s, e, _| put(b, s, e, &unit));
        }
        KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End if !k.alt || word => {
            stop_typing(ed);
            move_each(ed, k, word);
        }
        _ if k.cmd => match letter(&k) {
            Some('c') => copy(ed, false, out),
            Some('x') => copy(ed, true, out),
            Some('/') => comment(ed),
            Some('k') if k.shift => delete_lines(ed),
            Some('d' | 'v' | 's' | 'p' | 'b' | 'w' | ',') => return false,
            _ => {
                collapse(ed);
                return false;
            }
        },
        _ => {
            collapse(ed);
            return false;
        }
    }
    true
}

/// Replaces chars `s..e` with `text` and returns the index after it.
fn put(b: &mut Buffer, s: usize, e: usize, text: &str) -> usize {
    b.remove(s, e);
    b.insert_at(s, text)
}

/// Runs `edit` at every cursor, last in the text first. It gets the buffer, the cursor's
/// selection as char indices `[s, e)` and the cursor's index (0 is the main one), and returns
/// where the cursor goes. Cursors after an edit shift by the length it added or removed.
fn edit_each(ed: &mut Editor, mut edit: impl FnMut(&mut Buffer, usize, usize, usize) -> usize) {
    vim::begin(ed);
    let cs = all(ed);
    let buf = &mut ed.doc_mut().buf;
    let mut order: Vec<(usize, usize, usize)> = cs
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let (s, e) = span(c);
            (i, buf.idx(s), buf.idx(e))
        })
        .collect();
    order.sort_by_key(|o| std::cmp::Reverse(o.1));
    let mut done: Vec<(usize, usize)> = Vec::with_capacity(order.len());
    for (i, s, e) in order {
        let before = buf.len_chars() as isize;
        let at = edit(buf, s, e, i);
        let delta = buf.len_chars() as isize - before;
        for d in &mut done {
            d.1 = (d.1 as isize + delta).max(0) as usize;
        }
        done.push((i, at));
    }
    done.sort_by_key(|d| d.0);
    let new: Vec<Cursor> = done.iter().map(|&(_, at)| Cursor { pos: buf.pos(at), anchor: None }).collect();
    let main = new[0].pos;
    set_all(ed, new);
    ed.doc_mut().view.want_col = main.col;
}

/// Arrows, Home and End at every cursor. Shift extends each selection.
fn move_each(ed: &mut Editor, k: KeyInput, word: bool) {
    let want = ed.doc().view.want_col;
    let cs = all(ed);
    let mut keep_want = false;
    let new: Vec<Cursor> = cs
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let sel = if k.shift || !has_sel(c) { None } else { Some(span(c)) };
            let w = if i == 0 { want } else { c.pos.col };
            let (to, keep) = arrow_target(ed, c.pos, sel, w, k, word);
            if i == 0 {
                keep_want = keep;
            }
            let anchor = if k.shift { Some(c.anchor.unwrap_or(c.pos)) } else { None };
            Cursor { pos: to, anchor }
        })
        .collect();
    let main = new[0].pos;
    set_all(ed, new);
    if !keep_want {
        ed.doc_mut().view.want_col = if k.code == KeyCode::End { usize::MAX } else { main.col };
    }
}

/// Cmd+D: selects the word under the cursor. With a selection, adds a cursor on the next
/// match of it (wrapping, skipping matches already selected) and shows it.
pub(super) fn select_next(ed: &mut Editor) {
    stop_typing(ed);
    let main = all(ed)[0];
    if !has_sel(&main) {
        let (a, b) = vim::word_range_at(ed.buf(), main.pos);
        if a < b {
            let buf = ed.buf();
            let (pa, pb) = (buf.pos(a), buf.pos(b));
            let view = &mut ed.doc_mut().view;
            view.anchor = Some(pa);
            view.cursor = pb;
            view.want_col = pb.col;
        }
        return;
    }
    let cs = all(ed);
    let buf = ed.buf();
    let (s, e) = span(&main);
    let needle = buf.slice(buf.idx(s), buf.idx(e));
    let n = needle.chars().count();
    let taken: Vec<usize> = cs.iter().map(|c| buf.idx(span(c).0)).collect();
    let from = cs.last().map_or(0, |c| buf.idx(span(c).1));
    let hits = occurrences(buf, &needle);
    let next = hits.iter().chain(hits.iter()).skip_while(|&&a| a < from).chain(hits.iter()).find(|a| !taken.contains(a)).copied();
    let Some(a) = next else { return };
    let (pa, pb) = (buf.pos(a), buf.pos(a + n));
    ed.doc_mut().extra.push(Cursor { pos: pb, anchor: Some(pa) });
    reveal(ed, pb.line);
}

/// Cmd+Shift+L: a cursor on every match of the selection (or of the word under the cursor).
fn select_all(ed: &mut Editor) {
    stop_typing(ed);
    let main = all(ed)[0];
    let buf = ed.buf();
    let (s, e) = if has_sel(&main) {
        let (s, e) = span(&main);
        (buf.idx(s), buf.idx(e))
    } else {
        vim::word_range_at(buf, main.pos)
    };
    if s >= e {
        return;
    }
    let needle = buf.slice(s, e);
    let n = needle.chars().count();
    let hits = occurrences(buf, &needle);
    let mut cs: Vec<Cursor> = hits.iter().map(|&a| Cursor { pos: buf.pos(a + n), anchor: Some(buf.pos(a)) }).collect();
    if let Some(i) = hits.iter().position(|&a| a == s) {
        let m = cs.remove(i);
        cs.insert(0, m);
    }
    if !cs.is_empty() {
        set_all(ed, cs);
    }
}

/// The char starts of every match of `needle`, in order, not overlapping.
fn occurrences(buf: &Buffer, needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let text = buf.text();
    text.match_indices(needle).map(|(b, _)| buf.rope().byte_to_char(b)).collect()
}

/// Alt+Cmd+Up and Down: a new cursor on the line above the top cursor (below the bottom one),
/// at the main cursor's column.
fn add_vertical(ed: &mut Editor, down: bool) {
    stop_typing(ed);
    let cs = all(ed);
    let lines = cs.iter().map(|c| c.pos.line);
    let edge = if down { lines.max() } else { lines.min() }.unwrap_or(0);
    let target = if down {
        edge + 1
    } else if edge == 0 {
        return;
    } else {
        edge - 1
    };
    if target > ed.buf().last_line() {
        return;
    }
    let want = ed.doc().view.want_col;
    let col = if want == usize::MAX { ed.buf().line_len(target) } else { cs[0].pos.col.min(ed.buf().line_len(target)) };
    let mut all = cs;
    all.push(Cursor { pos: Pos::new(target, col), anchor: None });
    set_all(ed, all);
    reveal(ed, target);
}

/// Cmd+C and Cmd+X: the selections joined by line breaks, or every cursor's line.
fn copy(ed: &mut Editor, cut: bool, out: &mut Vec<Action>) {
    stop_typing(ed);
    let mut cs = all(ed);
    cs.sort_by_key(|c| span(c).0);
    let buf = ed.buf();
    if cs.iter().any(has_sel) {
        let text: Vec<String> = cs
            .iter()
            .filter(|c| has_sel(c))
            .map(|c| {
                let (s, e) = span(c);
                buf.slice(buf.idx(s), buf.idx(e))
            })
            .collect();
        vim::store(ed, None, text.join("\n"), false, !cut, out);
        if cut {
            edit_each(ed, |b, s, e, _| {
                b.remove(s, e);
                s
            });
            vim::commit_here(ed);
        }
        return;
    }
    let mut lines: Vec<usize> = cs.iter().map(|c| c.pos.line).collect();
    lines.dedup();
    let text: String = lines.iter().map(|&l| format!("{}\n", buf.line(l))).collect();
    vim::store(ed, None, text, true, !cut, out);
    if cut {
        remove_lines(ed, &lines);
    }
}

/// The lines every cursor's selection touches, sorted, each once.
fn cursor_lines(ed: &Editor) -> Vec<usize> {
    let mut lines: Vec<usize> = all(ed)
        .iter()
        .flat_map(|c| {
            let (s, e) = span(c);
            let last = if has_sel(c) && e.col == 0 && e.line > s.line { e.line - 1 } else { e.line };
            s.line..=last
        })
        .collect();
    lines.sort_unstable();
    lines.dedup();
    lines
}

/// Deletes `lines` (sorted) whole, and leaves one cursor where the first was.
fn remove_lines(ed: &mut Editor, lines: &[usize]) {
    let Some(&first) = lines.first() else { return };
    vim::begin(ed);
    let buf = &mut ed.doc_mut().buf;
    for &l in lines.iter().rev() {
        let rope = buf.rope();
        let start = buf.idx(Pos::new(l, 0));
        let (s, e) = if l + 1 < rope.len_lines() { (start, rope.line_to_char(l + 1)) } else { (start.saturating_sub(1), buf.len_chars()) };
        buf.remove(s, e);
    }
    let l = first.min(buf.last_line());
    collapse(ed);
    let view = &mut ed.doc_mut().view;
    view.anchor = None;
    view.cursor = Pos::new(l, 0);
    vim::commit_here(ed);
}

/// Cmd+Shift+K: deletes every cursor's lines.
fn delete_lines(ed: &mut Editor) {
    stop_typing(ed);
    let lines = cursor_lines(ed);
    remove_lines(ed, &lines);
}

/// Cmd+/: toggles comments on every cursor's lines, each run of lines on its own. Cursors keep
/// their place in the text.
fn comment(ed: &mut Editor) {
    stop_typing(ed);
    let lines = cursor_lines(ed);
    let before: Vec<usize> = lines.iter().map(|&l| ed.buf().line_len(l)).collect();
    vim::begin(ed);
    let mut i = 0;
    while i < lines.len() {
        let mut j = i;
        while j + 1 < lines.len() && lines[j + 1] == lines[j] + 1 {
            j += 1;
        }
        vim::toggle_comment(ed, lines[i], lines[j]);
        i = j + 1;
    }
    vim::commit_here(ed);
    let shift = |p: Pos, ed: &Editor| -> Pos {
        match lines.iter().position(|&l| l == p.line) {
            Some(k) if p.col > 0 => Pos::new(p.line, (p.col + ed.buf().line_len(p.line)).saturating_sub(before[k])),
            _ => p,
        }
    };
    let cs: Vec<Cursor> = all(ed).iter().map(|c| Cursor { pos: shift(c.pos, ed), anchor: c.anchor.map(|a| shift(a, ed)) }).collect();
    set_all(ed, cs);
}

/// Pastes at every cursor: one line of `text` per cursor when the counts match, else all of it
/// at each. Called by the paste the app answers, so it runs inside `put_reg`'s undo step.
pub(super) fn paste(ed: &mut Editor, text: &str) {
    let cs = all(ed);
    let t = text.strip_suffix('\n').unwrap_or(text);
    let parts: Vec<&str> = t.split('\n').collect();
    let mut by_pos: Vec<usize> = (0..cs.len()).collect();
    by_pos.sort_by_key(|&i| span(&cs[i]).0);
    let mut rank = vec![0; cs.len()];
    for (r, &i) in by_pos.iter().enumerate() {
        rank[i] = r;
    }
    let split = parts.len() == cs.len();
    edit_each(ed, |b, s, e, i| put(b, s, e, if split { parts[rank[i]] } else { text }));
}
