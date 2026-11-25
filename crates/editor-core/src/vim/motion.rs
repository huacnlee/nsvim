//! Motions: where a key moves the cursor, and whether an operator covers that place.
//!
//! The word motions follow Vim's own `fwd_word`, `bck_word`, `end_word` and `bckend_word`,
//! including their stops on empty lines and at line ends, so `dw`, `cw` and `e` behave as in
//! Vim. Positions can sit on a line's end (`col == line_len`), as Vim's NUL does.

use crate::buffer::{Buffer, Pos};
use crate::editor::Editor;

use super::{Parse, ch, ctrl_ch};
use crate::editor::KeyCode;
use crate::editor::KeyInput;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Motion {
    Left,
    Right,
    Up,
    Down,
    WordFwd(bool),
    WordBack(bool),
    WordEnd(bool),
    WordEndBack(bool),
    LineStart,
    FirstNonBlank,
    LineEnd,
    LastNonBlank,
    FileTop,
    FileBottom,
    Find {
        ch: char,
        fwd: bool,
        till: bool,
    },
    RepeatFind {
        reverse: bool,
    },
    MatchPair,
    ParaFwd,
    ParaBack,
    ScreenTop,
    ScreenMid,
    ScreenBot,
    NextLine,
    PrevLine,
    LineDown,
    Column,
    SearchNext {
        reverse: bool,
    },
    Star {
        backward: bool,
        whole: bool,
    },
    Mark {
        ch: char,
        exact: bool,
    },
    /// `)` and `(`.
    Sentence {
        forward: bool,
    },
    /// `]f` `[f` `]F` `[F` and the class and parameter forms: the next or previous start (or
    /// end) of a tree-sitter node.
    Syntax {
        obj: crate::syntax::Obj,
        forward: bool,
        end: bool,
    },
}

/// How an operator treats the text up to a motion's end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MKind {
    Excl,
    Incl,
    Line,
}

/// What a motion does to the column `j` and `k` keep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Want {
    Keep,
    Max,
    Col,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Reach {
    pub pos: Pos,
    pub kind: MKind,
    pub want: Want,
    /// The motion is a jump: the place it left goes on the jump list.
    pub jump: bool,
}

impl Reach {
    fn new(pos: Pos, kind: MKind) -> Reach {
        Reach { pos, kind, want: Want::Col, jump: false }
    }

    fn jump(mut self) -> Reach {
        self.jump = true;
        self
    }

    fn want(mut self, want: Want) -> Reach {
        self.want = want;
        self
    }
}

