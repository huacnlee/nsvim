//! Insert mode: typing with auto-indent and auto-close, and the end of an insert session.
//! The typing helpers here serve the classic keymap too.

use crate::buffer::Pos;
use crate::editor::{Action, Editor, KeyCode, KeyInput, Mode};

use super::normal::Cmd;
use super::ops::shift_lines;
use super::{Dot, InsKind, InsertSession, begin, ch, commit, ctrl_ch, set_mark};

/// Starts recording an insert session: its keys repeat for a count and for `.`.
pub(crate) fn start_session(ed: &mut Editor, kind: InsKind, count: usize, dot: Option<Cmd>) {
    ed.vim.insert = Some(InsertSession { kind, count: count.max(1), keys: Vec::new(), dot });
    ed.mode = Mode::Insert;
    begin(ed);
}

fn is_esc(k: &KeyInput) -> bool {
    k.code == KeyCode::Esc || matches!(ctrl_ch(k), Some('[' | 'c'))
}

pub(crate) fn key(ed: &mut Editor, k: KeyInput, out: &mut Vec<Action>) {
    if is_esc(&k) {
        finish(ed, out);
        return;
    }
    if ctrl_ch(&k) == Some('o') && ed.vim.pending.is_empty() {
        ed.vim.one_shot = true;
        ed.mode = Mode::Normal;
        return;
    }
    // `Ctrl r +` and `Ctrl r *` type the system clipboard, which only the app can read.
    if ed.vim.pending.first().is_some_and(|p| ctrl_ch(p) == Some('r')) && matches!(ch(&k), Some('+' | '*')) {
        ed.vim.pending.clear();
        out.push(Action::InsertClipboard);
        return;
    }
    if let Some(s) = &mut ed.vim.insert {
        s.keys.push(k);
    }
    act(ed, k);
}

/// The text a run of Insert-mode keys typed, for the `.` register.
fn typed(keys: &[KeyInput]) -> String {
    let mut out = String::new();
    for k in keys {
        match k.code {
            KeyCode::Char(c) if !k.ctrl && !k.cmd => out.push(c),
            KeyCode::Enter => out.push('\n'),
            KeyCode::Tab => out.push('\t'),
            KeyCode::Backspace => {
                out.pop();
            }
            _ => {}
        }
    }
    out
}

/// One Insert-mode key, without recording it.
pub(crate) fn act(ed: &mut Editor, k: KeyInput) {
    if ed.vim.pending.first().is_some_and(|p| ctrl_ch(p) == Some('r')) {
        ed.vim.pending.clear();
        if let Some(r) = ch(&k).and_then(|c| ed.reg(Some(c))) {
            insert_text(ed, &r.text);
        }
        return;
    }
    if ed.vim.replace.is_some() && super::replace::act(ed, k) {
        return;
    }
    let cur = ed.cursor();
    if let Some(c) = ctrl_ch(&k) {
        match c {
            'w' => delete_word_back(ed),
            'u' => {
                let fnb = ed.buf().first_non_blank(cur.line);
                let to = if cur.col > fnb { fnb } else { 0 };
                remove_back(ed, cur.col - to);
            }
            'h' => backspace(ed),
            'j' | 'm' => newline(ed),
            'r' => ed.vim.pending.push(k),
            't' | 'd' => {
                let before = ed.buf().line_len(cur.line);
                shift_lines(ed, cur.line, cur.line, c == 't', 1);
                let after = ed.buf().line_len(cur.line);
                let col = (cur.col + after).saturating_sub(before);
                put(ed, Pos::new(cur.line, col));
            }
            _ => {}
        }
        return;
    }
    match k.code {
        KeyCode::Enter => newline(ed),
        KeyCode::Backspace => backspace(ed),
        KeyCode::Delete => delete_fwd(ed),
        KeyCode::Tab if !k.shift => {
            let unit = if ed.opts.spaces {
                let tw = ed.opts.tab_width.max(1);
                " ".repeat(tw - cur.col % tw)
            } else {
                "\t".into()
            };
            insert_text(ed, &unit);
        }
        KeyCode::Left => {
            if cur.col > 0 {
                put(ed, Pos::new(cur.line, cur.col - 1));
            }
        }
        KeyCode::Right => {
            if cur.col < ed.buf().line_len(cur.line) {
                put(ed, Pos::new(cur.line, cur.col + 1));
            }
        }
        KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
            let rows = ed.rows.max(2) - 1;
            let n = if matches!(k.code, KeyCode::PageUp | KeyCode::PageDown) { rows } else { 1 };
            let line = if matches!(k.code, KeyCode::Up | KeyCode::PageUp) {
                cur.line.saturating_sub(n)
            } else {
                (cur.line + n).min(ed.buf().last_line())
            };
            vertical(ed, line);
        }
        KeyCode::Home => put(ed, Pos::new(cur.line, 0)),
        KeyCode::End => {
            let len = ed.buf().line_len(cur.line);
            put(ed, Pos::new(cur.line, len));
            ed.doc_mut().view.want_col = usize::MAX;
        }
        KeyCode::Char(c) if !k.ctrl && !k.cmd => type_char(ed, c),
        _ => {}
    }
}

