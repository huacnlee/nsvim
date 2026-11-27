//! The Vim keymap: Normal, Insert, Visual and Command modes over an [`Editor`].
//!
//! [`handle_key`] takes one key at a time. Normal and Visual mode collect keys until they form
//! a command (`"a3dw`), then run it. Each command is one undo step, and an insert session is
//! one step with the command that started it. Commands that change text are kept for `.`.

mod cmdline;
mod cmds;
mod confirm;
pub(crate) mod ex;
mod insert;
mod motion;
mod normal;
mod ops;
mod options;
pub(crate) mod regex;
mod replace;
pub(crate) mod search;
mod sentence;
mod tags;
mod textobj;
mod visual;

#[cfg(test)]
mod audit_tests;
#[cfg(test)]
mod ex_tests;
#[cfg(test)]
mod more_tests;
#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
mod visual_tests;

use std::collections::HashMap;
use std::path::PathBuf;

use crate::buffer::{Buffer, Pos};
use crate::editor::{Action, Editor, KeyCode, KeyInput, Keymap, Mode, Reg, VisualKind};

pub use cmdline::{cmdline_key, open_cmdline};
pub use ex::run_ex;
pub use ops::{comment_token, move_lines, toggle_comment};
pub use search::search_matches;

pub(crate) use insert::{backspace, delete_fwd, insert_text, newline, type_char};
pub(crate) use motion::char_cls;
pub(crate) use ops::shift_lines;

/// Keys typed so far of a command that isn't finished, the count, dot repeat and macros.
#[derive(Clone, Debug)]
pub struct VimState {
    /// The leader key: Space, or `\` as in stock Neovim.
    pub leader: char,
    /// Whether the last search pattern is highlighted. `:noh` turns it off until the next search.
    pub hlsearch: bool,
    pub(crate) pending: Vec<KeyInput>,
    pub(crate) last_find: Option<(char, bool, bool)>,
    pub(crate) dot: Option<Dot>,
    pub(crate) insert: Option<InsertSession>,
    /// Above 0 while `.`, a macro or `:normal` replays keys.
    pub(crate) replaying: u32,
    pub(crate) recording: Option<char>,
    pub(crate) rec_keys: Vec<KeyInput>,
    pub(crate) macros: HashMap<char, Vec<KeyInput>>,
    pub(crate) last_macro: Option<char>,
    /// Uppercase marks: the file each is in, and where it was set. Lowercase and special marks
    /// live in each buffer, so they move with its edits.
    pub(crate) file_marks: HashMap<char, (Option<PathBuf>, Pos)>,
    pub(crate) jumps: Vec<(Option<PathBuf>, Pos)>,
    pub(crate) jump_idx: usize,
    pub(crate) last_visual: HashMap<Option<PathBuf>, (Pos, Pos, VisualKind)>,
    /// Ctrl-o in Insert mode: one Normal command, then back to Insert.
    pub(crate) one_shot: bool,
    /// The mode the command line was opened from.
    pub(crate) cmd_from: Option<Mode>,
    pub(crate) ex_history: Vec<String>,
    pub(crate) search_history: Vec<String>,
    pub(crate) hist_idx: Option<usize>,
    pub(crate) hist_prefix: String,
    pub(crate) last_ex: Option<String>,
    pub(crate) put_target: Option<PutTarget>,
    pub(crate) block: Option<BlockInsert>,
    pub(crate) last_sub: Option<(String, String, String)>,
    /// The classic keymap is in a run of typing that undoes as one step.
    pub(crate) classic_typing: bool,
    /// Replace mode (`R`): what each typed char covered, for Backspace. `Some` while it runs.
    pub(crate) replace: Option<Vec<replace::Covered>>,
    /// A `:s///c` waiting for y, n, a, q or l.
    pub(crate) confirm: Option<confirm::Confirm>,
    /// An operator waiting for its `/` or `?` pattern (`d/foo<CR>`).
    pub(crate) op_search: Option<normal::Cmd>,
    /// `U`: the last changed line's file and its text before the changes.
    pub(crate) uline: Option<(Option<PathBuf>, String)>,
    /// The next settle leaves the view where a command put it (Cmd+D scrolls to a new cursor).
    pub(crate) keep_view: bool,
}