/// Reads a motion at the start of `keys`, and how many keys it used.
pub(crate) fn parse(keys: &[KeyInput]) -> Parse<(Motion, usize)> {
    let Some(k) = keys.first() else { return Parse::More };
    let one = |m| Parse::Done((m, 1));
    match k.code {
        KeyCode::Left => return one(Motion::Left),
        KeyCode::Right => return one(Motion::Right),
        KeyCode::Up => return one(Motion::Up),
        KeyCode::Down => return one(Motion::Down),
        KeyCode::Home => return one(Motion::LineStart),
        KeyCode::End => return one(Motion::LineEnd),
        KeyCode::Enter => return one(Motion::NextLine),
        KeyCode::Backspace => return one(Motion::Left),
        _ => {}
    }
    if let Some(c) = ctrl_ch(k) {
        return match c {
            'n' | 'j' => one(Motion::Down),
            'p' => one(Motion::Up),
            'h' => one(Motion::Left),
            'm' => one(Motion::NextLine),
            _ => Parse::Bad,
        };
    }
    let Some(c) = ch(k) else { return Parse::Bad };
    let arg = |f: &dyn Fn(char) -> Motion| -> Parse<(Motion, usize)> {
        match keys.get(1) {
            None => Parse::More,
            Some(k2) => match ch(k2) {
                Some(c2) => Parse::Done((f(c2), 2)),
                None if k2.code == KeyCode::Tab => Parse::Done((f('\t'), 2)),
                None => Parse::Bad,
            },
        }
    };
    match c {
        'h' => one(Motion::Left),
        'l' | ' ' => one(Motion::Right),
        'j' => one(Motion::Down),
        'k' => one(Motion::Up),
        'w' => one(Motion::WordFwd(false)),
        'W' => one(Motion::WordFwd(true)),
        'b' => one(Motion::WordBack(false)),
        'B' => one(Motion::WordBack(true)),
        'e' => one(Motion::WordEnd(false)),
        'E' => one(Motion::WordEnd(true)),
        '0' => one(Motion::LineStart),
        '^' => one(Motion::FirstNonBlank),
        '$' => one(Motion::LineEnd),
        'G' => one(Motion::FileBottom),
        '|' => one(Motion::Column),
        '%' => one(Motion::MatchPair),
        '}' => one(Motion::ParaFwd),
        '{' => one(Motion::ParaBack),
        ')' => one(Motion::Sentence { forward: true }),
        '(' => one(Motion::Sentence { forward: false }),
        ']' | '[' => {
            use crate::syntax::Obj;
            let forward = c == ']';
            match keys.get(1).and_then(ch) {
                None if keys.len() < 2 => Parse::More,
                Some(k2 @ ('f' | 'F' | 'c' | 'C' | 'a' | 'A')) => {
                    let obj = match k2.to_ascii_lowercase() {
                        'f' => Obj::Function,
                        'c' => Obj::Class,
                        _ => Obj::Parameter,
                    };
                    Parse::Done((Motion::Syntax { obj, forward, end: k2.is_ascii_uppercase() }, 2))
                }
                _ => Parse::Bad,
            }
        }
        'H' => one(Motion::ScreenTop),
        'M' => one(Motion::ScreenMid),
        'L' => one(Motion::ScreenBot),
        '+' => one(Motion::NextLine),
        '-' => one(Motion::PrevLine),
        '_' => one(Motion::LineDown),
        'n' => one(Motion::SearchNext { reverse: false }),
        'N' => one(Motion::SearchNext { reverse: true }),
        '*' => one(Motion::Star { backward: false, whole: true }),
        '#' => one(Motion::Star { backward: true, whole: true }),
        ';' => one(Motion::RepeatFind { reverse: false }),
        ',' => one(Motion::RepeatFind { reverse: true }),
        'f' => arg(&|c| Motion::Find { ch: c, fwd: true, till: false }),
        'F' => arg(&|c| Motion::Find { ch: c, fwd: false, till: false }),
        't' => arg(&|c| Motion::Find { ch: c, fwd: true, till: true }),
        'T' => arg(&|c| Motion::Find { ch: c, fwd: false, till: true }),
        '\'' => arg(&|c| Motion::Mark { ch: c, exact: false }),
        '`' => arg(&|c| Motion::Mark { ch: c, exact: true }),
        'g' => match keys.get(1).and_then(ch) {
            None if keys.len() < 2 => Parse::More,
            Some('g') => Parse::Done((Motion::FileTop, 2)),
            Some('e') => Parse::Done((Motion::WordEndBack(false), 2)),
            Some('E') => Parse::Done((Motion::WordEndBack(true), 2)),
            Some('_') => Parse::Done((Motion::LastNonBlank, 2)),
            Some('j') => Parse::Done((Motion::Down, 2)),
            Some('k') => Parse::Done((Motion::Up, 2)),
            Some('0') => Parse::Done((Motion::LineStart, 2)),
            Some('$') => Parse::Done((Motion::LineEnd, 2)),
            Some('*') => Parse::Done((Motion::Star { backward: false, whole: false }, 2)),
            Some('#') => Parse::Done((Motion::Star { backward: true, whole: false }, 2)),
            _ => Parse::Bad,
        },
        _ => Parse::Bad,
    }
}

