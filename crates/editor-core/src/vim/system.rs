//! The keys of a desktop editor in the Neovim keymap, as Zed's Vim mode keeps them.
//!
//! On macOS, `⌘` shortcuts work in Normal, Visual and Insert mode: `⌘ S`, `⌘ Z`, `⌘ C`,
//! `⌘ /` and the rest. Vim has no `⌘` keys, so nothing clashes. Off macOS, Control is the
//! shortcut key, and Vim keeps its Control keys.
//!
//! In Insert mode, the editing keys of the system text fields work too: `⌘ Backspace`,
//! `⌥ Backspace`, `⌘ ←` and `⌥ ←` on macOS, `Ctrl Backspace` and `Ctrl ←` elsewhere.

use crate::buffer::Pos;
use crate::classic::{self, LineEdit};
use crate::editor::{Action, Editor, KeyCode, KeyInput, Mode, PaneCmd};

use super::insert::{backspace, delete_fwd, delete_to_line_start, open_line, put, start_session};
use super::{InsKind, begin, commit_here, dispatch, set_cur, store};

/// A `⌘` shortcut (macOS only, the caller checks). Returns false for a key it leaves to Vim.
pub(crate) fn shortcut(ed: &mut Editor, k: KeyInput, out: &mut Vec<Action>) -> bool {
    let letter = match k.code {
        KeyCode::Char(c) => Some(c.to_ascii_lowercase()),
        _ => None,
    };
    let app = match letter {
        Some('s') if !k.shift => Some(Action::Save(None)),
        Some('n') if !k.shift => Some(Action::NewBuffer),
        Some('p') if !k.shift => Some(Action::FindFile),
        Some('o') if !k.shift => Some(Action::OpenProject),
        Some('f') if k.shift => Some(Action::GrepProject),
        Some('e') if k.shift => Some(Action::FocusTree),
        Some('b') => Some(Action::ToggleTree),
        Some('\\') => Some(Action::Pane(PaneCmd::SplitRight)),
        Some(',') => Some(Action::Settings),
        _ => None,
    };
    if let Some(a) = app {
        ed.vim.pending.clear();
        out.push(a);
        return true;
    }
    let insert = ed.mode == Mode::Insert && ed.vim.replace.is_none();
    let visual = matches!(ed.mode, Mode::Visual(_));
    if !insert && !visual && ed.mode != Mode::Normal {
        return false;
    }
    ed.vim.pending.clear();
    match (letter, k.code) {
        (Some('z'), _) if insert => {
            // End the insert, undo, and type on from where the undo goes, as Zed does.
            vim(ed, &[KeyInput::plain(KeyCode::Esc)], out);
            let buf = &mut ed.doc_mut().buf;
            let at = if k.shift { buf.redo() } else { buf.undo() };
            ed.mode = Mode::Insert;
            match at {
                Some(p) => set_cur(ed, p),
                None => ed.message(if k.shift { "Already at newest change" } else { "Already at oldest change" }),
            }
            start_session(ed, InsKind::Plain, 1, None);
        }
        (Some('z'), _) => {
            if visual {
                vim(ed, &[KeyInput::plain(KeyCode::Esc)], out);
            }
            let step = if k.shift {
                KeyInput { code: KeyCode::Char('r'), ctrl: true, alt: false, shift: false, cmd: false }
            } else {
                KeyInput::ch('u')
            };
            vim(ed, &[step], out);
        }
        (Some('c'), _) if insert => {
            let line = ed.buf().line(ed.cursor().line);
            store(ed, None, format!("{line}\n"), true, true, out);
        }
        (Some('c'), _) => keys(ed, if visual { "y" } else { "yy" }, out),
        (Some('x'), _) if insert => {
            vim(ed, &[KeyInput::plain(KeyCode::Esc)], out);
            keys(ed, "ddi", out);
        }
        (Some('x'), _) => keys(ed, if visual { "d" } else { "dd" }, out),
        (Some('v'), _) if insert => out.push(Action::InsertClipboard),
        (Some('v'), _) => keys(ed, "P", out),
        (Some('a'), _) => {
            if insert || visual {
                vim(ed, &[KeyInput::plain(KeyCode::Esc)], out);
            }
            keys(ed, "ggVG", out);
        }
        (Some('/'), _) => line_edit(ed, LineEdit::Comment, insert),
        (Some(']'), _) => line_edit(ed, LineEdit::Indent, insert),
        (Some('['), _) => line_edit(ed, LineEdit::Outdent, insert),
        (Some('k'), _) if k.shift => line_edit(ed, LineEdit::Delete, insert),
        // `⌘ Enter` and `⇧ ⌘ Enter` open a line below, above, and type there.
        (_, KeyCode::Enter) => {
            if !insert {
                if visual {
                    vim(ed, &[KeyInput::plain(KeyCode::Esc)], out);
                }
                keys(ed, if k.shift { "O" } else { "o" }, out);
            } else {
                open_line(ed, !k.shift);
            }
        }
        _ => return false,
    }
    true
}

