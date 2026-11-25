//! Normal-mode commands that stand alone: macros, Ctrl-a and Ctrl-x, scrolling, the jump
//! list, `gf`, `U` and `gn`.

use std::path::PathBuf;

use crate::buffer::Pos;
use crate::editor::{Action, Editor, KeyCode, KeyInput, Mode, VisualKind};

use super::normal::Scroll;
use super::{begin, ch, commit_here, doc_key, ops, push_jump, search, set_cur};

/// A private mark that follows the line `U` restores.
const ULINE: char = '\u{F8FF}';

pub(crate) fn stop_record(ed: &mut Editor) {
    let Some(reg) = ed.vim.recording.take() else { return };
    let mut keys = std::mem::take(&mut ed.vim.rec_keys);
    keys.pop();
    let text: String = keys.iter().filter_map(|k| ch(k).or((k.code == KeyCode::Enter).then_some('\n'))).collect();
    let lower = reg.to_ascii_lowercase();
    if reg.is_ascii_uppercase() {
        ed.vim.macros.entry(lower).or_default().extend(keys);
    } else {
        ed.vim.macros.insert(lower, keys);
    }
    ed.regs.named.insert(lower, crate::editor::Reg { text, linewise: false });
    ed.msg = None;
}

pub(crate) fn play(ed: &mut Editor, reg: char, n: usize, out: &mut Vec<Action>) {
    if reg == ':' {
        if let Some(line) = ed.vim.last_ex.clone() {
            for _ in 0..n {
                out.extend(super::run_ex(ed, &line));
            }
        }
        return;
    }
    let reg = if reg == '@' {
        match ed.vim.last_macro {
            Some(r) => r,
            None => return,
        }
    } else {
        reg.to_ascii_lowercase()
    };
    let keys = match ed.vim.macros.get(&reg) {
        Some(k) => k.clone(),
        None => match ed.reg(Some(reg)) {
            Some(r) => r.text.chars().map(|c| if c == '\n' { KeyInput::plain(KeyCode::Enter) } else { KeyInput::ch(c) }).collect(),
            None => return,
        },
    };
    if ed.vim.replaying > 20 {
        return;
    }
    ed.vim.last_macro = Some(reg);
    ed.vim.replaying += 1;
    for _ in 0..n {
        for k in &keys {
            super::dispatch(ed, *k, out);
        }
    }
    ed.vim.replaying -= 1;
}

/// Ctrl-a and Ctrl-x: adds `delta` to the number at or after the cursor.
pub(crate) fn incr(ed: &mut Editor, delta: i64) -> bool {
    let cur = ed.cursor();
    let line: Vec<char> = ed.buf().line(cur.line).chars().collect();
    let len = line.len();
    let mut i = cur.col;
    if i >= len {
        return false;
    }
    if line[i].is_ascii_digit() {
        while i > 0 && line[i - 1].is_ascii_digit() {
            i -= 1;
        }
    } else {
        while i < len && !line[i].is_ascii_digit() {
            i += 1;
        }
        if i >= len {
            return false;
        }
    }
    let mut s = i;
    let mut e = i;
    while e < len && line[e].is_ascii_digit() {
        e += 1;
    }
    if s > 0 && line[s - 1] == '-' {
        s -= 1;
    }
    let text: String = line[s..e].iter().collect();
    let Ok(num) = text.parse::<i64>() else { return false };
    let new = num.saturating_add(delta).to_string();
    begin(ed);
    let a = ed.buf().idx(Pos::new(cur.line, s));
    let b = ed.buf().idx(Pos::new(cur.line, e));
    ed.doc_mut().buf.replace(a, b, &new);
    set_cur(ed, Pos::new(cur.line, s + new.chars().count() - 1));
    commit_here(ed);
    true
}

pub(crate) fn scroll(ed: &mut Editor, s: Scroll, count: Option<usize>) {
    let rows = ed.rows.max(1);
    let so = ed.opts.scrolloff.min(rows.saturating_sub(1) / 2);
    let cur = ed.cursor();
    let last = ed.buf().last_line();
    let n = count.unwrap_or(1).max(1);
    let move_cursor = |ed: &mut Editor, line: usize| {
        let want = ed.doc().view.want_col;
        let len = ed.buf().line_len(line);
        let col = if want == usize::MAX { len.saturating_sub(1) } else { want.min(len.saturating_sub(1)) };
        ed.doc_mut().view.cursor = Pos::new(line, col);
    };
    match s {
        Scroll::HalfDown | Scroll::HalfUp => {
            let amt = count.unwrap_or((rows / 2).max(1));
            let down = s == Scroll::HalfDown;
            if (down && cur.line >= last) || (!down && cur.line == 0) {
                return;
            }
            let top = ed.doc().view.top;
            let new_top =
                if down { (top + amt).min(last.saturating_sub(rows.saturating_sub(1)).max(top)) } else { top.saturating_sub(amt) };
            ed.doc_mut().view.top = new_top;
            let line = if down { (cur.line + amt).min(last) } else { cur.line.saturating_sub(amt) };
            move_cursor(ed, line);
        }
        Scroll::PageDown | Scroll::PageUp => {
            let amt = (rows.saturating_sub(2).max(1) * n) as isize;
            ed.scroll_by(if s == Scroll::PageDown { amt } else { -amt });
        }
        Scroll::LineDown => ed.scroll_by(n as isize),
        Scroll::LineUp => ed.scroll_by(-(n as isize)),
        Scroll::Center | Scroll::Top | Scroll::Bottom => {
            let line = count.map(|c| c.saturating_sub(1).min(last)).unwrap_or(cur.line);
            if count.is_some() {
                let col = ed.buf().first_non_blank(line);
                set_cur(ed, Pos::new(line, col));
            }
            let top = match s {
                Scroll::Center => line.saturating_sub(rows / 2),
                Scroll::Top => line.saturating_sub(so),
                _ => (line + so + 1).saturating_sub(rows),
            };
            ed.doc_mut().view.top = top;
        }
    }
}