/// Where motion `m` goes from the cursor, `count` times. `op` says an operator waits for it.
pub(crate) fn eval(ed: &mut Editor, m: Motion, count: Option<usize>, op: bool) -> Option<Reach> {
    let n = count.unwrap_or(1).max(1);
    let cur = ed.cursor();
    let rows = ed.rows.max(1);
    let so = ed.opts.scrolloff.min(rows.saturating_sub(1) / 2);
    let top = ed.doc().view.top;
    let want_col = ed.doc().view.want_col;
    let buf = &ed.doc().buf;
    let last = buf.last_line();
    let len = buf.line_len(cur.line);
    let fnb = |l: usize| Pos::new(l, buf.first_non_blank(l));
    let line_motion = |l: usize| Reach::new(fnb(l), MKind::Line);
    Some(match m {
        Motion::Left => {
            if cur.col == 0 {
                return None;
            }
            Reach::new(Pos::new(cur.line, cur.col.saturating_sub(n)), MKind::Excl)
        }
        Motion::Right => {
            let max = if op { len } else { len.saturating_sub(1) };
            let col = (cur.col + n).min(max);
            if col == cur.col && !op {
                return None;
            }
            Reach::new(Pos::new(cur.line, col.max(cur.col)), MKind::Excl)
        }
        Motion::Up | Motion::Down => {
            let line = if m == Motion::Up {
                if cur.line == 0 {
                    return None;
                }
                cur.line.saturating_sub(n)
            } else {
                if cur.line >= last {
                    return None;
                }
                (cur.line + n).min(last)
            };
            let l = buf.line_len(line);
            let col = if want_col == usize::MAX { l.saturating_sub(1) } else { want_col.min(l.saturating_sub(1)) };
            Reach::new(Pos::new(line, col), MKind::Line).want(Want::Keep)
        }
        Motion::WordFwd(big) => {
            let p = fwd_word(buf, cur, n, big, op);
            if p == cur && !op {
                return None;
            }
            Reach::new(p, MKind::Excl)
        }
        Motion::WordBack(big) => {
            let p = bck_word(buf, cur, n, big, false);
            if p == cur {
                return None;
            }
            Reach::new(p, MKind::Excl)
        }
        Motion::WordEnd(big) => Reach::new(end_word(buf, cur, n, big, false, false), MKind::Incl),
        Motion::WordEndBack(big) => Reach::new(bckend_word(buf, cur, n, big), MKind::Incl),
        Motion::LineStart => Reach::new(Pos::new(cur.line, 0), MKind::Excl),
        Motion::FirstNonBlank => Reach::new(fnb(cur.line), MKind::Excl),
        Motion::LineEnd => {
            let line = (cur.line + n - 1).min(last);
            let col = buf.line_len(line).saturating_sub(1);
            Reach::new(Pos::new(line, col), MKind::Incl).want(Want::Max)
        }
        Motion::LastNonBlank => {
            let line = (cur.line + n - 1).min(last);
            let text: Vec<char> = buf.line(line).chars().collect();
            let col = text.iter().rposition(|c| *c != ' ' && *c != '\t').unwrap_or(0);
            Reach::new(Pos::new(line, col), MKind::Incl)
        }
        Motion::FileTop => line_motion(count.map_or(0, |c| c.saturating_sub(1)).min(last)).jump(),
        Motion::FileBottom => line_motion(count.map_or(last, |c| c.saturating_sub(1)).min(last)).jump(),
        Motion::Find { ch, fwd, till } => {
            let p = find_char(buf, cur, ch, fwd, till, n, false);
            ed.vim.last_find = Some((ch, fwd, till));
            Reach::new(p?, if fwd { MKind::Incl } else { MKind::Excl })
        }
        Motion::RepeatFind { reverse } => {
            let (ch, fwd, till) = ed.vim.last_find?;
            let fwd = fwd != reverse;
            let buf = &ed.doc().buf;
            Reach::new(find_char(buf, cur, ch, fwd, till, n, true)?, if fwd { MKind::Incl } else { MKind::Excl })
        }
        Motion::MatchPair => {
            if let Some(c) = count {
                let line = ((c * buf.line_count()).div_ceil(100)).saturating_sub(1).min(last);
                line_motion(line).jump()
            } else {
                Reach::new(match_pair(buf, cur)?, MKind::Incl).jump()
            }
        }
        Motion::ParaFwd => {
            let mut p = cur;
            for _ in 0..n {
                p = para_fwd(buf, p.line);
            }
            Reach::new(p, MKind::Excl).jump()
        }
        Motion::ParaBack => {
            let mut p = cur;
            for _ in 0..n {
                p = para_back(buf, p.line);
            }
            Reach::new(p, MKind::Excl).jump()
        }
        Motion::ScreenTop => {
            let off = if top == 0 { 0 } else { so };
            line_motion((top + (n - 1).max(off)).min(last)).jump()
        }
        Motion::ScreenMid => {
            let bottom = (top + rows).min(last + 1);
            line_motion(top + (bottom - top).saturating_sub(1) / 2).jump()
        }
        Motion::ScreenBot => {
            let bottom = (top + rows).min(last + 1).saturating_sub(1);
            let off = if bottom >= last { 0 } else { so };
            line_motion(bottom.saturating_sub((n - 1).max(off)).max(top)).jump()
        }
        Motion::NextLine => {
            if cur.line + n > last {
                return None;
            }
            line_motion(cur.line + n)
        }
        Motion::PrevLine => {
            if cur.line < n {
                return None;
            }
            line_motion(cur.line - n)
        }
        Motion::LineDown => line_motion((cur.line + n - 1).min(last)),
        Motion::Column => Reach::new(Pos::new(cur.line, (n - 1).min(len.saturating_sub(1))), MKind::Excl),
        Motion::SearchNext { reverse } => Reach::new(super::search::step(ed, reverse, n)?, MKind::Excl).jump(),
        Motion::Star { backward, whole } => Reach::new(super::search::star(ed, backward, whole, n)?, MKind::Excl).jump(),
        Motion::Sentence { forward } => Reach::new(super::sentence::motion(buf, cur, forward, n), MKind::Excl).jump(),
        Motion::Syntax { obj, forward, end } => {
            let path = buf.path.clone();
            let found = crate::syntax::jump(buf, path.as_deref(), cur, obj, forward, end, n);
            match found {
                Some(p) => Reach::new(p, MKind::Excl).jump(),
                None => {
                    let kind = super::textobj::kind_name(obj);
                    if !crate::syntax::supported(path.as_deref()) {
                        ed.error("E: no syntax tree for this file");
                    } else {
                        ed.error(format!("E: no {} {kind}", if forward { "next" } else { "previous" }));
                    }
                    return None;
                }
            }
        }
        Motion::Mark { ch, exact } => {
            let p = super::get_mark(ed, ch)?;
            let buf = &ed.doc().buf;
            let p = buf.clamp(p, false);
            if exact {
                Reach::new(p, MKind::Excl).jump()
            } else {
                Reach::new(Pos::new(p.line, buf.first_non_blank(p.line)), MKind::Line).jump()
            }
        }
    })
}