impl Default for VimState {
    fn default() -> Self {
        VimState {
            leader: ' ',
            hlsearch: true,
            pending: Vec::new(),
            last_find: None,
            dot: None,
            insert: None,
            replaying: 0,
            recording: None,
            rec_keys: Vec::new(),
            macros: HashMap::new(),
            last_macro: None,
            file_marks: HashMap::new(),
            jumps: Vec::new(),
            jump_idx: 0,
            last_visual: HashMap::new(),
            one_shot: false,
            cmd_from: None,
            ex_history: Vec::new(),
            search_history: Vec::new(),
            hist_idx: None,
            hist_prefix: String::new(),
            last_ex: None,
            put_target: None,
            block: None,
            last_sub: None,
            classic_typing: false,
            replace: None,
            confirm: None,
            op_search: None,
            uline: None,
            keep_view: false,
        }
    }
}

impl VimState {
    /// The register a macro is being recorded into, for the status line (`recording @q`).
    pub fn recording(&self) -> Option<char> {
        self.recording
    }
}

/// Whether Replace mode (`R`) runs. The editor's mode is `Insert` then; the UI labels it
/// `REPLACE`.
pub fn replacing(state: &VimState) -> bool {
    state.replace.is_some()
}

/// Whether a `:s///c` waits for an answer. The match it asks about is selected (Visual mode)
/// and the prompt is in `ed.msg`.
pub fn confirming(state: &VimState) -> bool {
    state.confirm.is_some()
}

/// The last change, for `.`: the command and the keys typed in the insert session it started.
#[derive(Clone, Debug)]
pub(crate) struct Dot {
    pub cmd: normal::Cmd,
    pub keys: Vec<KeyInput>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InsKind {
    Plain,
    Below,
    Above,
    Block,
    Replace,
}

#[derive(Clone, Debug)]
pub(crate) struct InsertSession {
    pub kind: InsKind,
    pub count: usize,
    pub keys: Vec<KeyInput>,
    pub dot: Option<normal::Cmd>,
}

/// Where a Visual `p` puts the clipboard text once it arrives.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PutTarget {
    pub at: usize,
    pub line: bool,
}

/// A Visual-block `I`, `A` or `c`: the text typed on the first line goes to the others.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BlockInsert {
    pub first: usize,
    pub last: usize,
    pub col: usize,
    pub append: bool,
    pub to_eol: bool,
    pub line_len: usize,
}

pub(crate) enum Parse<T> {
    More,
    Bad,
    Done(T),
}

/// The char a plain key types: no Ctrl or Cmd. Space included.
pub(crate) fn ch(k: &KeyInput) -> Option<char> {
    match k.code {
        KeyCode::Char(c) if !k.ctrl && !k.cmd => Some(c),
        _ => None,
    }
}

/// The letter of a Ctrl chord, lowercased.
pub(crate) fn ctrl_ch(k: &KeyInput) -> Option<char> {
    match k.code {
        KeyCode::Char(c) if k.ctrl => Some(c.to_ascii_lowercase()),
        _ => None,
    }
}

/// Handles one key in the Vim keymap, in any mode, and returns what the app must do.
pub fn handle_key(ed: &mut Editor, key: KeyInput) -> Vec<Action> {
    let mut out = Vec::new();
    // Extra cursors belong to the classic keymap.
    if ed.has_doc() && !ed.doc().extra.is_empty() {
        ed.doc_mut().extra.clear();
    }
    if ed.vim.recording.is_some() && ed.vim.replaying == 0 {
        ed.vim.rec_keys.push(key);
    }
    dispatch(ed, key, &mut out);
    out
}

/// Types `text` at the cursor in Insert mode, as one run of typing: `Ctrl r +` after the app
/// reads the clipboard.
pub fn type_text(ed: &mut Editor, text: &str) {
    if ed.has_doc() && ed.mode == Mode::Insert {
        insert::insert_text(ed, text);
    }
}

/// One key to the mode it belongs to, without recording it into a macro.
pub(crate) fn dispatch(ed: &mut Editor, key: KeyInput, out: &mut Vec<Action>) {
    if ed.vim.confirm.is_some() && ed.has_doc() {
        confirm::key(ed, key, out);
        settle(ed);
        return;
    }
    match ed.mode {
        Mode::Command => out.extend(cmdline_key(ed, key)),
        _ if !ed.has_doc() => normal::no_doc(ed, key, out),
        Mode::Insert => insert::key(ed, key, out),
        Mode::Visual(_) => visual::key(ed, key, out),
        Mode::Normal => normal::key(ed, key, out),
    }
    settle(ed);
}

