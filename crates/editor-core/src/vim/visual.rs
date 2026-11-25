//! Visual mode: char, line and block selections, and the commands that act on them.

use crate::buffer::Pos;
use crate::editor::{Action, Editor, KeyCode, KeyInput, Mode, VisualKind};

use super::motion::{self, Motion};
use super::normal::{self, parse_prefix};
use super::ops::{self, Op, Range};
use super::textobj::{self, ObjRange, TextObj};
use super::{BlockInsert, Dot, InsKind, Parse, PutTarget, begin, ch, commit_here, ctrl_ch, doc_key, insert, set_cur, set_mark};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VAct {
    Move(Motion),
    Obj(TextObj),
    Op(Op),
    LineOp(Op),
    Join(bool),
    Replace(char),
    Put,
    Insert(bool),
    Swap,
    Cmdline(char),
    Kind(VisualKind),
}

struct VCmd {
    reg: Option<char>,
    count: Option<usize>,
    act: VAct,
}

/// A Visual command kept for `.`: what it did and how big its selection was. `.` runs it
/// again on a region of the same size at the cursor, as Vim does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VRepeat {
    act: VAct,
    kind: VisualKind,
    reg: Option<char>,
    count: Option<usize>,
    /// Lines the selection covered.
    lines: usize,
    /// Chars on its last line (a one-line char selection: its width), or a block's width.
    cols: usize,
    /// A block that reached each line's end (`$`).
    to_eol: bool,
}

/// Whether `.` repeats this Visual command: the ones that change text.
fn repeatable(act: VAct) -> bool {
    match act {
        VAct::Op(op) | VAct::LineOp(op) => op != Op::Yank,
        VAct::Join(_) | VAct::Replace(_) => true,
        _ => false,
    }
}

/// The size of the selection now, for `.`.
fn measure(ed: &Editor, c: &VCmd) -> VRepeat {
    let doc = ed.doc();
    let cur = doc.view.cursor;
    let a = doc.view.anchor.unwrap_or(cur);
    let (s, e) = if a <= cur { (a, cur) } else { (cur, a) };
    let k = kind(ed);
    let lines = e.line - s.line + 1;
    let cols = match k {
        VisualKind::Block => a.col.abs_diff(cur.col) + 1,
        VisualKind::Char if lines == 1 => e.col - s.col + 1,
        _ => e.col + 1,
    };
    VRepeat {
        act: c.act,
        kind: k,
        reg: c.reg,
        count: c.count,
        lines,
        cols,
        to_eol: k == VisualKind::Block && doc.view.want_col == usize::MAX,
    }
}

/// `.` after a Visual command: selects a region of the same size at the cursor and runs it.
pub(crate) fn repeat(ed: &mut Editor, r: VRepeat, out: &mut Vec<Action>) {
    let cur = ed.cursor();
    let last = ed.buf().last_line();
    let end_line = (cur.line + r.lines - 1).min(last);
    let end = match r.kind {
        VisualKind::Line => Pos::new(end_line, 0),
        VisualKind::Char if r.lines == 1 => Pos::new(cur.line, cur.col + r.cols - 1),
        VisualKind::Char => Pos::new(end_line, r.cols - 1),
        VisualKind::Block => Pos::new(end_line, cur.col + r.cols - 1),
    };
    let end = ed.buf().clamp(end, false);
    ed.mode = Mode::Visual(r.kind);
    let view = &mut ed.doc_mut().view;
    view.anchor = Some(cur);
    view.cursor = end;
    view.want_col = if r.to_eol { usize::MAX } else { end.col };
    exec(ed, VCmd { reg: r.reg, count: r.count, act: r.act }, out);
}

