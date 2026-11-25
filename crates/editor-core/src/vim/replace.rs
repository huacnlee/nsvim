//! Replace mode (`R`): typed chars overwrite the text, and Backspace brings back what they
//! covered. The editor's mode stays `Insert`, so the insert session gives counts, `.` and one
//! undo step.

use crate::buffer::Pos;
use crate::editor::{Editor, KeyCode, KeyInput};

use super::normal::Cmd;
use super::{InsKind, begin, insert};

/// What one typed key did, so Backspace can put it back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Covered {
    /// It overwrote this char.
    Char(char),
    /// It was typed past the line's end.
    Appended,
    /// It split the line.
    Newline,
}

/// Starts Replace mode at the cursor, `count` times over.
pub(crate) fn start(ed: &mut Editor, count: usize, cmd: Cmd) {
    insert::start_session(ed, InsKind::Replace, count, Some(cmd));
    ed.vim.replace = Some(Vec::new());
}

/// One key in Replace mode. False leaves it to Insert mode (moves and Ctrl keys), which also
/// forgets what Backspace could bring back, as in Vim.
pub(crate) fn act(ed: &mut Editor, k: KeyInput) -> bool {
    if k.ctrl || k.cmd {
        return false;
    }
    match k.code {
        KeyCode::Char(c) => overtype(ed, c),
        KeyCode::Tab => {
            let unit = if ed.opts.spaces { " ".repeat(ed.opts.tab_width.max(1)) } else { "\t".into() };
            for c in unit.chars() {
                overtype(ed, c);
            }
        }
        KeyCode::Enter => split(ed),
        KeyCode::Backspace => back(ed),
        _ => {
            if let Some(r) = &mut ed.vim.replace {
                r.clear();
            }
            return false;
        }
    }
    true
}

fn push(ed: &mut Editor, c: Covered) {
    if let Some(r) = &mut ed.vim.replace {
        r.push(c);
    }
}

fn overtype(ed: &mut Editor, c: char) {
    begin(ed);
    let cur = ed.cursor();
    let len = ed.buf().line_len(cur.line);
    let at = ed.buf().idx(cur);
    if cur.col < len {
        let old = ed.buf().char_at(cur).unwrap_or(' ');
        ed.doc_mut().buf.replace(at, at + 1, &c.to_string());
        push(ed, Covered::Char(old));
    } else {
        ed.doc_mut().buf.insert_at(at, &c.to_string());
        push(ed, Covered::Appended);
    }
    insert::put(ed, Pos::new(cur.line, cur.col + 1));
}

fn split(ed: &mut Editor) {
    begin(ed);
    let cur = ed.cursor();
    let at = ed.buf().idx(cur);
    ed.doc_mut().buf.insert_at(at, "\n");
    push(ed, Covered::Newline);
    insert::put(ed, Pos::new(cur.line + 1, 0));
}

/// Backspace: undoes the last overwrite. Before the first one it only moves left.
fn back(ed: &mut Editor) {
    let cur = ed.cursor();
    let top = ed.vim.replace.as_mut().and_then(|r| r.pop());
    let Some(top) = top else {
        if cur.col > 0 {
            insert::put(ed, Pos::new(cur.line, cur.col - 1));
        }
        return;
    };
    begin(ed);
    let at = ed.buf().idx(cur);
    match top {
        Covered::Char(old) if cur.col > 0 => {
            ed.doc_mut().buf.replace(at - 1, at, &old.to_string());
            insert::put(ed, Pos::new(cur.line, cur.col - 1));
        }
        Covered::Appended if cur.col > 0 => {
            ed.doc_mut().buf.remove(at - 1, at);
            insert::put(ed, Pos::new(cur.line, cur.col - 1));
        }
        Covered::Newline if cur.line > 0 && cur.col == 0 => {
            let prev = ed.buf().line_len(cur.line - 1);
            ed.doc_mut().buf.remove(at - 1, at);
            insert::put(ed, Pos::new(cur.line - 1, prev));
        }
        _ => {}
    }
}