/// Vim's char classes for word motions: 0 blank (and a line's end), 1 punctuation, 2 keyword.
/// For WORD motions every non-blank is class 1.
pub(crate) fn cls(buf: &Buffer, p: Pos, big: bool) -> u8 {
    match buf.char_at(p) {
        None => 0,
        Some(c) => char_cls(c, big),
    }
}

pub(crate) fn char_cls(c: char, big: bool) -> u8 {
    if c == ' ' || c == '\t' {
        0
    } else if big {
        1
    } else if c.is_alphanumeric() || c == '_' {
        2
    } else {
        1
    }
}

/// One char forward: 0 inside the line, 2 onto the line's end, 1 onto the next line, -1 at
/// the end of the text.
pub(crate) fn inc(buf: &Buffer, p: &mut Pos) -> i32 {
    let len = buf.line_len(p.line);
    if p.col < len {
        p.col += 1;
        return if p.col < len { 0 } else { 2 };
    }
    if p.line < buf.last_line() {
        p.line += 1;
        p.col = 0;
        return 1;
    }
    -1
}

/// One char back: 0 inside the line, 1 onto the previous line's end, -1 at the start.
pub(crate) fn dec(buf: &Buffer, p: &mut Pos) -> i32 {
    let len = buf.line_len(p.line);
    if p.col > len {
        p.col = len;
    }
    if p.col > 0 {
        p.col -= 1;
        return 0;
    }
    if p.line > 0 {
        p.line -= 1;
        p.col = buf.line_len(p.line);
        return 1;
    }
    -1
}