fn parse(keys: &[KeyInput]) -> Parse<VCmd> {
    let (reg, count, i) = match parse_prefix(keys) {
        Parse::Done(v) => v,
        Parse::More => return Parse::More,
        Parse::Bad => return Parse::Bad,
    };
    let rest = &keys[i..];
    let k = rest[0];
    let done = |act| Parse::Done(VCmd { reg, count, act });
    if ctrl_ch(&k) == Some('v') || ctrl_ch(&k) == Some('q') {
        return done(VAct::Kind(VisualKind::Block));
    }
    let motion = |rest: &[KeyInput]| match motion::parse(rest) {
        Parse::Done((m, n)) if n == rest.len() => Parse::Done(VCmd { reg, count, act: VAct::Move(m) }),
        Parse::More => Parse::More,
        _ => Parse::Bad,
    };
    let Some(c) = ch(&k) else { return motion(rest) };
    match c {
        'd' | 'x' => done(VAct::Op(Op::Delete)),
        'y' => done(VAct::Op(Op::Yank)),
        'c' | 's' => done(VAct::Op(Op::Change)),
        '>' => done(VAct::Op(Op::Indent)),
        '<' => done(VAct::Op(Op::Outdent)),
        '=' => done(VAct::Op(Op::Reindent)),
        '~' => done(VAct::Op(Op::Toggle)),
        'u' => done(VAct::Op(Op::Lower)),
        'U' => done(VAct::Op(Op::Upper)),
        'D' | 'X' => done(VAct::LineOp(Op::Delete)),
        'Y' => done(VAct::LineOp(Op::Yank)),
        'C' | 'S' | 'R' => done(VAct::LineOp(Op::Change)),
        'J' => done(VAct::Join(true)),
        'p' | 'P' => done(VAct::Put),
        'I' => done(VAct::Insert(false)),
        'A' => done(VAct::Insert(true)),
        'o' | 'O' => done(VAct::Swap),
        ':' | '/' | '?' => done(VAct::Cmdline(c)),
        'v' => done(VAct::Kind(VisualKind::Char)),
        'V' => done(VAct::Kind(VisualKind::Line)),
        'r' => match rest.get(1) {
            None => Parse::More,
            Some(k2) if k2.code == KeyCode::Enter => done(VAct::Replace('\n')),
            Some(k2) => match ch(k2) {
                Some(r) => done(VAct::Replace(r)),
                None => Parse::Bad,
            },
        },
        'g' => match rest.get(1).and_then(ch) {
            None if rest.len() < 2 => Parse::More,
            Some('u') => done(VAct::Op(Op::Lower)),
            Some('U') => done(VAct::Op(Op::Upper)),
            Some('~') => done(VAct::Op(Op::Toggle)),
            Some('c') => done(VAct::Op(Op::Comment)),
            Some('J') => done(VAct::Join(false)),
            _ => motion(rest),
        },
        'i' | 'a' => match textobj::parse(rest) {
            Parse::Done((o, 2)) if rest.len() == 2 => done(VAct::Obj(o)),
            Parse::More => Parse::More,
            _ => Parse::Bad,
        },
        _ => motion(rest),
    }
}

pub(crate) fn key(ed: &mut Editor, k: KeyInput, out: &mut Vec<Action>) {
    if k.code == KeyCode::Esc || matches!(ctrl_ch(&k), Some('[' | 'c')) {
        ed.vim.pending.clear();
        exit(ed);
        return;
    }
    ed.vim.pending.push(k);
    match parse(&ed.vim.pending) {
        Parse::More => {}
        Parse::Bad => ed.vim.pending.clear(),
        Parse::Done(c) => {
            ed.vim.pending.clear();
            let rep = (repeatable(c.act) && ed.vim.replaying == 0).then(|| measure(ed, &c));
            if rep.is_some() {
                super::cmds::note_line(ed);
            }
            exec(ed, c, out);
            if let Some(r) = rep {
                let cmd = normal::Cmd { reg: r.reg, count: None, act: normal::Act::VisualRepeat(r) };
                match &mut ed.vim.insert {
                    Some(s) if ed.mode == Mode::Insert => s.dot = Some(cmd),
                    _ => ed.vim.dot = Some(Dot { cmd, keys: Vec::new() }),
                }
            }
        }
    }
}

fn kind(ed: &Editor) -> VisualKind {
    match ed.mode {
        Mode::Visual(k) => k,
        _ => VisualKind::Char,
    }
}

