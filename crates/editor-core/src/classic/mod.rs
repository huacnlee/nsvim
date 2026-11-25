//! The classic keymap: no modes, arrows and Shift to select, Cmd (Ctrl off macOS) shortcuts.
//!
//! The editor stays in Insert mode. A selection runs from `view.anchor` to the cursor, with the
//! cursor's char left out. A run of typing undoes as one step until a key moves the cursor.

use crate::buffer::Pos;
use crate::editor::{Action, Editor, KeyCode, KeyInput, Mode};
use crate::vim;

const MAC: bool = cfg!(target_os = "macos");

/// Handles one key in the classic keymap and returns what the app must do.
pub fn handle_key(ed: &mut Editor, key: KeyInput) -> Vec<Action> {
    // The `:s///c` prompt takes its keys first, as in the Neovim keymap.
    if ed.vim.confirm.is_some() && ed.has_doc() {
        let mut out = Vec::new();
        vim::dispatch(ed, key, &mut out);
        if ed.vim.confirm.is_none() {
            ed.mode = Mode::Insert;
        }
        return out;
    }
    if ed.mode == Mode::Command {
        let out = vim::cmdline_key(ed, key);
        if ed.mode != Mode::Command {
            ed.mode = Mode::Insert;
        }
        return out;
    }
    ed.mode = Mode::Insert;
    let mut out = Vec::new();
    run(ed, key, &mut out);
    if ed.mode != Mode::Command {
        ed.mode = Mode::Insert;
    }
    vim::settle(ed);
    out
}

/// Pastes at every cursor while several are out. The shared paste path calls it.
pub(crate) fn paste_multi(ed: &mut Editor, text: &str) -> bool {
    if !multi::active(ed) {
        return false;
    }
    multi::paste(ed, text);
    true
}

fn letter(k: &KeyInput) -> Option<char> {
    match k.code {
        KeyCode::Char(c) => Some(c.to_ascii_lowercase()),
        _ => None,
    }
}

fn run(ed: &mut Editor, k: KeyInput, out: &mut Vec<Action>) {
    // Shortcuts that work with no file open.
    if k.cmd {
        let act = match letter(&k) {
            Some('s') => Some(Action::Save(None)),
            Some('p') if k.shift => Some(Action::Palette),
            Some('p') => Some(Action::FindFile),
            Some('o') => Some(Action::OpenProject),
            Some('f') if k.shift => Some(Action::GrepProject),
            Some('e') if k.shift => Some(Action::FocusTree),
            Some('b') => Some(Action::ToggleTree),
            Some('w') => Some(Action::CloseBuffer),
            Some(',') => Some(Action::Settings),
            Some('q') => Some(Action::QuitAll { force: false }),
            _ => None,
        };
        if let Some(a) = act {
            stop_typing(ed);
            out.push(a);
            return;
        }
    }
    match k.code {
        KeyCode::F(12) => {
            out.push(Action::GotoDefinition);
            return;
        }
        KeyCode::Tab if k.ctrl => {
            out.push(if k.shift { Action::PrevBuffer } else { Action::NextBuffer });
            return;
        }
        _ => {}
    }
    if !ed.has_doc() {
        return;
    }
    if multi::key(ed, k, out) {
        return;
    }
    if k.cmd {
        match letter(&k) {
            Some('f') => {
                stop_typing(ed);
                if k.alt {
                    vim::open_cmdline(ed, ':', "%s/");
                } else {
                    vim::open_cmdline(ed, '/', "");
                }
                return;
            }
            Some('z') if k.shift => return undo(ed, false),
            Some('z') => return undo(ed, true),
            Some('y') => return undo(ed, false),
            Some('c') => return copy(ed, false, out),
            Some('x') => return copy(ed, true, out),
            Some('v') => {
                stop_typing(ed);
                out.push(Action::Paste { before: true, count: 1 });
                return;
            }
            Some('a') => {
                stop_typing(ed);
                let last = ed.buf().last_line();
                let end = Pos::new(last, ed.buf().line_len(last));
                let view = &mut ed.doc_mut().view;
                view.anchor = Some(Pos::new(0, 0));
                view.cursor = end;
                return;
            }
            Some('d') => return multi::select_next(ed),
            Some('k') if k.shift => return delete_lines(ed),
            Some('/') => {
                stop_typing(ed);
                let (a, b) = sel_lines(ed);
                vim::toggle_comment(ed, a, b);
                return;
            }
            Some(']') | Some('[') => {
                stop_typing(ed);
                shift(ed, letter(&k) == Some(']'));
                return;
            }
            _ => {}
        }
    }
    let word = if MAC { k.alt } else { k.cmd };
    match k.code {
        KeyCode::Esc => {
            stop_typing(ed);
            ed.doc_mut().view.anchor = None;
        }
        KeyCode::Char(c) if !k.cmd && !k.ctrl => {
            typing(ed);
            delete_sel(ed);
            vim::type_char(ed, c);
        }
        KeyCode::Enter => {
            typing(ed);
            delete_sel(ed);
            vim::newline(ed);
        }
        KeyCode::Backspace => {
            typing(ed);
            if !delete_sel(ed) {
                if word {
                    let to = word_left(ed, ed.cursor());
                    remove_to(ed, to);
                } else {
                    vim::backspace(ed);
                }
            }
        }
        KeyCode::Delete => {
            typing(ed);
            if !delete_sel(ed) {
                if word {
                    let to = word_right(ed, ed.cursor());
                    remove_to(ed, to);
                } else {
                    vim::delete_fwd(ed);
                }
            }
        }
        KeyCode::Tab => {
            let (a, b) = sel_lines(ed);
            if k.shift || (a != b && sel(ed).is_some()) {
                stop_typing(ed);
                shift(ed, !k.shift);
            } else {
                typing(ed);
                delete_sel(ed);
                let unit = ed.indent_unit();
                vim::insert_text(ed, &unit);
            }
        }
        KeyCode::Up | KeyCode::Down if k.alt && !k.cmd => {
            stop_typing(ed);
            let (a, b) = sel_lines(ed);
            vim::move_lines(ed, a, b, k.code == KeyCode::Down);
        }
        KeyCode::Left
        | KeyCode::Right
        | KeyCode::Up
        | KeyCode::Down
        | KeyCode::Home
        | KeyCode::End
        | KeyCode::PageUp
        | KeyCode::PageDown => {
            stop_typing(ed);
            arrow(ed, k, word);
        }
        _ => {}
    }
}