fn line_empty(buf: &Buffer, line: usize) -> bool {
    buf.line_len(line) == 0
}

/// `w`: the start of the next word. With `eol` (an operator waits) the last word stops at
/// its line's end, so `dw` never joins lines.
pub(crate) fn fwd_word(buf: &Buffer, from: Pos, count: usize, big: bool, eol: bool) -> Pos {
    let mut p = from;
    for k in (0..count).rev() {
        let sclass = cls(buf, p, big);
        let last_line = p.line == buf.last_line();
        let i = inc(buf, &mut p);
        if i == -1 || (i >= 1 && last_line) {
            return p;
        }
        if i >= 1 && eol && k == 0 {
            return p;
        }
        if sclass != 0 {
            while cls(buf, p, big) == sclass {
                let i = inc(buf, &mut p);
                if i == -1 || (i >= 1 && eol && k == 0) {
                    return p;
                }
            }
        }
        while cls(buf, p, big) == 0 {
            if p.col == 0 && line_empty(buf, p.line) {
                break;
            }
            let i = inc(buf, &mut p);
            if i == -1 || (i >= 1 && eol && k == 0) {
                return p;
            }
        }
    }
    p
}

/// `b`: the start of this or the previous word. `stop` keeps it in the current word.
pub(crate) fn bck_word(buf: &Buffer, from: Pos, count: usize, big: bool, mut stop: bool) -> Pos {
    let mut p = from;
    'outer: for _ in 0..count {
        let sclass = cls(buf, p, big);
        if dec(buf, &mut p) == -1 {
            return p;
        }
        if !stop || sclass == cls(buf, p, big) || sclass == 0 {
            while cls(buf, p, big) == 0 {
                if p.col == 0 && line_empty(buf, p.line) {
                    stop = false;
                    continue 'outer;
                }
                if dec(buf, &mut p) == -1 {
                    return p;
                }
            }
            let c = cls(buf, p, big);
            while cls(buf, p, big) == c {
                if dec(buf, &mut p) == -1 {
                    return p;
                }
            }
        }
        inc(buf, &mut p);
        stop = false;
    }
    p
}

/// `e`: the end of this or the next word. `stop` keeps the first count in the current word
/// (`cw`), and `empty` stops on empty lines.
pub(crate) fn end_word(buf: &Buffer, from: Pos, count: usize, big: bool, mut stop: bool, empty: bool) -> Pos {
    let mut p = from;
    'outer: for _ in 0..count {
        let sclass = cls(buf, p, big);
        if inc(buf, &mut p) == -1 {
            return last_char(buf, p);
        }
        if cls(buf, p, big) == sclass && sclass != 0 {
            if skip_fwd(buf, &mut p, sclass, big) {
                return last_char(buf, p);
            }
        } else if !stop || sclass == 0 {
            while cls(buf, p, big) == 0 {
                if p.col == 0 && line_empty(buf, p.line) && empty {
                    stop = false;
                    continue 'outer;
                }
                if inc(buf, &mut p) == -1 {
                    return last_char(buf, p);
                }
            }
            let c = cls(buf, p, big);
            if skip_fwd(buf, &mut p, c, big) {
                return last_char(buf, p);
            }
        }
        dec(buf, &mut p);
        stop = false;
    }
    p
}

/// The last char of the text when a motion runs off its end.
fn last_char(buf: &Buffer, p: Pos) -> Pos {
    buf.clamp(p, false)
}

/// Moves past chars of class `c`. True when it hit the end of the text.
fn skip_fwd(buf: &Buffer, p: &mut Pos, c: u8, big: bool) -> bool {
    while cls(buf, *p, big) == c {
        if inc(buf, p) == -1 {
            return true;
        }
    }
    false
}

/// `ge`: the end of the previous word.
pub(crate) fn bckend_word(buf: &Buffer, from: Pos, count: usize, big: bool) -> Pos {
    let mut p = from;
    'outer: for _ in 0..count {
        let sclass = cls(buf, p, big);
        if dec(buf, &mut p) == -1 {
            return p;
        }
        if sclass != 0 {
            while cls(buf, p, big) == sclass {
                if dec(buf, &mut p) == -1 {
                    return p;
                }
            }
        }
        while cls(buf, p, big) == 0 {
            if p.col == 0 && line_empty(buf, p.line) {
                continue 'outer;
            }
            if dec(buf, &mut p) == -1 {
                return p;
            }
        }
    }
    p
}