/// The selected text, as an operator range.
pub(crate) fn range(ed: &Editor) -> Range {
    let doc = ed.doc();
    let buf = &doc.buf;
    let c = doc.view.cursor;
    let a = doc.view.anchor.unwrap_or(c);
    let (s, e) = if a <= c { (a, c) } else { (c, a) };
    match kind(ed) {
        VisualKind::Line => Range::Lines(s.line, e.line),
        VisualKind::Block => {
            let c1 = if doc.view.want_col == usize::MAX { usize::MAX } else { a.col.max(c.col) };
            Range::Block { first: s.line, last: e.line, c0: a.col.min(c.col), c1 }
        }
        VisualKind::Char => Range::Chars(buf.idx(s), (buf.idx(e) + 1).min(buf.len_chars())),
    }
}

/// Leaves Visual mode, keeping the selection for `gv` and the `'<` and `'>` marks.
pub(crate) fn exit(ed: &mut Editor) {
    if !ed.has_doc() {
        ed.mode = Mode::Normal;
        return;
    }
    let k = kind(ed);
    let c = ed.cursor();
    if let Some(a) = ed.doc().view.anchor {
        let (s, e) = if a <= c { (a, c) } else { (c, a) };
        set_mark(ed, '<', s);
        set_mark(ed, '>', e);
        let key = doc_key(ed);
        ed.vim.last_visual.insert(key, (a, c, k));
    }
    ed.mode = Mode::Normal;
    ed.doc_mut().view.anchor = None;
}

fn exec(ed: &mut Editor, c: VCmd, out: &mut Vec<Action>) {
    let n = c.count.unwrap_or(1).max(1);
    match c.act {
        VAct::Move(m) => {
            if let Some(r) = motion::eval(ed, m, c.count, false) {
                normal::go(ed, r);
            }
        }
        VAct::Obj(o) => select_obj(ed, o, n),
        VAct::Kind(k) => {
            if ed.mode == Mode::Visual(k) {
                exit(ed);
            } else {
                ed.mode = Mode::Visual(k);
            }
        }
        VAct::Swap => {
            let view = &mut ed.doc_mut().view;
            if let Some(a) = view.anchor {
                view.anchor = Some(view.cursor);
                view.cursor = a;
                view.want_col = a.col;
            }
        }
        VAct::Op(op) => {
            let r = range(ed);
            exit(ed);
            let entered = ops::apply(ed, op, r, c.reg, n, out);
            if !entered {
                commit_here(ed);
            }
        }
        VAct::LineOp(op) => {
            let r = range(ed);
            let block = kind(ed) == VisualKind::Block;
            exit(ed);
            let r = match (r, op) {
                (Range::Block { first, last, c0, .. }, Op::Delete | Op::Change) if block => {
                    Range::Block { first, last, c0, c1: usize::MAX }
                }
                (r, _) => {
                    let (a, b) = r.lines(ed.buf());
                    Range::Lines(a, b)
                }
            };
            let entered = ops::apply(ed, op, r, c.reg, 1, out);
            if !entered {
                commit_here(ed);
            }
        }
        VAct::Join(spaces) => {
            let (a, b) = range(ed).lines(ed.buf());
            exit(ed);
            begin(ed);
            if let Some(p) = ops::join_lines(&mut ed.doc_mut().buf, a, (b - a + 1).max(2), spaces) {
                set_cur(ed, p);
            }
            commit_here(ed);
        }
        VAct::Replace(ch) => {
            let r = range(ed);
            exit(ed);
            replace_chars(ed, r, ch);
        }
        VAct::Put => put(ed, c.reg, n, out),
        VAct::Insert(append) => visual_insert(ed, append),
        VAct::Cmdline(':') => {
            exit(ed);
            super::open_cmdline(ed, ':', "'<,'>");
        }
        VAct::Cmdline(k) => super::open_cmdline(ed, k, ""),
    }
}