/// Ctrl-o and Ctrl-i: older and newer places on the jump list. A place in another file asks
/// the app to open it there.
pub(crate) fn jump(ed: &mut Editor, by: isize, out: &mut Vec<Action>) {
    let key = doc_key(ed);
    let cur = ed.cursor();
    if by < 0 && ed.vim.jump_idx >= ed.vim.jumps.len() {
        push_jump(ed, cur);
        ed.vim.jump_idx = ed.vim.jumps.len().saturating_sub(1);
    }
    let len = ed.vim.jumps.len() as isize;
    let to = (ed.vim.jump_idx as isize + by).clamp(0, (len - 1).max(0));
    if len == 0 || to as usize == ed.vim.jump_idx {
        return;
    }
    ed.vim.jump_idx = to as usize;
    let (k, p) = ed.vim.jumps[to as usize].clone();
    if k == key {
        set_cur(ed, p);
    } else if let Some(path) = k {
        out.push(Action::OpenAt(path, p));
    }
}

/// Chars Vim's `isfname` allows in a file name.
fn fname_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '/' | '.' | '-' | '_' | '+' | ',' | '#' | '$' | '%' | '~' | '=' | '@')
}

/// `gf`: opens the file named under the cursor, looked up beside the current file, then in
/// the project root.
pub(crate) fn goto_file(ed: &mut Editor, out: &mut Vec<Action>) {
    let cur = ed.cursor();
    let line: Vec<char> = ed.buf().line(cur.line).chars().collect();
    let mut s = cur.col.min(line.len());
    while s < line.len() && !fname_char(line[s]) {
        s += 1;
    }
    if s >= line.len() {
        ed.error("E446: No file name under cursor");
        return;
    }
    while s > 0 && fname_char(line[s - 1]) {
        s -= 1;
    }
    let mut e = s;
    while e < line.len() && fname_char(line[e]) {
        e += 1;
    }
    let name: String = line[s..e].iter().collect::<String>().trim_end_matches(['.', ',', ':']).to_string();
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let candidates: Vec<PathBuf> = if let Some(rest) = name.strip_prefix("~/") {
        vec![home.join(rest)]
    } else if name.starts_with('/') {
        vec![PathBuf::from(&name)]
    } else {
        let dir = ed.buf().path.as_ref().and_then(|p| p.parent()).map(|d| d.join(&name));
        dir.into_iter().chain(std::iter::once(ed.root.join(&name))).collect()
    };
    match candidates.into_iter().find(|p| p.exists()) {
        Some(p) => out.push(Action::Open(p)),
        None => ed.error(format!("E447: Can't find file \"{name}\" in path")),
    }
}

/// Before a change: remembers the cursor's line as it is, for `U`, unless the last change was
/// on this line too.
pub(crate) fn note_line(ed: &mut Editor) {
    if !ed.has_doc() {
        return;
    }
    let cur = ed.cursor();
    let key = doc_key(ed);
    let same = ed.vim.uline.as_ref().is_some_and(|(k, _)| *k == key) && ed.buf().mark(ULINE).is_some_and(|p| p.line == cur.line);
    if !same {
        let text = ed.buf().line(cur.line);
        ed.doc_mut().buf.set_mark(ULINE, Pos::new(cur.line, 0));
        ed.vim.uline = Some((key, text));
    }
}

/// `U`: puts the last changed line back as it was before its changes. It is a change itself,
/// so `u` undoes it and `U` again brings the changes back.
pub(crate) fn line_undo(ed: &mut Editor) {
    let key = doc_key(ed);
    let Some((k, orig)) = ed.vim.uline.clone() else { return };
    let Some(p) = ed.buf().mark(ULINE) else { return };
    if k != key {
        return;
    }
    let now = ed.buf().line(p.line);
    if now == orig {
        return;
    }
    begin(ed);
    ops::replace_lines(&mut ed.doc_mut().buf, p.line, p.line, &[orig]);
    ed.doc_mut().buf.set_mark(ULINE, Pos::new(p.line, 0));
    set_cur(ed, Pos::new(p.line, 0));
    commit_here(ed);
    ed.vim.uline = Some((k, now));
}

/// `gn` and `gN`: selects the search match under or after (before) the cursor.
pub(crate) fn select_match(ed: &mut Editor, forward: bool) {
    let Some((a, b)) = search::match_here(ed, forward) else { return };
    let buf = ed.buf();
    let (pa, pb) = (buf.pos(a), buf.pos(b.max(a + 1) - 1));
    let (anchor, cursor) = if forward { (pa, pb) } else { (pb, pa) };
    ed.mode = Mode::Visual(VisualKind::Char);
    let view = &mut ed.doc_mut().view;
    view.anchor = Some(anchor);
    view.cursor = cursor;
    view.want_col = cursor.col;
}