/// Ends the insert session: repeats it for a count, records it for `.`, and returns to Normal.
pub(crate) fn finish(ed: &mut Editor, out: &mut Vec<Action>) {
    let _ = out;
    ed.vim.pending.clear();
    ed.vim.one_shot = false;
    let sess = ed.vim.insert.take();
    if let Some(s) = &sess {
        if s.kind == InsKind::Block {
            super::visual::finish_block_insert(ed);
        } else if s.count > 1 {
            for _ in 1..s.count {
                if matches!(s.kind, InsKind::Below | InsKind::Above) {
                    open_line(ed, true);
                }
                for k in &s.keys {
                    act(ed, *k);
                }
            }
        }
        if let Some(cmd) = &s.dot {
            ed.vim.dot = Some(Dot { cmd: cmd.clone(), keys: s.keys.clone() });
        }
        let text = typed(&s.keys);
        if !text.is_empty() {
            ed.regs.inserted = crate::editor::Reg { text, linewise: false };
        }
    }
    ed.vim.replace = None;
    if !ed.has_doc() {
        ed.mode = Mode::Normal;
        return;
    }
    let cur = ed.cursor();
    let line = ed.buf().line(cur.line);
    let mut col = cur.col;
    if !line.is_empty() && line.trim().is_empty() && col == line.chars().count() {
        let a = ed.buf().idx(Pos::new(cur.line, 0));
        ed.doc_mut().buf.remove(a, a + col);
        col = 0;
    }
    let p = Pos::new(cur.line, col);
    ensure_line(ed, p.line);
    commit(ed, p);
    set_mark(ed, '^', p);
    set_mark(ed, '.', p);
    ed.mode = Mode::Normal;
    let p = ed.buf().clamp(Pos::new(p.line, p.col.saturating_sub(1)), false);
    ed.doc_mut().view.cursor = p;
    ed.doc_mut().view.want_col = p.col;
}

/// Sets the cursor without clamping it to a char, as Insert mode may sit past the end. A line
/// after the last one is made real: the text's final `\n` ends a line rather than starting
/// one, so an empty last line needs a `\n` of its own.
pub(crate) fn put(ed: &mut Editor, p: Pos) {
    ensure_line(ed, p.line);
    let view = &mut ed.doc_mut().view;
    view.cursor = p;
    view.want_col = p.col;
}

/// Appends line ends until `line` exists.
pub(crate) fn ensure_line(ed: &mut Editor, line: usize) {
    while ed.buf().last_line() < line {
        begin(ed);
        let len = ed.buf().len_chars();
        let before = ed.buf().line_count();
        ed.doc_mut().buf.insert_at(len, "\n");
        if ed.buf().line_count() == before && ed.buf().last_line() < line {
            ed.doc_mut().buf.insert_at(len, "\n");
        }
    }
}

fn vertical(ed: &mut Editor, line: usize) {
    let want = ed.doc().view.want_col;
    let len = ed.buf().line_len(line);
    let col = if want == usize::MAX { len } else { want.min(len) };
    ed.doc_mut().view.cursor = Pos::new(line, col);
}

/// Inserts `text` at the cursor and moves the cursor past it.
pub(crate) fn insert_text(ed: &mut Editor, text: &str) {
    begin(ed);
    let cur = ed.cursor();
    let p = ed.doc_mut().buf.insert(cur, text);
    put(ed, p);
}

fn closer(c: char) -> Option<char> {
    Some(match c {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '"' => '"',
        '\'' => '\'',
        '`' => '`',
        _ => return None,
    })
}

/// Types `c`, closing brackets and quotes when `auto_close` is on.
pub(crate) fn type_char(ed: &mut Editor, c: char) {
    begin(ed);
    let cur = ed.cursor();
    let buf = ed.buf();
    let next = buf.char_at(cur);
    let prev = if cur.col > 0 { buf.char_at(Pos::new(cur.line, cur.col - 1)) } else { None };
    if ed.opts.auto_close {
        let quote = matches!(c, '"' | '\'' | '`');
        if (matches!(c, ')' | ']' | '}') || quote) && next == Some(c) {
            put(ed, Pos::new(cur.line, cur.col + 1));
            return;
        }
        let next_ok = next.is_none_or(|n| n.is_whitespace() || ")]},;:".contains(n));
        if let Some(close) = closer(c).filter(|_| next_ok) {
            let prev_ok =
                !quote || prev.is_none_or(|p| !(p.is_alphanumeric() || p == '_' || p == c || (c == '\'' && (p == '&' || p == '<'))));
            if prev_ok {
                let at = ed.buf().idx(cur);
                ed.doc_mut().buf.insert_at(at, &format!("{c}{close}"));
                put(ed, Pos::new(cur.line, cur.col + 1));
                return;
            }
        }
    }
    let at = ed.buf().idx(cur);
    ed.doc_mut().buf.insert_at(at, &c.to_string());
    put(ed, Pos::new(cur.line, cur.col + 1));
}