/// The selection, start first, when it is not empty.
fn sel(ed: &Editor) -> Option<(Pos, Pos)> {
    let v = ed.doc().view;
    v.anchor.filter(|a| *a != v.cursor).map(|a| if a < v.cursor { (a, v.cursor) } else { (v.cursor, a) })
}

/// The lines the selection touches, or the cursor's line. A selection that ends at a line's
/// start leaves that line out.
fn sel_lines(ed: &Editor) -> (usize, usize) {
    match sel(ed) {
        Some((s, e)) => (s.line, if e.col == 0 && e.line > s.line { e.line - 1 } else { e.line }),
        None => {
            let l = ed.cursor().line;
            (l, l)
        }
    }
}

/// Opens a typing run's undo step, unless one is open.
fn typing(ed: &mut Editor) {
    if !ed.vim.classic_typing || !ed.buf().in_txn() {
        vim::commit_here(ed);
        vim::begin(ed);
        ed.vim.classic_typing = true;
    }
}

fn stop_typing(ed: &mut Editor) {
    if ed.vim.classic_typing {
        ed.vim.classic_typing = false;
        vim::commit_here(ed);
    }
}

/// Deletes the selection. Returns false when nothing was selected.
fn delete_sel(ed: &mut Editor) -> bool {
    let Some((s, e)) = sel(ed) else {
        ed.doc_mut().view.anchor = None;
        return false;
    };
    vim::begin(ed);
    let buf = &mut ed.doc_mut().buf;
    let (a, b) = (buf.idx(s), buf.idx(e));
    buf.remove(a, b);
    let view = &mut ed.doc_mut().view;
    view.anchor = None;
    view.cursor = s;
    view.want_col = s.col;
    true
}

fn remove_to(ed: &mut Editor, to: Pos) {
    let cur = ed.cursor();
    let buf = &mut ed.doc_mut().buf;
    let (a, b) = (buf.idx(cur), buf.idx(to));
    buf.remove(a.min(b), a.max(b));
    let p = if to < cur { to } else { cur };
    let view = &mut ed.doc_mut().view;
    view.cursor = p;
    view.want_col = p.col;
}