fn select_obj(ed: &mut Editor, o: TextObj, n: usize) {
    let cur = ed.cursor();
    let anchor = ed.doc().view.anchor.unwrap_or(cur);
    let found = textobj::select(ed.buf(), cur, o, n);
    if found.is_none() {
        textobj::missing(ed, o);
    }
    match found {
        Some(ObjRange::Chars(a, b)) if b > a => {
            let buf = ed.buf();
            let (s, e) = (buf.pos(a), buf.pos(b - 1));
            let (na, nc) = if anchor == cur {
                (s, e)
            } else if cur >= anchor {
                (anchor.min(s), cur.max(e))
            } else {
                (anchor.max(e), cur.min(s))
            };
            ed.mode = Mode::Visual(VisualKind::Char);
            let view = &mut ed.doc_mut().view;
            view.anchor = Some(na);
            view.cursor = nc;
            view.want_col = nc.col;
        }
        Some(ObjRange::Lines(a, b)) => {
            ed.mode = Mode::Visual(VisualKind::Line);
            let view = &mut ed.doc_mut().view;
            let first = if anchor == cur { a } else { anchor.line.min(a) };
            view.anchor = Some(Pos::new(first, 0));
            view.cursor = Pos::new(b, 0);
        }
        _ => {}
    }
}

fn replace_chars(ed: &mut Editor, r: Range, c: char) {
    begin(ed);
    let buf = &mut ed.doc_mut().buf;
    let spans: Vec<(usize, usize)> = match r {
        Range::Chars(a, b) => vec![(a, b)],
        Range::Lines(a, b) => (a..=b).map(|l| (buf.idx(Pos::new(l, 0)), buf.idx(Pos::new(l, buf.line_len(l))))).collect(),
        Range::Block { first, last, c0, c1 } => (first..=last)
            .filter(|&l| c0 < buf.line_len(l))
            .map(|l| {
                let len = buf.line_len(l);
                (buf.idx(Pos::new(l, c0)), buf.idx(Pos::new(l, c1.saturating_add(1).min(len))))
            })
            .collect(),
    };
    for (a, b) in spans {
        let old = buf.slice(a, b);
        let new: String = old.chars().map(|x| if x == '\n' { x } else { c }).collect();
        if new != old {
            buf.replace(a, b, &new);
        }
    }
    let start = match r {
        Range::Chars(a, _) => buf.pos(a),
        Range::Lines(a, _) => Pos::new(a, 0),
        Range::Block { first, c0, .. } => Pos::new(first, c0),
    };
    set_cur(ed, start);
    commit_here(ed);
}

/// Visual `p`: replaces the selection with a register. The clipboard's text arrives later, so
/// the selection goes now and the text lands where it was.
fn put(ed: &mut Editor, reg: Option<char>, n: usize, out: &mut Vec<Action>) {
    let clip = matches!(reg, None | Some('"' | '+' | '*')) && ed.vim.replaying == 0;
    let known = if clip { None } else { ed.reg(reg) };
    if !clip && known.is_none() {
        exit(ed);
        return;
    }
    let r = range(ed);
    exit(ed);
    begin(ed);
    let buf = &mut ed.doc_mut().buf;
    let target = match r {
        Range::Chars(a, b) => {
            buf.remove(a, b);
            PutTarget { at: a, line: false }
        }
        Range::Lines(a, b) => {
            ops::delete_lines(buf, a, b);
            let at = if a < buf.line_count() { buf.idx(Pos::new(a, 0)) } else { buf.len_chars() };
            PutTarget { at, line: true }
        }
        Range::Block { first, last, c0, c1 } => {
            ops::delete_block(buf, first, last, c0, c1);
            PutTarget { at: buf.idx(Pos::new(first, c0)), line: false }
        }
    };
    match known {
        Some(reg) => {
            put_at_target(ed, target, &reg.text, reg.linewise, n);
            commit_here(ed);
        }
        None => {
            commit_here(ed);
            ed.vim.put_target = Some(target);
            out.push(Action::Paste { before: true, count: n });
        }
    }
}