/// Keeps the cursor on the text for the mode, and the view on the cursor.
pub(crate) fn settle(ed: &mut Editor) {
    if !ed.has_doc() {
        return;
    }
    let past_end = ed.mode == Mode::Insert || ed.vim.one_shot || ed.mode == Mode::Command;
    let doc = ed.doc_mut();
    doc.view.cursor = doc.buf.clamp(doc.view.cursor, past_end);
    if let Some(a) = doc.view.anchor {
        doc.view.anchor = Some(doc.buf.clamp(a, true));
    }
    if ed.vim.keep_view {
        ed.vim.keep_view = false;
    } else {
        ed.scroll_to_cursor();
    }
}

/// The keys of the unfinished command, for the command line's showcmd area (`2d`, `"a`).
pub fn showcmd(state: &VimState) -> String {
    state.pending.iter().map(|k| k.show()).collect()
}

/// Pastes `text` as `p` (after) or `P` (`before`) would, `count` times. The app calls it once the
/// system clipboard answers an [`Action::Paste`].
pub fn paste_text(ed: &mut Editor, text: &str, linewise: bool, before: bool, count: usize) {
    if !ed.has_doc() || (text.is_empty() && ed.vim.put_target.is_none()) {
        return;
    }
    put_reg(ed, text, linewise, before, count);
    settle(ed);
}

/// The char range of the word under `p` (`[a, b)`), for a double-click.
pub fn word_range_at(buf: &Buffer, p: Pos) -> (usize, usize) {
    let start = buf.idx(Pos::new(p.line, 0));
    let line: Vec<char> = buf.line(p.line).chars().collect();
    if line.is_empty() {
        return (start, start);
    }
    let col = p.col.min(line.len() - 1);
    let k = char_cls(line[col], false);
    let mut s = col;
    while s > 0 && char_cls(line[s - 1], false) == k {
        s -= 1;
    }
    let mut e = col;
    while e < line.len() && char_cls(line[e], false) == k {
        e += 1;
    }
    (start + s, start + e)
}

// Shared helpers.

/// Opens a transaction with the cursor before it, unless one is open.
pub(crate) fn begin(ed: &mut Editor) {
    if !ed.has_doc() {
        return;
    }
    let cur = ed.cursor();
    let buf = &mut ed.doc_mut().buf;
    if !buf.in_txn() {
        buf.begin(cur);
    }
}

pub(crate) fn commit(ed: &mut Editor, p: Pos) {
    if ed.has_doc() {
        ed.doc_mut().buf.commit(p);
    }
}

/// Commits the open transaction with the cursor where it is now.
pub(crate) fn commit_here(ed: &mut Editor) {
    let p = ed.cursor();
    commit(ed, p);
}

/// Moves the cursor to `p`, clamped for the mode, and makes its column the wanted one.
pub(crate) fn set_cur(ed: &mut Editor, p: Pos) {
    let past_end = ed.mode == Mode::Insert;
    let doc = ed.doc_mut();
    let p = doc.buf.clamp(p, past_end);
    doc.view.cursor = p;
    doc.view.want_col = p.col;
}

/// Stores a yank or delete. The unnamed, `+` and `*` registers also go to the system clipboard.
pub(crate) fn store(ed: &mut Editor, reg: Option<char>, text: String, linewise: bool, yank: bool, out: &mut Vec<Action>) {
    if reg == Some('_') {
        return;
    }
    let name = match reg {
        Some('"' | '+' | '*') => None,
        other => other,
    };
    ed.regs.set(name, Reg { text: text.clone(), linewise }, yank);
    if name.is_none() {
        out.push(Action::Copy(text));
    }
}

pub(crate) fn doc_key(ed: &Editor) -> Option<PathBuf> {
    if ed.has_doc() { ed.buf().path.clone() } else { None }
}

/// Sets mark `c` at `p`. Lowercase and special marks go in the buffer, so they move with its
/// edits. Uppercase marks also remember the file.
pub(crate) fn set_mark(ed: &mut Editor, c: char, p: Pos) {
    let c = if c == '`' { '\'' } else { c };
    if !ed.has_doc() {
        return;
    }
    if c.is_ascii_uppercase() {
        let key = doc_key(ed);
        ed.vim.file_marks.insert(c, (key, p));
    }
    ed.doc_mut().buf.set_mark(c, p);
}