fn undo(ed: &mut Editor, back: bool) {
    stop_typing(ed);
    vim::commit_here(ed);
    let buf = &mut ed.doc_mut().buf;
    let at = if back { buf.undo() } else { buf.redo() };
    match at {
        Some(p) => {
            let p = ed.buf().clamp(p, true);
            let view = &mut ed.doc_mut().view;
            view.cursor = p;
            view.anchor = None;
            view.want_col = p.col;
        }
        None => ed.message(if back { "Already at oldest change" } else { "Already at newest change" }),
    }
}

/// Cmd+C and Cmd+X. With no selection they take the whole line.
fn copy(ed: &mut Editor, cut: bool, out: &mut Vec<Action>) {
    stop_typing(ed);
    let (text, linewise) = match sel(ed) {
        Some((s, e)) => (ed.buf().slice(ed.buf().idx(s), ed.buf().idx(e)), false),
        None => (format!("{}\n", ed.buf().line(ed.cursor().line)), true),
    };
    vim::store(ed, None, text, linewise, !cut, out);
    if !cut {
        return;
    }
    vim::begin(ed);
    if linewise {
        let l = ed.cursor().line;
        delete_line_range(ed, l, l);
    } else {
        delete_sel(ed);
    }
    vim::commit_here(ed);
}

fn delete_line_range(ed: &mut Editor, a: usize, b: usize) {
    let col = ed.cursor().col;
    let buf = &mut ed.doc_mut().buf;
    let rope = buf.rope();
    let start = buf.idx(Pos::new(a, 0));
    let (s, e) = if b + 1 < rope.len_lines() { (start, rope.line_to_char(b + 1)) } else { (start.saturating_sub(1), buf.len_chars()) };
    buf.remove(s, e);
    let l = a.min(buf.last_line());
    let p = Pos::new(l, col.min(buf.line_len(l)));
    let view = &mut ed.doc_mut().view;
    view.anchor = None;
    view.cursor = p;
}

fn delete_lines(ed: &mut Editor) {
    stop_typing(ed);
    let (a, b) = sel_lines(ed);
    vim::begin(ed);
    delete_line_range(ed, a, b);
    vim::commit_here(ed);
}

/// Cmd+] and Cmd+[, Tab and Shift+Tab over a selection: shifts the lines and keeps the
/// selection on the same text.
fn shift(ed: &mut Editor, right: bool) {
    let (a, b) = sel_lines(ed);
    let before = ed.buf().line_len(ed.cursor().line);
    let anchor_line = ed.doc().view.anchor.map(|p| (p, ed.buf().line_len(p.line)));
    vim::begin(ed);
    vim::shift_lines(ed, a, b, right, 1);
    vim::commit_here(ed);
    let fix = |p: Pos, old: usize, new: usize| -> Pos {
        if p.line < a || p.line > b {
            return p;
        }
        Pos::new(p.line, (p.col + new).saturating_sub(old))
    };
    let cur = ed.cursor();
    let after = ed.buf().line_len(cur.line);
    let c = fix(cur, before, after);
    let anchor = anchor_line.map(|(p, old)| fix(p, old, ed.buf().line_len(p.line)));
    let view = &mut ed.doc_mut().view;
    view.cursor = c;
    view.anchor = anchor;
}

fn word_left(ed: &Editor, p: Pos) -> Pos {
    if p.col == 0 {
        return if p.line == 0 { p } else { Pos::new(p.line - 1, ed.buf().line_len(p.line - 1)) };
    }
    let line: Vec<char> = ed.buf().line(p.line).chars().collect();
    let mut i = p.col.min(line.len());
    while i > 0 && line[i - 1].is_whitespace() {
        i -= 1;
    }
    if i > 0 {
        let k = vim::char_cls(line[i - 1], false);
        while i > 0 && vim::char_cls(line[i - 1], false) == k && !line[i - 1].is_whitespace() {
            i -= 1;
        }
    }
    Pos::new(p.line, i)
}

fn word_right(ed: &Editor, p: Pos) -> Pos {
    let len = ed.buf().line_len(p.line);
    if p.col >= len {
        return if p.line >= ed.buf().last_line() { p } else { Pos::new(p.line + 1, 0) };
    }
    let line: Vec<char> = ed.buf().line(p.line).chars().collect();
    let mut i = p.col;
    while i < len && line[i].is_whitespace() {
        i += 1;
    }
    if i < len {
        let k = vim::char_cls(line[i], false);
        while i < len && vim::char_cls(line[i], false) == k && !line[i].is_whitespace() {
            i += 1;
        }
    }
    Pos::new(p.line, i)
}

