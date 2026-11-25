//! Normal mode: reading a command from the keys typed so far, and running it.

use crate::buffer::Pos;
use crate::editor::{Action, Editor, KeyCode, KeyInput, Mode, VisualKind};

use super::cmds;
use super::motion::{self, MKind, Motion, Reach, Want};
use super::ops::{self, Op, Range};
use super::textobj::{self, ObjRange, TextObj};
use super::visual::VRepeat;
use super::{Dot, InsKind, Parse, begin, ch, commit_here, ctrl_ch, insert, push_jump, set_cur, set_mark};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Cmd {
    pub reg: Option<char>,
    pub count: Option<usize>,
    pub act: Act,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Motion(Motion),
    Obj(TextObj),
    Lines,
    /// `d/pat<CR>`: the motion is a search typed on the command line (`?` when true).
    Search(bool),
    /// `dgn`: the search match under or after the cursor (`gN`: before, when false).
    Gn(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ins {
    I,
    A,
    BigI,
    BigA,
    O,
    BigO,
    Gi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scroll {
    HalfDown,
    HalfUp,
    PageDown,
    PageUp,
    LineDown,
    LineUp,
    Center,
    Top,
    Bottom,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Act {
    Move(Motion),
    Op(Op, Target),
    Put {
        before: bool,
    },
    Join {
        spaces: bool,
    },
    Replace(char),
    Tilde,
    Undo,
    Redo,
    Dot,
    Insert(Ins),
    Visual(VisualKind),
    Reselect,
    Cmdline(char),
    Record(char),
    StopRecord,
    Play(char),
    Mark(char),
    Incr(i64),
    Scroll(Scroll),
    App(Action),
    SaveQuit,
    JumpOlder,
    JumpNewer,
    Leader,
    SubRepeat,
    /// `R`: Replace mode.
    ReplaceMode,
    /// `U`: the last changed line as it was.
    LineUndo,
    /// `gn` and `gN`: selects a search match.
    SelectMatch(bool),
    /// `gf`: opens the file under the cursor.
    GotoFile,
    /// `.` after a Visual command: the same command on a region of the same size.
    VisualRepeat(VRepeat),
}

fn valid_reg(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '"' | '+' | '*' | '_' | '-' | '.' | ':' | '/')
}

/// Digits that start with 1-9, at `keys[i..]`.
pub(crate) fn parse_count(keys: &[KeyInput], mut i: usize) -> (Option<usize>, usize) {
    let mut n: Option<usize> = None;
    while let Some(c) = keys.get(i).and_then(ch) {
        match c.to_digit(10) {
            Some(d) if d > 0 || n.is_some() => {
                n = Some(n.unwrap_or(0).saturating_mul(10).saturating_add(d as usize));
                i += 1;
            }
            _ => break,
        }
    }
    (n, i)
}

pub(crate) fn mul(a: Option<usize>, b: Option<usize>) -> Option<usize> {
    match (a, b) {
        (None, None) => None,
        (a, b) => Some(a.unwrap_or(1).saturating_mul(b.unwrap_or(1))),
    }
}

/// The optional `"x` register prefix and count at the start of `keys`.
pub(crate) fn parse_prefix(keys: &[KeyInput]) -> Parse<(Option<char>, Option<usize>, usize)> {
    let mut i = 0;
    let mut reg = None;
    if keys.first().and_then(ch) == Some('"') {
        let Some(k) = keys.get(1) else { return Parse::More };
        match ch(k) {
            Some(c) if valid_reg(c) => reg = Some(c),
            _ => return Parse::Bad,
        }
        i = 2;
    }
    let (count, i) = parse_count(keys, i);
    if i >= keys.len() {
        return Parse::More;
    }
    Parse::Done((reg, count, i))
}

/// Reads a Normal-mode command from `keys`.
pub(crate) fn parse(keys: &[KeyInput], leader: char, recording: bool) -> Parse<Cmd> {
    let (reg, count, i) = match parse_prefix(keys) {
        Parse::Done(v) => v,
        Parse::More => return Parse::More,
        Parse::Bad => return Parse::Bad,
    };
    let rest = &keys[i..];
    let k = rest[0];
    let done = |act| Parse::Done(Cmd { reg, count, act });
    if i == 0 && keys.len() == 1 && ch(&k) == Some(leader) {
        return done(Act::Leader);
    }
    // Ctrl-w and a second key move between the tree and the text. The text is the right
    // window, so h, w, p and Ctrl-w go to the tree, and l stays here.
    if ctrl_ch(&k) == Some('w') {
        return match rest.get(1) {
            None => Parse::More,
            Some(k2) => match ctrl_ch(k2).or_else(|| ch(k2)) {
                Some('h' | 'w' | 'p') => done(Act::App(Action::FocusTree)),
                _ => Parse::Bad,
            },
        };
    }
    if let Some(c) = ctrl_ch(&k) {
        let act = match c {
            'r' => Act::Redo,
            'a' => Act::Incr(1),
            'x' => Act::Incr(-1),
            'd' => Act::Scroll(Scroll::HalfDown),
            'u' => Act::Scroll(Scroll::HalfUp),
            'f' => Act::Scroll(Scroll::PageDown),
            'b' => Act::Scroll(Scroll::PageUp),
            'e' => Act::Scroll(Scroll::LineDown),
            'y' => Act::Scroll(Scroll::LineUp),
            'o' => Act::JumpOlder,
            'i' => Act::JumpNewer,
            'v' | 'q' => Act::Visual(VisualKind::Block),
            _ => return motion_cmd(rest, reg, count),
        };
        return done(act);
    }
    match k.code {
        KeyCode::Tab => return done(Act::JumpNewer),
        KeyCode::Delete => return done(Act::Op(Op::Delete, Target::Motion(Motion::Right))),
        KeyCode::PageDown => return done(Act::Scroll(Scroll::PageDown)),
        KeyCode::PageUp => return done(Act::Scroll(Scroll::PageUp)),
        KeyCode::Esc => return Parse::Bad,
        _ => {}
    }
    let Some(c) = ch(&k) else { return motion_cmd(rest, reg, count) };
    let second = || rest.get(1).copied();
    let need2 = |f: &dyn Fn(KeyInput) -> Option<Act>| -> Parse<Cmd> {
        match second() {
            None => Parse::More,
            Some(k2) => match f(k2) {
                Some(act) => Parse::Done(Cmd { reg, count, act }),
                None => Parse::Bad,
            },
        }
    };
    let op = |o| op_cmd(rest, 1, o, reg, count);
    match c {
        'd' => op(Op::Delete),
        'c' => op(Op::Change),
        'y' => op(Op::Yank),
        '>' => op(Op::Indent),
        '<' => op(Op::Outdent),
        '=' => op(Op::Reindent),
        'g' => match second().and_then(|k| ch(&k)) {
            None if rest.len() < 2 => Parse::More,
            Some('u') => op_cmd(rest, 2, Op::Lower, reg, count),
            Some('U') => op_cmd(rest, 2, Op::Upper, reg, count),
            Some('~') => op_cmd(rest, 2, Op::Toggle, reg, count),
            Some('c') => op_cmd(rest, 2, Op::Comment, reg, count),
            Some('J') => done(Act::Join { spaces: false }),
            Some('i') => done(Act::Insert(Ins::Gi)),
            Some('v') => done(Act::Reselect),
            Some('t') => done(Act::App(Action::NextBuffer)),
            Some('T') => done(Act::App(Action::PrevBuffer)),
            Some('d' | 'D') => done(Act::App(Action::GotoDefinition)),
            Some('n') => done(Act::SelectMatch(true)),
            Some('N') => done(Act::SelectMatch(false)),
            Some('f') => done(Act::GotoFile),
            _ => motion_cmd(rest, reg, count),
        },
        'z' => need2(&|k| match k.code {
            KeyCode::Enter => Some(Act::Scroll(Scroll::Top)),
            _ => match ch(&k)? {
                'z' | '.' => Some(Act::Scroll(Scroll::Center)),
                't' => Some(Act::Scroll(Scroll::Top)),
                'b' | '-' => Some(Act::Scroll(Scroll::Bottom)),
                _ => None,
            },
        }),
        'Z' => need2(&|k| match ch(&k)? {
            'Z' => Some(Act::SaveQuit),
            'Q' => Some(Act::App(Action::Quit { force: true })),
            _ => None,
        }),
        'q' if recording => done(Act::StopRecord),
        'q' => need2(&|k| ch(&k).filter(|c| c.is_ascii_alphanumeric() || *c == '"').map(Act::Record)),
        '@' => need2(&|k| ch(&k).map(Act::Play)),
        'm' => need2(&|k| ch(&k).filter(|c| c.is_ascii_alphabetic() || matches!(c, '\'' | '`')).map(Act::Mark)),
        'r' => need2(&|k| match k.code {
            KeyCode::Enter => Some(Act::Replace('\n')),
            KeyCode::Tab => Some(Act::Replace('\t')),
            _ => ch(&k).map(Act::Replace),
        }),
        'x' => done(Act::Op(Op::Delete, Target::Motion(Motion::Right))),
        'X' => done(Act::Op(Op::Delete, Target::Motion(Motion::Left))),
        's' => done(Act::Op(Op::Change, Target::Motion(Motion::Right))),
        'S' => done(Act::Op(Op::Change, Target::Lines)),
        'D' => done(Act::Op(Op::Delete, Target::Motion(Motion::LineEnd))),
        'C' => done(Act::Op(Op::Change, Target::Motion(Motion::LineEnd))),
        'Y' => done(Act::Op(Op::Yank, Target::Motion(Motion::LineEnd))),
        'p' => done(Act::Put { before: false }),
        'P' => done(Act::Put { before: true }),
        'J' => done(Act::Join { spaces: true }),
        '~' => done(Act::Tilde),
        'u' => done(Act::Undo),
        'U' => done(Act::LineUndo),
        'R' => done(Act::ReplaceMode),
        '.' => done(Act::Dot),
        'i' => done(Act::Insert(Ins::I)),
        'a' => done(Act::Insert(Ins::A)),
        'I' => done(Act::Insert(Ins::BigI)),
        'A' => done(Act::Insert(Ins::BigA)),
        'o' => done(Act::Insert(Ins::O)),
        'O' => done(Act::Insert(Ins::BigO)),
        'v' => done(Act::Visual(VisualKind::Char)),
        'V' => done(Act::Visual(VisualKind::Line)),
        ':' | '/' | '?' => done(Act::Cmdline(c)),
        'K' => done(Act::App(Action::Hover)),
        '&' => done(Act::SubRepeat),
        _ => motion_cmd(rest, reg, count),
    }
}

fn motion_cmd(rest: &[KeyInput], reg: Option<char>, count: Option<usize>) -> Parse<Cmd> {
    match motion::parse(rest) {
        Parse::Done((m, n)) if n == rest.len() => Parse::Done(Cmd { reg, count, act: Act::Move(m) }),
        Parse::Done(_) | Parse::Bad => Parse::Bad,
        Parse::More => Parse::More,
    }
}

/// An operator of `len` keys at the start of `rest`, then its target.
fn op_cmd(rest: &[KeyInput], len: usize, op: Op, reg: Option<char>, count: Option<usize>) -> Parse<Cmd> {
    let after = &rest[len..];
    let (c2, j) = parse_count(after, 0);
    let t = &after[j..];
    let Some(k) = t.first() else { return Parse::More };
    let count = mul(count, c2);
    let done = |target| Parse::Done(Cmd { reg, count, act: Act::Op(op, target) });
    let last = ch(&rest[len - 1]);
    if ch(k).is_some() && ch(k) == last && t.len() == 1 {
        return done(Target::Lines);
    }
    if len == 2 && ch(k) == Some('g') {
        let Some(k2) = t.get(1) else { return Parse::More };
        if ch(k2) == last && t.len() == 2 {
            return done(Target::Lines);
        }
    }
    match ch(k) {
        Some(c @ ('/' | '?')) if t.len() == 1 => return done(Target::Search(c == '?')),
        Some('g') => match t.get(1).and_then(ch) {
            None if t.len() < 2 => return Parse::More,
            Some(c @ ('n' | 'N')) if t.len() == 2 => return done(Target::Gn(c == 'n')),
            _ => {}
        },
        _ => {}
    }
    if matches!(ch(k), Some('i' | 'a')) {
        return match textobj::parse(t) {
            Parse::Done((o, n)) if n == t.len() => done(Target::Obj(o)),
            Parse::More => Parse::More,
            _ => Parse::Bad,
        };
    }
    match motion::parse(t) {
        Parse::Done((m, n)) if n == t.len() => done(Target::Motion(m)),
        Parse::More => Parse::More,
        _ => Parse::Bad,
    }
}

/// A key while no document is open: only `:` and the leader work.
pub(crate) fn no_doc(ed: &mut Editor, k: KeyInput, out: &mut Vec<Action>) {
    ed.mode = ed.rest_mode();
    ed.vim.pending.clear();
    if ch(&k) == Some(':') {
        super::open_cmdline(ed, ':', "");
    } else if ch(&k) == Some(ed.vim.leader) {
        out.push(Action::LeaderMenu);
    }
}

pub(crate) fn key(ed: &mut Editor, k: KeyInput, out: &mut Vec<Action>) {
    if k.code == KeyCode::Esc || matches!(ctrl_ch(&k), Some('[' | 'c')) {
        ed.vim.pending.clear();
        if ed.vim.one_shot {
            ed.vim.one_shot = false;
            ed.mode = Mode::Insert;
        }
        return;
    }
    ed.vim.pending.push(k);
    let parsed = parse(&ed.vim.pending, ed.vim.leader, ed.vim.recording.is_some());
    match parsed {
        Parse::More => {}
        Parse::Bad => ed.vim.pending.clear(),
        Parse::Done(cmd) => {
            ed.vim.pending.clear();
            exec(ed, cmd, out);
            if ed.vim.one_shot && ed.mode == Mode::Normal {
                ed.vim.one_shot = false;
                ed.mode = Mode::Insert;
            } else if ed.mode != Mode::Normal && ed.mode != Mode::Command {
                ed.vim.one_shot = false;
            }
        }
    }
}

/// Moves the cursor where a motion reached, keeping the jump list and the wanted column.
pub(crate) fn go(ed: &mut Editor, r: Reach) {
    let cur = ed.cursor();
    if r.jump {
        push_jump(ed, cur);
    }
    let doc = ed.doc_mut();
    let p = doc.buf.clamp(r.pos, false);
    doc.view.cursor = p;
    match r.want {
        Want::Keep => {}
        Want::Max => doc.view.want_col = usize::MAX,
        Want::Col => doc.view.want_col = p.col,
    }
}

/// The text an operator covers when it moves to `r` from `cur`, with Vim's rules for
/// exclusive motions that end at a line's start.
pub(crate) fn motion_range(ed: &Editor, cur: Pos, r: Reach) -> Range {
    let buf = ed.buf();
    let (a, b) = if r.pos < cur { (r.pos, cur) } else { (cur, r.pos) };
    match r.kind {
        MKind::Line => Range::Lines(a.line, b.line),
        MKind::Incl => {
            let bi = buf.idx(b);
            let end = if b.col < buf.line_len(b.line) { bi + 1 } else { bi };
            Range::Chars(buf.idx(a), end)
        }
        MKind::Excl => {
            if b.col == 0 && b.line > a.line {
                let before: String = buf.line(a.line).chars().take(a.col).collect();
                if before.chars().all(|c| c == ' ' || c == '\t') {
                    return Range::Lines(a.line, b.line - 1);
                }
                let end = buf.idx(Pos::new(b.line - 1, buf.line_len(b.line - 1)));
                return Range::Chars(buf.idx(a), end);
            }
            Range::Chars(buf.idx(a), buf.idx(b))
        }
    }
}

fn target_range(ed: &mut Editor, op: Op, target: Target, count: Option<usize>) -> Option<Range> {
    let cur = ed.cursor();
    let n = count.unwrap_or(1).max(1);
    match target {
        Target::Lines => {
            let last = (cur.line + n - 1).min(ed.buf().last_line());
            Some(Range::Lines(cur.line, last))
        }
        Target::Obj(o) => match textobj::select(ed.buf(), cur, o, n) {
            Some(ObjRange::Chars(a, b)) => Some(Range::Chars(a, b)),
            Some(ObjRange::Lines(a, b)) => Some(Range::Lines(a, b)),
            None => {
                textobj::missing(ed, o);
                None
            }
        },
        Target::Gn(forward) => {
            let (a, b) = super::search::match_here(ed, forward)?;
            Some(Range::Chars(a, b))
        }
        Target::Search(_) => None,
        Target::Motion(m) => {
            // `cw` on a word changes to the word's end, as `ce` does.
            if let (Op::Change, Motion::WordFwd(big)) = (op, m)
                && ed.buf().char_at(cur).is_some_and(|c| c != ' ' && c != '\t')
            {
                let e = motion::end_word(ed.buf(), cur, n, big, true, false);
                let r = Reach { pos: e, kind: MKind::Incl, want: Want::Col, jump: false };
                return Some(motion_range(ed, cur, r));
            }
            let r = motion::eval(ed, m, count, true)?;
            if r.jump {
                push_jump(ed, cur);
            }
            Some(motion_range(ed, cur, r))
        }
    }
}

fn is_change(act: &Act) -> bool {
    match act {
        Act::Op(op, _) => *op != Op::Yank,
        Act::Put { .. } | Act::Join { .. } | Act::Replace(_) | Act::Tilde | Act::Incr(_) => true,
        _ => false,
    }
}

pub(crate) fn exec(ed: &mut Editor, cmd: Cmd, out: &mut Vec<Action>) {
    let n = cmd.count.unwrap_or(1).max(1);
    let cur = ed.cursor();
    let changes = is_change(&cmd.act);
    if changes || matches!(cmd.act, Act::Insert(_) | Act::ReplaceMode | Act::VisualRepeat(_)) {
        cmds::note_line(ed);
    }
    match cmd.act.clone() {
        Act::Move(Motion::Mark { ch, .. }) if super::mark_elsewhere(ed, ch).is_some() => {
            if let Some((path, p)) = super::mark_elsewhere(ed, ch) {
                push_jump(ed, cur);
                out.push(Action::OpenAt(path, p));
            }
        }
        Act::Move(m) => {
            if let Some(r) = motion::eval(ed, m, cmd.count, false) {
                go(ed, r);
            }
        }
        Act::Op(_, Target::Search(backward)) => {
            ed.vim.op_search = Some(cmd.clone());
            super::open_cmdline(ed, if backward { '?' } else { '/' }, "");
            return;
        }
        Act::Op(_, Target::Motion(Motion::Mark { ch, .. })) if super::mark_elsewhere(ed, ch).is_some() => {
            ed.error("E20: Mark not set");
            return;
        }
        Act::Op(op, target) => {
            let Some(range) = target_range(ed, op, target, cmd.count) else { return };
            run_op(ed, op, range, &cmd, out);
        }
        Act::Put { before } => put(ed, cmd.reg, before, n, out),
        Act::Join { spaces } => {
            if cur.line >= ed.buf().last_line() {
                return;
            }
            begin(ed);
            let at = ops::join_lines(&mut ed.doc_mut().buf, cur.line, n.max(2), spaces);
            if let Some(p) = at {
                set_cur(ed, p);
            }
            commit_here(ed);
        }
        Act::Replace(c) => {
            let len = ed.buf().line_len(cur.line);
            if cur.col + n > len {
                return;
            }
            begin(ed);
            let a = ed.buf().idx(cur);
            let p = if c == '\n' {
                ed.doc_mut().buf.replace(a, a + n, "\n");
                Pos::new(cur.line + 1, 0)
            } else {
                ed.doc_mut().buf.replace(a, a + n, &c.to_string().repeat(n));
                Pos::new(cur.line, cur.col + n - 1)
            };
            set_cur(ed, p);
            commit_here(ed);
        }
        Act::Tilde => {
            let len = ed.buf().line_len(cur.line);
            if len == 0 {
                return;
            }
            let e = (cur.col + n).min(len);
            let a = ed.buf().idx(cur);
            let b = ed.buf().idx(Pos::new(cur.line, e));
            ops::apply(ed, Op::Toggle, Range::Chars(a, b), None, 1, out);
            set_cur(ed, Pos::new(cur.line, e.min(len - 1)));
            commit_here(ed);
        }
        Act::Undo | Act::Redo => {
            for _ in 0..n {
                let buf = &mut ed.doc_mut().buf;
                let at = if cmd.act == Act::Undo { buf.undo() } else { buf.redo() };
                match at {
                    Some(p) => set_cur(ed, p),
                    None => {
                        ed.message(if cmd.act == Act::Undo { "Already at oldest change" } else { "Already at newest change" });
                        break;
                    }
                }
            }
        }
        Act::Dot => repeat_dot(ed, cmd.count, out),
        Act::Insert(kind) => start_insert(ed, kind, n, cmd.clone()),
        Act::Visual(kind) => {
            ed.mode = Mode::Visual(kind);
            ed.doc_mut().view.anchor = Some(cur);
        }
        Act::Reselect => {
            let key = super::doc_key(ed);
            if let Some((a, c, kind)) = ed.vim.last_visual.get(&key).copied() {
                ed.mode = Mode::Visual(kind);
                let buf = &ed.doc().buf;
                let (a, c) = (buf.clamp(a, false), buf.clamp(c, false));
                let view = &mut ed.doc_mut().view;
                view.anchor = Some(a);
                view.cursor = c;
            }
        }
        Act::Cmdline(c) => {
            let prefill = if c == ':' && cmd.count.is_some() && n > 1 { format!(".,.+{}", n - 1) } else { String::new() };
            super::open_cmdline(ed, c, &prefill);
        }
        Act::Record(c) => {
            ed.vim.recording = Some(c);
            ed.vim.rec_keys.clear();
            ed.message(format!("recording @{c}"));
        }
        Act::StopRecord => cmds::stop_record(ed),
        Act::Play(c) => cmds::play(ed, c, n, out),
        Act::Mark(c) => set_mark(ed, c, cur),
        Act::Incr(d) => {
            if cmds::incr(ed, d * n as i64) {
                let p = ed.cursor();
                set_mark(ed, '.', p);
            }
        }
        Act::Scroll(s) => cmds::scroll(ed, s, cmd.count),
        Act::App(a) => out.push(a),
        Act::SaveQuit => {
            if ed.buf().is_dirty() {
                out.push(Action::Save(None));
            }
            out.push(Action::Quit { force: false });
        }
        Act::JumpOlder => cmds::jump(ed, -(n as isize), out),
        Act::JumpNewer => cmds::jump(ed, n as isize, out),
        Act::Leader => out.push(Action::LeaderMenu),
        Act::SubRepeat => out.extend(super::run_ex(ed, "s")),
        Act::ReplaceMode => super::replace::start(ed, n, cmd.clone()),
        Act::LineUndo => cmds::line_undo(ed),
        Act::SelectMatch(forward) => cmds::select_match(ed, forward),
        Act::GotoFile => cmds::goto_file(ed, out),
        Act::VisualRepeat(v) => super::visual::repeat(ed, v, out),
    }
    if changes && ed.mode != Mode::Insert {
        ed.vim.dot = Some(Dot { cmd, keys: Vec::new() });
    }
}

/// Runs operator `op` on `range` for `cmd`: one undo step, or an insert session for `c`.
fn run_op(ed: &mut Editor, op: Op, range: Range, cmd: &Cmd, out: &mut Vec<Action>) {
    begin(ed);
    let entered = ops::apply(ed, op, range, cmd.reg, 1, out);
    if entered {
        if let Some(s) = &mut ed.vim.insert {
            s.dot = Some(cmd.clone());
        }
    } else {
        commit_here(ed);
    }
    if op != Op::Yank {
        let p = ed.cursor();
        set_mark(ed, '.', p);
    }
}

/// An operator whose `/` or `?` pattern arrived: it covers the text from the cursor to the
/// match, as an exclusive motion. `.` repeats it with the last search.
pub(crate) fn op_to_search(ed: &mut Editor, cmd: Cmd, out: &mut Vec<Action>) {
    let Act::Op(op, _) = cmd.act else { return };
    let n = cmd.count.unwrap_or(1).max(1);
    let Some((a, _)) = super::search::step_match(ed, false, n) else { return };
    let cur = ed.cursor();
    let r = Reach { pos: ed.buf().pos(a), kind: MKind::Excl, want: Want::Col, jump: true };
    push_jump(ed, cur);
    let range = motion_range(ed, cur, r);
    let again = Cmd { act: Act::Op(op, Target::Motion(Motion::SearchNext { reverse: false })), ..cmd };
    cmds::note_line(ed);
    run_op(ed, op, range, &again, out);
    if op != Op::Yank && ed.mode != Mode::Insert {
        ed.vim.dot = Some(Dot { cmd: again, keys: Vec::new() });
    }
}

fn put(ed: &mut Editor, reg: Option<char>, before: bool, n: usize, out: &mut Vec<Action>) {
    let clip = matches!(reg, None | Some('"' | '+' | '*'));
    if clip && ed.vim.replaying == 0 {
        out.push(Action::Paste { before, count: n });
        return;
    }
    match ed.reg(reg) {
        Some(r) => super::put_reg(ed, &r.text, r.linewise, before, n),
        None => {
            let name = reg.unwrap_or('"');
            ed.error(format!("E353: Nothing in register {name}"));
        }
    }
}

fn start_insert(ed: &mut Editor, kind: Ins, n: usize, cmd: Cmd) {
    let cur = ed.cursor();
    let buf = ed.buf();
    let len = buf.line_len(cur.line);
    let session = match kind {
        Ins::O => InsKind::Below,
        Ins::BigO => InsKind::Above,
        _ => InsKind::Plain,
    };
    let to = match kind {
        Ins::I => Some(cur),
        Ins::A => Some(Pos::new(cur.line, if len == 0 { 0 } else { (cur.col + 1).min(len) })),
        Ins::BigI => Some(Pos::new(cur.line, buf.first_non_blank(cur.line))),
        Ins::BigA => Some(Pos::new(cur.line, len)),
        Ins::Gi => Some(buf.mark('^').unwrap_or(cur)),
        Ins::O | Ins::BigO => None,
    };
    insert::start_session(ed, session, n, Some(cmd));
    match to {
        Some(p) => {
            let p = ed.buf().clamp(p, true);
            insert::put(ed, p);
        }
        None => insert::open_line(ed, kind == Ins::O),
    }
}

/// `.`: runs the last change again, with a new count when one is given.
fn repeat_dot(ed: &mut Editor, count: Option<usize>, out: &mut Vec<Action>) {
    let Some(dot) = ed.vim.dot.clone() else { return };
    let mut cmd = dot.cmd.clone();
    if count.is_some() {
        cmd.count = count;
    }
    ed.vim.replaying += 1;
    exec(ed, cmd, out);
    if ed.mode == Mode::Insert && ed.vim.insert.is_some() {
        for k in &dot.keys {
            insert::key(ed, *k, out);
        }
        insert::finish(ed, out);
    }
    ed.vim.replaying -= 1;
}