/// Puts `text` at a Visual `p`'s target.
pub(crate) fn put_at_target(ed: &mut Editor, t: PutTarget, text: &str, linewise: bool, count: usize) {
    let rep = text.repeat(count.max(1));
    let buf = &mut ed.doc_mut().buf;
    let len = buf.len_chars();
    let at = t.at.min(len);
    let p = if t.line {
        let mut s = rep;
        if !s.ends_with('\n') {
            s.push('\n');
        }
        if at >= len && len > 0 && buf.char_idx(len - 1) != Some('\n') {
            buf.insert_at(len, &format!("\n{s}"));
            let l = buf.pos(len).line + 1;
            Pos::new(l, buf.first_non_blank(l))
        } else {
            buf.insert_at(at, &s);
            let l = buf.pos(at).line;
            Pos::new(l, buf.first_non_blank(l))
        }
    } else if linewise {
        let mut s = rep;
        if !s.ends_with('\n') {
            s.push('\n');
        }
        buf.insert_at(at, &format!("\n{s}"));
        let l = buf.pos(at).line + 1;
        Pos::new(l, buf.first_non_blank(l))
    } else {
        let end = buf.insert_at(at, &rep);
        if rep.contains('\n') { buf.pos(at) } else { buf.pos(end.saturating_sub(1)) }
    };
    set_cur(ed, p);
}

fn visual_insert(ed: &mut Editor, append: bool) {
    let r = range(ed);
    let k = kind(ed);
    let eol = ed.doc().view.want_col == usize::MAX;
    exit(ed);
    match (r, k) {
        (Range::Block { first, last, c0, c1 }, VisualKind::Block) => {
            let col = if append { c1.saturating_add(1) } else { c0 };
            start_block_insert(ed, first, last, col, append, append && eol);
        }
        (Range::Chars(a, b), _) => {
            let p = ed.buf().pos(if append { b } else { a });
            insert::start_session(ed, InsKind::Plain, 1, None);
            insert::put(ed, p);
        }
        (r, _) => {
            let (a, b) = r.lines(ed.buf());
            let buf = ed.buf();
            let p = if append { Pos::new(b, buf.line_len(b)) } else { Pos::new(a, buf.first_non_blank(a)) };
            insert::start_session(ed, InsKind::Plain, 1, None);
            insert::put(ed, p);
        }
    }
}

/// Starts typing on line `first` at `col`. On Esc the text goes to lines `first+1..=last` too.
pub(crate) fn start_block_insert(ed: &mut Editor, first: usize, last: usize, col: usize, append: bool, to_eol: bool) {
    insert::start_session(ed, InsKind::Block, 1, None);
    let len = ed.buf().line_len(first);
    let col = if to_eol { len } else { col };
    if append && len < col {
        let at = ed.buf().idx(Pos::new(first, len));
        ed.doc_mut().buf.insert_at(at, &" ".repeat(col - len));
    }
    let line_len = ed.buf().line_len(first);
    ed.vim.block = Some(BlockInsert { first, last, col, append, to_eol, line_len });
    insert::put(ed, Pos::new(first, col.min(line_len)));
}

/// Copies the text typed on a block's first line to its other lines.
pub(crate) fn finish_block_insert(ed: &mut Editor) {
    let Some(b) = ed.vim.block.take() else { return };
    let cur = ed.cursor();
    let len = ed.buf().line_len(b.first);
    // The rope's own line count: a block's last line can be the empty line after the file's
    // final line end, which `last_line` leaves out.
    if cur.line != b.first || len <= b.line_len || ed.buf().rope().len_lines().saturating_sub(1) < b.last {
        return;
    }
    let n = len - b.line_len;
    let start = b.col.min(b.line_len);
    let text: String = ed.buf().line(b.first).chars().skip(start).take(n).collect();
    let buf = &mut ed.doc_mut().buf;
    for l in b.first + 1..=b.last {
        let ll = buf.line_len(l);
        let col = if b.to_eol { ll } else { b.col };
        if !b.append && ll < col {
            continue;
        }
        if b.append && ll < col {
            let at = buf.idx(Pos::new(l, ll));
            buf.insert_at(at, &" ".repeat(col - ll));
        }
        let at = buf.idx(Pos::new(l, col));
        buf.insert_at(at, &text);
    }
}