/// Enter: splits the line and keeps its indent, one level more after an opening bracket.
pub(crate) fn newline(ed: &mut Editor) {
    begin(ed);
    let cur = ed.cursor();
    let buf = ed.buf();
    let chars: Vec<char> = buf.line(cur.line).chars().collect();
    let col = cur.col.min(chars.len());
    let before: String = chars[..col].iter().collect();
    let indent: String = before.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
    let trimmed = before.trim_end();
    let py = buf.path.as_ref().and_then(|p| p.extension()).is_some_and(|e| e == "py");
    let opener = trimmed.chars().last().filter(|c| matches!(c, '{' | '(' | '[') || (py && *c == ':'));
    let unit = ed.indent_unit();
    let line_start = buf.idx(Pos::new(cur.line, 0));
    let mut at = buf.idx(Pos::new(cur.line, col));
    if !before.is_empty() && before.trim().is_empty() {
        ed.doc_mut().buf.remove(line_start, at);
        at = line_start;
    }
    let extra = if opener.is_some() { unit.as_str() } else { "" };
    let closes = opener.and_then(closer).is_some_and(|close| chars.get(col) == Some(&close));
    let text = if closes { format!("\n{indent}{extra}\n{indent}") } else { format!("\n{indent}{extra}") };
    ed.doc_mut().buf.insert_at(at, &text);
    put(ed, Pos::new(cur.line + 1, indent.chars().count() + extra.chars().count()));
}

fn remove_back(ed: &mut Editor, n: usize) {
    if n == 0 {
        return;
    }
    begin(ed);
    let cur = ed.cursor();
    let at = ed.buf().idx(cur);
    ed.doc_mut().buf.remove(at - n, at);
    put(ed, Pos::new(cur.line, cur.col - n));
}

/// Backspace: joins lines at column 0, deletes a bracket pair, and steps back one indent level
/// over leading spaces.
pub(crate) fn backspace(ed: &mut Editor) {
    begin(ed);
    let cur = ed.cursor();
    if cur.col == 0 {
        if cur.line == 0 {
            return;
        }
        let prev = ed.buf().line_len(cur.line - 1);
        let at = ed.buf().idx(cur);
        ed.doc_mut().buf.remove(at - 1, at);
        put(ed, Pos::new(cur.line - 1, prev));
        return;
    }
    let chars: Vec<char> = ed.buf().line(cur.line).chars().collect();
    let col = cur.col.min(chars.len());
    if col == 0 {
        put(ed, Pos::new(cur.line, 0));
        return;
    }
    if ed.opts.auto_close && col < chars.len() && closer(chars[col - 1]) == Some(chars[col]) {
        let at = ed.buf().idx(Pos::new(cur.line, col));
        ed.doc_mut().buf.remove(at - 1, at + 1);
        put(ed, Pos::new(cur.line, col - 1));
        return;
    }
    if ed.opts.spaces && chars[..col].iter().all(|c| *c == ' ') {
        let tw = ed.opts.tab_width.max(1);
        remove_back(ed, (col - 1) % tw + 1);
        return;
    }
    remove_back(ed, 1);
}

/// Delete: removes the char under the cursor, or joins the next line at the end.
pub(crate) fn delete_fwd(ed: &mut Editor) {
    begin(ed);
    let cur = ed.cursor();
    let at = ed.buf().idx(cur);
    if at < ed.buf().len_chars() {
        ed.doc_mut().buf.remove(at, at + 1);
    }
}

/// Ctrl-w: deletes the word before the cursor, or the line break at column 0.
pub(crate) fn delete_word_back(ed: &mut Editor) {
    let cur = ed.cursor();
    if cur.col == 0 {
        backspace(ed);
        return;
    }
    let chars: Vec<char> = ed.buf().line(cur.line).chars().collect();
    let mut i = cur.col.min(chars.len());
    while i > 0 && (chars[i - 1] == ' ' || chars[i - 1] == '\t') {
        i -= 1;
    }
    if i > 0 {
        let k = super::motion::char_cls(chars[i - 1], false);
        while i > 0 && super::motion::char_cls(chars[i - 1], false) == k {
            i -= 1;
        }
    }
    remove_back(ed, cur.col.min(chars.len()) - i);
}

/// Opens a line below or above the cursor's line with its indent, and puts the cursor there.
pub(crate) fn open_line(ed: &mut Editor, below: bool) {
    begin(ed);
    let cur = ed.cursor();
    let buf = ed.buf();
    let line = buf.line(cur.line);
    let mut indent = buf.indent_of(cur.line);
    if below && line.trim_end().ends_with(['{', '(', '[']) {
        indent.push_str(&ed.indent_unit());
    }
    let n = indent.chars().count();
    if below {
        let at = buf.idx(Pos::new(cur.line, buf.line_len(cur.line)));
        ed.doc_mut().buf.insert_at(at, &format!("\n{indent}"));
        put(ed, Pos::new(cur.line + 1, n));
    } else {
        let at = buf.idx(Pos::new(cur.line, 0));
        ed.doc_mut().buf.insert_at(at, &format!("{indent}\n"));
        put(ed, Pos::new(cur.line, n));
    }
}
