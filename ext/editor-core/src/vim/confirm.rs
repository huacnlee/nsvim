//! `:s///c`: asks about each match in turn. `y` replaces it, `n` skips it, `a` replaces it and
//! every one after, `l` replaces it and stops, `q` or Esc stops. Ctrl-e and Ctrl-y scroll. The
//! match being asked about is selected, and the whole session undoes as one step.

use crate::buffer::Pos;
use crate::editor::{Action, Editor, KeyCode, KeyInput, Mode, VisualKind};

use super::regex::Regex;
use super::{begin, ch, commit_here, ctrl_ch, set_cur};

#[derive(Clone, Debug)]
pub(crate) struct Confirm {
    re: Regex,
    rep: String,
    global: bool,
    /// The line of the match being asked about, and its char columns.
    line: usize,
    m: (usize, usize),
    /// The range's last line. Replacements that add lines move it down.
    last: usize,
    subs: usize,
    lines: usize,
    last_sub_line: Option<usize>,
}

/// Starts asking at the first match on lines `first..=last`. False when there is none.
pub(crate) fn start(ed: &mut Editor, re: Regex, rep: String, global: bool, first: usize, last: usize) -> bool {
    let Some((line, m)) = find(ed, &re, first, 0, last) else { return false };
    ed.vim.confirm = Some(Confirm { re, rep, global, line, m, last, subs: 0, lines: 0, last_sub_line: None });
    show(ed);
    true
}

/// The first match at or after column `col` of `line`, on lines up to `last`.
fn find(ed: &Editor, re: &Regex, line: usize, col: usize, last: usize) -> Option<(usize, (usize, usize))> {
    let buf = ed.buf();
    for l in line..=last.min(buf.last_line()) {
        let chars: Vec<char> = buf.line(l).chars().collect();
        let from = if l == line { col } else { 0 };
        if from > chars.len() {
            continue;
        }
        if let Some(m) = re.find_at(&chars, from) {
            return Some((l, (m.start, m.end)));
        }
    }
    None
}

/// Selects the match and puts the question on the command line.
fn show(ed: &mut Editor) {
    let Some(st) = &ed.vim.confirm else { return };
    let (line, (s, e), rep) = (st.line, st.m, st.rep.clone());
    ed.mode = Mode::Visual(VisualKind::Char);
    let view = &mut ed.doc_mut().view;
    view.anchor = Some(Pos::new(line, s));
    view.cursor = Pos::new(line, e.max(s + 1) - 1);
    view.want_col = view.cursor.col;
    ed.message(format!("replace with {rep} (y/n/a/q/l/^E/^Y)?"));
}

pub(crate) fn key(ed: &mut Editor, k: KeyInput, _out: &mut Vec<Action>) {
    if k.code == KeyCode::Esc || matches!(ctrl_ch(&k), Some('[' | 'c')) {
        finish(ed);
        return;
    }
    match ctrl_ch(&k) {
        Some('e') => {
            ed.scroll_by(1);
            ed.vim.keep_view = true;
            return;
        }
        Some('y') => {
            ed.scroll_by(-1);
            ed.vim.keep_view = true;
            return;
        }
        _ => {}
    }
    match ch(&k) {
        Some('y') => {
            let (l, c) = replace(ed);
            advance(ed, l, c);
        }
        Some('l') => {
            replace(ed);
            finish(ed);
        }
        Some('n') => {
            let Some(st) = &ed.vim.confirm else { return };
            let (l, (s, e)) = (st.line, st.m);
            advance(ed, l, if e > s { e } else { e + 1 });
        }
        Some('a') => loop {
            let (l, c) = replace(ed);
            if !advance(ed, l, c) {
                break;
            }
        },
        Some('q') => finish(ed),
        _ => {}
    }
}

/// Replaces the match asked about. Returns where the search goes on: the line and column
/// after the new text.
fn replace(ed: &mut Editor) -> (usize, usize) {
    let Some(st) = ed.vim.confirm.clone() else { return (0, 0) };
    let chars: Vec<char> = ed.buf().line(st.line).chars().collect();
    let Some(m) = st.re.find_at(&chars, st.m.0).filter(|m| m.start == st.m.0) else { return (st.line, st.m.1 + 1) };
    let new = super::ex::expand(&st.rep, &chars, &m);
    ed.doc_mut().view.anchor = None;
    set_cur(ed, Pos::new(st.line, m.start));
    begin(ed);
    let a = ed.buf().idx(Pos::new(st.line, m.start));
    let b = ed.buf().idx(Pos::new(st.line, m.end));
    ed.doc_mut().buf.replace(a, b, &new);
    let added = new.matches('\n').count();
    let (line, col) = match new.rsplit_once('\n') {
        Some((_, tail)) => (st.line + added, tail.chars().count()),
        None => (st.line, m.start + new.chars().count()),
    };
    let empty = m.start == m.end;
    if let Some(c) = &mut ed.vim.confirm {
        c.subs += 1;
        if c.last_sub_line != Some(st.line) {
            c.lines += 1;
        }
        c.last_sub_line = Some(line);
        c.last += added;
    }
    (line, if empty { col + 1 } else { col })
}

/// Moves to the next match after `line`:`col` (the next line without the `g` flag). False when
/// there is none, and the session ends.
fn advance(ed: &mut Editor, line: usize, col: usize) -> bool {
    let Some(st) = ed.vim.confirm.clone() else { return false };
    let next = if st.global { find(ed, &st.re, line, col, st.last) } else { find(ed, &st.re, line + 1, 0, st.last) };
    match next {
        Some((l, m)) => {
            if let Some(c) = &mut ed.vim.confirm {
                c.line = l;
                c.m = m;
            }
            show(ed);
            true
        }
        None => {
            finish(ed);
            false
        }
    }
}

/// Ends the session: one undo step for every replacement, and Vim's count on the command line.
fn finish(ed: &mut Editor) {
    let Some(st) = ed.vim.confirm.take() else { return };
    ed.mode = Mode::Normal;
    ed.doc_mut().view.anchor = None;
    if let Some(l) = st.last_sub_line {
        let col = ed.buf().first_non_blank(l.min(ed.buf().last_line()));
        set_cur(ed, Pos::new(l, col));
    }
    if ed.buf().in_txn() {
        commit_here(ed);
    }
    if st.subs == 0 {
        ed.msg = None;
    } else {
        let s = |n: usize| if n == 1 { "" } else { "s" };
        ed.message(format!("{} substitution{} on {} line{}", st.subs, s(st.subs), st.lines, s(st.lines)));
    }
}