/// `f`, `F`, `t` and `T` on the cursor's line. `repeat` is `;` or `,`: a till that already
/// sits next to its char looks for the next one.
pub(crate) fn find_char(buf: &Buffer, cur: Pos, c: char, fwd: bool, till: bool, count: usize, repeat: bool) -> Option<Pos> {
    let line: Vec<char> = buf.line(cur.line).chars().collect();
    let mut col = cur.col as isize;
    let step: isize = if fwd { 1 } else { -1 };
    if repeat && till {
        let next = col + step;
        if next >= 0 && (next as usize) < line.len() && line[next as usize] == c {
            col = next;
        }
    }
    for _ in 0..count {
        loop {
            col += step;
            if col < 0 || col as usize >= line.len() {
                return None;
            }
            if line[col as usize] == c {
                break;
            }
        }
    }
    let col = if till { col - step } else { col } as usize;
    Some(Pos::new(cur.line, col))
}

fn pair_of(c: char) -> Option<(char, char, bool)> {
    Some(match c {
        '(' => ('(', ')', true),
        '[' => ('[', ']', true),
        '{' => ('{', '}', true),
        ')' => ('(', ')', false),
        ']' => ('[', ']', false),
        '}' => ('{', '}', false),
        _ => return None,
    })
}

/// `%`: the bracket that matches the first bracket at or after the cursor on its line.
pub(crate) fn match_pair(buf: &Buffer, cur: Pos) -> Option<Pos> {
    let line: Vec<char> = buf.line(cur.line).chars().collect();
    let col = (cur.col..line.len()).find(|&i| pair_of(line[i]).is_some())?;
    let at = buf.idx(Pos::new(cur.line, col));
    let (open, close, forward) = pair_of(line[col])?;
    let found = if forward { match_close(buf, at, open, close) } else { match_open(buf, at, open, close) }?;
    Some(buf.pos(found))
}

/// The `close` that matches the `open` at char index `at`.
pub(crate) fn match_close(buf: &Buffer, at: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let rope = buf.rope();
    for (i, c) in rope.chars_at(at + 1).enumerate() {
        if c == open {
            depth += 1;
        } else if c == close {
            if depth == 0 {
                return Some(at + 1 + i);
            }
            depth -= 1;
        }
    }
    None
}

/// The `open` that matches the `close` at char index `at`.
pub(crate) fn match_open(buf: &Buffer, at: usize, open: char, close: char) -> Option<usize> {
    unmatched_open(buf, at, open, close)
}

/// The nearest `open` before char index `at` that no `close` between them matches.
pub(crate) fn unmatched_open(buf: &Buffer, at: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut chars = buf.rope().chars_at(at);
    let mut i = at;
    while let Some(c) = chars.prev() {
        i -= 1;
        if c == close {
            depth += 1;
        } else if c == open {
            if depth == 0 {
                return Some(i);
            }
            depth -= 1;
        }
    }
    None
}

/// `}`: the next empty line after this paragraph, or the end of the last line.
pub(crate) fn para_fwd(buf: &Buffer, line: usize) -> Pos {
    let last = buf.last_line();
    let mut l = line;
    while l < last && line_empty(buf, l) {
        l += 1;
    }
    while l < last && !line_empty(buf, l) {
        l += 1;
    }
    if l == last && !line_empty(buf, l) {
        return Pos::new(l, buf.line_len(l));
    }
    Pos::new(l, 0)
}

/// `{`: the previous empty line before this paragraph, or the first line.
pub(crate) fn para_back(buf: &Buffer, line: usize) -> Pos {
    let mut l = line;
    while l > 0 && line_empty(buf, l) {
        l -= 1;
    }
    while l > 0 && !line_empty(buf, l) {
        l -= 1;
    }
    Pos::new(l, 0)
}