/// Where mark `c` is in the active buffer. An uppercase mark in another file, or a mark that
/// isn't set, gives `None` and the error E20.
pub(crate) fn get_mark(ed: &mut Editor, c: char) -> Option<Pos> {
    let c = if c == '`' { '\'' } else { c };
    let p = if c.is_ascii_uppercase() {
        let key = doc_key(ed);
        match ed.vim.file_marks.get(&c) {
            Some((k, p)) if *k == key => ed.buf().mark(c).or(Some(*p)),
            _ => None,
        }
    } else if ed.has_doc() {
        ed.buf().mark(c)
    } else {
        None
    };
    if p.is_none() {
        ed.error("E20: Mark not set");
    }
    p
}

/// An uppercase mark set in a file other than the active one: the file and the position.
pub(crate) fn mark_elsewhere(ed: &Editor, c: char) -> Option<(PathBuf, Pos)> {
    if !c.is_ascii_uppercase() {
        return None;
    }
    let (k, p) = ed.vim.file_marks.get(&c)?;
    let path = k.clone()?;
    (Some(&path) != doc_key(ed).as_ref()).then_some((path, *p))
}

/// Puts the cursor's place on the jump list before a jump, and sets the `''` mark.
pub(crate) fn push_jump(ed: &mut Editor, p: Pos) {
    let key = doc_key(ed);
    let v = &mut ed.vim;
    v.jumps.truncate(v.jump_idx);
    v.jumps.retain(|(k, q)| !(k == &key && q.line == p.line));
    v.jumps.push((key, p));
    if v.jumps.len() > 100 {
        v.jumps.remove(0);
    }
    v.jump_idx = v.jumps.len();
    set_mark(ed, '\'', p);
}

/// Puts known text: `p`, `P`, a named register, or the clipboard's answer.
pub(crate) fn put_reg(ed: &mut Editor, text: &str, linewise: bool, before: bool, count: usize) {
    begin(ed);
    let count = count.max(1);
    if let Some(t) = ed.vim.put_target.take() {
        visual::put_at_target(ed, t, text, linewise, count);
    } else if ed.keymap == Keymap::Classic {
        put_classic(ed, text, linewise);
    } else if linewise {
        let mut t = text.to_string();
        if !t.ends_with('\n') {
            t.push('\n');
        }
        let rep = t.repeat(count);
        let cur = ed.cursor();
        let buf = &mut ed.doc_mut().buf;
        let first = ops::put_lines(buf, cur.line, &rep, before);
        let p = Pos::new(first, buf.first_non_blank(first));
        set_cur(ed, p);
    } else {
        let rep = text.repeat(count);
        let cur = ed.cursor();
        let buf = &mut ed.doc_mut().buf;
        let len = buf.line_len(cur.line);
        let col = if before || len == 0 { cur.col.min(len) } else { (cur.col + 1).min(len) };
        let at = buf.idx(Pos::new(cur.line, col));
        let end = buf.insert_at(at, &rep);
        let p = if rep.contains('\n') { buf.pos(at) } else { buf.pos(end.saturating_sub(1)) };
        set_cur(ed, p);
    }
    let p = ed.cursor();
    set_mark(ed, '.', p);
    commit(ed, p);
}

/// The classic paste: replaces the selection, or puts a copied line above the cursor's line.
fn put_classic(ed: &mut Editor, text: &str, linewise: bool) {
    if crate::classic::paste_multi(ed, text) {
        return;
    }
    let view = ed.doc().view;
    let sel = view.anchor.filter(|a| *a != view.cursor);
    let buf = &mut ed.doc_mut().buf;
    let mut cur = view.cursor;
    if let Some(a) = sel {
        let (s, e) = if a < cur { (a, cur) } else { (cur, a) };
        let (i, j) = (buf.idx(s), buf.idx(e));
        buf.remove(i, j);
        cur = s;
    }
    let p = if linewise && sel.is_none() {
        let mut t = text.to_string();
        if !t.ends_with('\n') {
            t.push('\n');
        }
        let n = t.matches('\n').count();
        buf.insert_at(buf.idx(Pos::new(cur.line, 0)), &t);
        Pos::new(cur.line + n, cur.col)
    } else {
        buf.insert(cur, text)
    };
    let view = &mut ed.doc_mut().view;
    view.anchor = None;
    view.cursor = p;
    view.want_col = p.col;
}
