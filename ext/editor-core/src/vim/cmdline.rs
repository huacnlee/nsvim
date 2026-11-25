//! The command line: typing `:`, `/` and `?` lines, their history, and running them.

use crate::editor::{Action, CmdLine, Editor, KeyCode, KeyInput, Keymap, Mode, Search};

use super::{ctrl_ch, push_jump, search};

/// Opens the command line with `kind` (`:`, `/` or `?`) and `prefill` typed after the caret.
pub fn open_cmdline(ed: &mut Editor, kind: char, prefill: &str) {
    ed.vim.cmd_from = Some(ed.mode);
    ed.vim.pending.clear();
    ed.vim.hist_idx = None;
    ed.cmd = Some(CmdLine { kind, text: prefill.to_string(), caret: prefill.chars().count() });
    ed.mode = Mode::Command;
    ed.msg = None;
}

/// Handles a key while the command line is open, for both keymaps.
pub fn cmdline_key(ed: &mut Editor, key: KeyInput) -> Vec<Action> {
    let mut out = Vec::new();
    let Some(mut line) = ed.cmd.clone() else {
        restore(ed, ':', false);
        return out;
    };
    let mut chars: Vec<char> = line.text.chars().collect();
    let caret = line.caret.min(chars.len());
    let ctrl = ctrl_ch(&key);
    if key.code == KeyCode::Esc || matches!(ctrl, Some('[' | 'c')) {
        ed.cmd = None;
        ed.vim.op_search = None;
        restore(ed, line.kind, false);
        return out;
    }
    if key.code == KeyCode::Enter || matches!(ctrl, Some('m' | 'j')) {
        ed.cmd = None;
        run(ed, &line, &mut out);
        super::settle(ed);
        return out;
    }
    match (key.code, ctrl) {
        (KeyCode::Backspace, _) | (_, Some('h')) => {
            if chars.is_empty() {
                ed.cmd = None;
                ed.vim.op_search = None;
                restore(ed, line.kind, false);
                return out;
            }
            if caret > 0 {
                chars.remove(caret - 1);
                line.caret = caret - 1;
            }
        }
        (KeyCode::Delete, _) => {
            if caret < chars.len() {
                chars.remove(caret);
            }
        }
        (_, Some('w')) => {
            let mut i = caret;
            while i > 0 && chars[i - 1] == ' ' {
                i -= 1;
            }
            let word = |c: char| c.is_alphanumeric() || c == '_';
            if i > 0 && word(chars[i - 1]) {
                while i > 0 && word(chars[i - 1]) {
                    i -= 1;
                }
            } else {
                i = i.saturating_sub(1);
            }
            chars.drain(i..caret);
            line.caret = i;
        }
        (_, Some('u')) => {
            chars.drain(..caret);
            line.caret = 0;
        }
        (KeyCode::Left, _) => line.caret = caret.saturating_sub(1),
        (KeyCode::Right, _) => line.caret = (caret + 1).min(chars.len()),
        (KeyCode::Home, _) | (_, Some('b')) => line.caret = 0,
        (KeyCode::End, _) | (_, Some('e')) => line.caret = chars.len(),
        (KeyCode::Up, _) | (KeyCode::Down, _) | (_, Some('p' | 'n')) => {
            let up = matches!(key.code, KeyCode::Up) || ctrl == Some('p');
            let text: String = chars.iter().collect();
            if let Some(t) = history(ed, line.kind, &text, up) {
                chars = t.chars().collect();
                line.caret = chars.len();
            }
        }
        (KeyCode::Tab, _) => {
            chars.insert(caret, '\t');
            line.caret = caret + 1;
        }
        (KeyCode::Char(c), None) if !key.cmd => {
            chars.insert(caret, c);
            line.caret = caret + 1;
        }
        _ => {}
    }
    line.text = chars.into_iter().collect();
    ed.cmd = Some(line);
    out
}

/// Steps through the history of this kind of line, keeping to entries that start with what was
/// typed before the first step.
fn history(ed: &mut Editor, kind: char, text: &str, up: bool) -> Option<String> {
    let list = if kind == ':' { &ed.vim.ex_history } else { &ed.vim.search_history };
    let len = list.len();
    if ed.vim.hist_idx.is_none() {
        ed.vim.hist_prefix = text.to_string();
    }
    let prefix = ed.vim.hist_prefix.clone();
    let mut i = ed.vim.hist_idx.unwrap_or(len);
    loop {
        if up {
            if i == 0 {
                return None;
            }
            i -= 1;
        } else {
            i += 1;
            if i >= len {
                ed.vim.hist_idx = None;
                return Some(prefix);
            }
        }
        if list[i].starts_with(&prefix) {
            ed.vim.hist_idx = Some(i);
            return Some(list[i].clone());
        }
    }
}

fn remember(list: &mut Vec<String>, text: &str) {
    if text.is_empty() {
        return;
    }
    list.retain(|t| t != text);
    list.push(text.to_string());
    if list.len() > 200 {
        list.remove(0);
    }
}

/// Puts the mode back after the command line closes. A `/` from Visual mode keeps the
/// selection. A `:` ends it.
fn restore(ed: &mut Editor, kind: char, _ran: bool) {
    let from = ed.vim.cmd_from.take().unwrap_or(ed.rest_mode());
    ed.mode = match from {
        Mode::Visual(k) if kind != ':' => Mode::Visual(k),
        Mode::Visual(_) | Mode::Command => {
            if ed.has_doc() {
                ed.doc_mut().view.anchor = None;
            }
            ed.rest_mode()
        }
        m => m,
    };
    if ed.vim.one_shot {
        ed.vim.one_shot = false;
        ed.mode = Mode::Insert;
    }
}

fn run(ed: &mut Editor, line: &CmdLine, out: &mut Vec<Action>) {
    match line.kind {
        ':' => {
            remember(&mut ed.vim.ex_history, &line.text);
            restore(ed, ':', true);
            if !line.text.trim().is_empty() {
                ed.vim.last_ex = Some(line.text.clone());
            }
            out.extend(super::run_ex(ed, &line.text));
        }
        _ => {
            remember(&mut ed.vim.search_history, &line.text);
            restore(ed, line.kind, true);
            if !ed.has_doc() {
                return;
            }
            let pattern = if line.text.is_empty() { ed.search.pattern.clone() } else { line.text.clone() };
            ed.search = Search { pattern, backward: line.kind == '?' };
            if let Some(cmd) = ed.vim.op_search.take() {
                super::normal::op_to_search(ed, cmd, out);
                if let Some(m) = ed.msg.clone().filter(|m| m.error) {
                    out.push(Action::Error(m.text));
                }
                return;
            }
            let cur = ed.cursor();
            if let Some((a, b)) = search::step_match(ed, false, 1) {
                push_jump(ed, cur);
                let buf = &ed.doc().buf;
                let (pa, pb) = (buf.pos(a), buf.pos(b));
                let classic = ed.keymap == Keymap::Classic;
                let view = &mut ed.doc_mut().view;
                if classic {
                    view.anchor = Some(pa);
                    view.cursor = pb;
                } else {
                    view.cursor = pa;
                }
                view.want_col = view.cursor.col;
            } else if let Some(m) = ed.msg.clone().filter(|m| m.error) {
                out.push(Action::Error(m.text));
            }
        }
    }
}