/// `⌥ ↑` and `⌥ ↓` move the lines, `⇧ ⌥ ↑` and `⇧ ⌥ ↓` copy them, in every mode and on
/// every platform. Vim does not use Alt.
pub(crate) fn line_key(ed: &mut Editor, k: KeyInput) -> bool {
    if !k.alt || k.cmd || k.ctrl || ed.vim.replace.is_some() {
        return false;
    }
    let op = match (k.code, k.shift) {
        (KeyCode::Up, false) => LineEdit::MoveUp,
        (KeyCode::Down, false) => LineEdit::MoveDown,
        (KeyCode::Up, true) => LineEdit::DuplicateUp,
        (KeyCode::Down, true) => LineEdit::DuplicateDown,
        _ => return false,
    };
    ed.vim.pending.clear();
    line_edit(ed, op, ed.mode == Mode::Insert);
    true
}

/// Runs a line command. In Insert mode it is an undo step of its own, and the insert goes on.
fn line_edit(ed: &mut Editor, op: LineEdit, insert: bool) {
    if insert {
        commit_here(ed);
    }
    classic::line_edit(ed, op);
    if insert {
        ed.mode = Mode::Insert;
        begin(ed);
    }
}

fn vim(ed: &mut Editor, ks: &[KeyInput], out: &mut Vec<Action>) {
    for k in ks {
        dispatch(ed, *k, out);
    }
}

fn keys(ed: &mut Editor, s: &str, out: &mut Vec<Action>) {
    for c in s.chars() {
        dispatch(ed, KeyInput::ch(c), out);
    }
}

/// An Insert mode editing key of the system's text fields. `mac` says whether the platform is
/// macOS. There `⌘` acts on the line and `⌥` on a word. Elsewhere Control acts on a word.
/// Returns false for any other key.
pub(crate) fn edit(ed: &mut Editor, k: KeyInput, mac: bool) -> bool {
    let line = mac && k.cmd && !k.ctrl && !k.alt;
    let word = if mac { k.alt && !k.cmd && !k.ctrl } else { k.ctrl && !k.alt };
    let cur = ed.cursor();
    let len = ed.buf().line_len(cur.line);
    match k.code {
        KeyCode::Backspace if line => {
            if cur.col == 0 {
                backspace(ed);
            } else {
                delete_to_line_start(ed);
            }
        }
        KeyCode::Delete if line => {
            if cur.col >= len {
                delete_fwd(ed);
            } else {
                remove(ed, cur, Pos::new(cur.line, len));
            }
        }
        KeyCode::Backspace if word => {
            let to = classic::word_left(ed, cur);
            remove(ed, to, cur);
        }
        KeyCode::Delete if word => {
            let to = classic::word_right(ed, cur);
            remove(ed, cur, to);
        }
        KeyCode::Left if line => {
            let fnb = ed.buf().first_non_blank(cur.line);
            put(ed, Pos::new(cur.line, if cur.col == fnb { 0 } else { fnb }));
        }
        KeyCode::Right if line => {
            put(ed, Pos::new(cur.line, len));
            ed.doc_mut().view.want_col = usize::MAX;
        }
        KeyCode::Left if word => {
            let to = classic::word_left(ed, cur);
            put(ed, to);
        }
        KeyCode::Right if word => {
            let to = classic::word_right(ed, cur);
            put(ed, to);
        }
        KeyCode::Up if line => put(ed, Pos::new(0, 0)),
        KeyCode::Home if k.cmd || k.ctrl => put(ed, Pos::new(0, 0)),
        KeyCode::Down if line => doc_end(ed),
        KeyCode::End if k.cmd || k.ctrl => doc_end(ed),
        _ => return false,
    }
    true
}

fn doc_end(ed: &mut Editor) {
    let last = ed.buf().last_line();
    let len = ed.buf().line_len(last);
    put(ed, Pos::new(last, len));
}

/// Deletes `[a, b)` in the open insert and puts the cursor at `a`.
fn remove(ed: &mut Editor, a: Pos, b: Pos) {
    if a == b {
        return;
    }
    begin(ed);
    let buf = &mut ed.doc_mut().buf;
    let (i, j) = (buf.idx(a), buf.idx(b));
    buf.remove(i, j);
    put(ed, a);
}