fn arrow(ed: &mut Editor, k: KeyInput, word: bool) {
    let extend = k.shift;
    let cur = ed.cursor();
    let want = ed.doc().view.want_col;
    let rows = ed.rows.max(2) - 1;
    let collapsed = if extend { None } else { sel(ed) };
    let (to, keep_want) = arrow_target(ed, cur, collapsed, want, k, word);
    let view = &mut ed.doc_mut().view;
    if extend {
        if view.anchor.is_none() {
            view.anchor = Some(cur);
        }
    } else {
        view.anchor = None;
    }
    view.cursor = to;
    if !keep_want {
        view.want_col = if k.code == KeyCode::End { usize::MAX } else { to.col };
    }
    if matches!(k.code, KeyCode::PageUp | KeyCode::PageDown) {
        let rows = rows as isize;
        ed.scroll_by(if k.code == KeyCode::PageUp { -rows } else { rows });
        ed.doc_mut().view.cursor = to;
    }
}

/// Where an arrow, Home, End or page key takes a cursor at `cur`. `sel` is its selection
/// (start first) when the key collapses it. Also says whether the wanted column stays.
fn arrow_target(ed: &Editor, cur: Pos, sel: Option<(Pos, Pos)>, want: usize, k: KeyInput, word: bool) -> (Pos, bool) {
    let buf = ed.buf();
    let last = buf.last_line();
    let len = buf.line_len(cur.line);
    let rows = ed.rows.max(2) - 1;
    let vertical = |line: usize| -> Pos {
        let l = ed.buf().line_len(line);
        Pos::new(line, if want == usize::MAX { l } else { want.min(l) })
    };
    let home = |p: Pos| -> Pos {
        let fnb = ed.buf().first_non_blank(p.line);
        Pos::new(p.line, if p.col == fnb { 0 } else { fnb })
    };
    let doc_end = Pos::new(last, buf.line_len(last));
    let mut keep_want = false;
    let to = match k.code {
        KeyCode::Left if sel.is_some() => sel.map(|(s, _)| s).unwrap_or(cur),
        KeyCode::Right if sel.is_some() => sel.map(|(_, e)| e).unwrap_or(cur),
        KeyCode::Left if MAC && k.cmd => home(cur),
        KeyCode::Right if MAC && k.cmd => Pos::new(cur.line, len),
        KeyCode::Left if word => word_left(ed, cur),
        KeyCode::Right if word => word_right(ed, cur),
        KeyCode::Left => {
            if cur.col > 0 {
                Pos::new(cur.line, cur.col - 1)
            } else if cur.line > 0 {
                Pos::new(cur.line - 1, buf.line_len(cur.line - 1))
            } else {
                cur
            }
        }
        KeyCode::Right => {
            if cur.col < len {
                Pos::new(cur.line, cur.col + 1)
            } else if cur.line < last {
                Pos::new(cur.line + 1, 0)
            } else {
                cur
            }
        }
        KeyCode::Up if MAC && k.cmd => Pos::new(0, 0),
        KeyCode::Down if MAC && k.cmd => doc_end,
        KeyCode::Up => {
            keep_want = true;
            if cur.line == 0 { Pos::new(0, 0) } else { vertical(cur.line - 1) }
        }
        KeyCode::Down => {
            keep_want = true;
            if cur.line >= last { Pos::new(last, len) } else { vertical(cur.line + 1) }
        }
        KeyCode::PageUp => {
            keep_want = true;
            vertical(cur.line.saturating_sub(rows))
        }
        KeyCode::PageDown => {
            keep_want = true;
            vertical((cur.line + rows).min(last))
        }
        KeyCode::Home if k.cmd => Pos::new(0, 0),
        KeyCode::End if k.cmd => doc_end,
        KeyCode::Home => home(cur),
        KeyCode::End => Pos::new(cur.line, len),
        _ => cur,
    };
    (to, keep_want)
}

mod multi;

#[cfg(test)]
mod multi_tests;
#[cfg(test)]
mod tests;
