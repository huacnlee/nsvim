//! Operators and the line edits they share: delete, change, yank, shift, case, comment,
//! join, move and put.

use std::path::Path;

use crate::buffer::{Buffer, Pos};
use crate::editor::{Action, Editor, Mode};

use super::{InsKind, begin, set_cur, store};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    Delete,
    Change,
    Yank,
    Indent,
    Outdent,
    Reindent,
    Lower,
    Upper,
    Toggle,
    Comment,
}

/// Text an operator works on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Range {
    /// Chars `[a, b)`.
    Chars(usize, usize),
    /// Lines `first..=last`.
    Lines(usize, usize),
    /// Columns `c0..=c1` of lines `first..=last`. `c1 == usize::MAX` reaches each line's end.
    Block { first: usize, last: usize, c0: usize, c1: usize },
}

impl Range {
    /// The lines the range touches.
    pub(crate) fn lines(self, buf: &Buffer) -> (usize, usize) {
        match self {
            Range::Lines(a, b) => (a, b),
            Range::Block { first, last, .. } => (first, last),
            Range::Chars(a, b) => {
                let first = buf.pos(a).line;
                let last = if b > a { buf.pos(b - 1).line } else { first };
                (first, last)
            }
        }
    }
}

/// Runs `op` on `range`. `times` is how far `>` and `<` shift. Returns true when the operator
/// left the editor in Insert mode (`c`).
pub(crate) fn apply(ed: &mut Editor, op: Op, range: Range, reg: Option<char>, times: usize, out: &mut Vec<Action>) -> bool {
    begin(ed);
    let cur = ed.cursor();
    match op {
        Op::Delete => {
            delete(ed, range, reg, out);
            report_lines(ed, range, "fewer lines");
        }
        Op::Yank => {
            let (text, linewise) = range_text(&ed.doc().buf, range);
            store(ed, reg, text, linewise, true, out);
            let buf = &ed.doc().buf;
            let to = match range {
                Range::Chars(a, _) => buf.pos(a),
                Range::Lines(a, _) => Pos::new(a, if a == cur.line { cur.col } else { cur.col.min(buf.line_len(a)) }),
                Range::Block { first, c0, .. } => Pos::new(first, c0),
            };
            set_cur(ed, to);
            report_lines(ed, range, "lines yanked");
        }
        Op::Change => {
            change(ed, range, reg, out);
            return true;
        }
        Op::Indent | Op::Outdent => {
            let (a, b) = range.lines(&ed.doc().buf);
            shift_lines(ed, a, b, op == Op::Indent, times.max(1));
            let n = b - a + 1;
            if n > 2 {
                let sign = if op == Op::Indent { '>' } else { '<' };
                ed.message(format!("{n} lines {sign}ed {} time{}", times.max(1), if times > 1 { "s" } else { "" }));
            }
            set_cur(ed, Pos::new(a, ed.doc().buf.first_non_blank(a)));
        }
        Op::Reindent => {
            let (a, b) = range.lines(&ed.doc().buf);
            reindent(ed, a, b);
            set_cur(ed, Pos::new(a, ed.doc().buf.first_non_blank(a)));
        }
        Op::Lower | Op::Upper | Op::Toggle => {
            map_case(ed, range, op);
            let buf = &ed.doc().buf;
            let to = match range {
                Range::Chars(a, _) => buf.pos(a),
                Range::Lines(a, _) => Pos::new(a, if a == cur.line { cur.col } else { 0 }),
                Range::Block { first, c0, .. } => Pos::new(first, c0),
            };
            set_cur(ed, to);
        }
        Op::Comment => {
            let (a, b) = range.lines(&ed.doc().buf);
            toggle_comment(ed, a, b);
            set_cur(ed, Pos::new(a, cur.col));
        }
    }
    false
}

fn report_lines(ed: &mut Editor, range: Range, what: &str) {
    if let Range::Lines(a, b) = range {
        let n = b - a + 1;
        if n > 2 {
            ed.message(format!("{n} {what}"));
        }
    }
}

/// The text of a range, and whether it is whole lines.
pub(crate) fn range_text(buf: &Buffer, range: Range) -> (String, bool) {
    match range {
        Range::Chars(a, b) => (buf.slice(a, b), false),
        Range::Lines(a, b) => (lines_text(buf, a, b), true),
        Range::Block { first, last, c0, c1 } => {
            let parts: Vec<String> = (first..=last)
                .map(|l| {
                    let line: Vec<char> = buf.line(l).chars().collect();
                    let s = c0.min(line.len());
                    let e = c1.saturating_add(1).min(line.len());
                    line[s..e.max(s)].iter().collect()
                })
                .collect();
            (parts.join("\n"), false)
        }
    }
}

/// Lines `a..=b` as register text: each line and its `\n`.
pub(crate) fn lines_text(buf: &Buffer, a: usize, b: usize) -> String {
    let mut s = String::new();
    for l in a..=b.min(buf.last_line()) {
        s.push_str(&buf.line(l));
        s.push('\n');
    }
    s
}

/// The chars to remove to delete lines `a..=b` whole. The last line without a `\n` takes the
/// `\n` before it instead.
pub(crate) fn line_span(buf: &Buffer, a: usize, b: usize) -> (usize, usize) {
    let rope = buf.rope();
    let start = buf.idx(Pos::new(a, 0));
    if b + 1 < rope.len_lines() {
        (start, rope.line_to_char(b + 1))
    } else {
        let end = buf.len_chars();
        (if start > 0 { start - 1 } else { start }, end)
    }
}

/// Deletes lines `a..=b` and returns their text.
pub(crate) fn delete_lines(buf: &mut Buffer, a: usize, b: usize) -> String {
    let text = lines_text(buf, a, b);
    let (s, e) = line_span(buf, a, b);
    buf.remove(s, e);
    text
}

/// Replaces the text of lines `a..=b` (not their line ends) with `lines`, one or more.
pub(crate) fn replace_lines(buf: &mut Buffer, a: usize, b: usize, lines: &[String]) {
    let s = buf.idx(Pos::new(a, 0));
    let e = buf.idx(Pos::new(b, buf.line_len(b)));
    buf.replace(s, e, &lines.join("\n"));
}

/// Inserts `text` (whole lines, ending in `\n`) below or above `line`. Returns the first new line.
pub(crate) fn put_lines(buf: &mut Buffer, line: usize, text: &str, above: bool) -> usize {
    let mut text = text.to_string();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    if above {
        let at = buf.idx(Pos::new(line, 0));
        buf.insert_at(at, &text);
        return line;
    }
    let next = line + 1;
    let rope_lines = buf.rope().len_lines();
    let len = buf.len_chars();
    let ends_nl = len > 0 && buf.char_idx(len - 1) == Some('\n');
    if next < buf.line_count() || ends_nl {
        let at = if next < rope_lines { buf.rope().line_to_char(next) } else { len };
        buf.insert_at(at, &text);
    } else {
        buf.insert_at(len, &format!("\n{text}"));
    }
    next
}

fn delete(ed: &mut Editor, range: Range, reg: Option<char>, out: &mut Vec<Action>) {
    let (text, linewise) = range_text(&ed.doc().buf, range);
    if matches!(range, Range::Chars(a, b) if a == b) {
        return;
    }
    store(ed, reg, text, linewise, false, out);
    let buf = &mut ed.doc_mut().buf;
    let to = match range {
        Range::Chars(a, b) => {
            buf.remove(a, b);
            buf.pos(a)
        }
        Range::Lines(a, b) => {
            delete_lines(buf, a, b);
            let l = a.min(buf.last_line());
            Pos::new(l, buf.first_non_blank(l))
        }
        Range::Block { first, last, c0, c1 } => {
            delete_block(buf, first, last, c0, c1);
            Pos::new(first, c0)
        }
    };
    set_cur(ed, to);
}

pub(crate) fn delete_block(buf: &mut Buffer, first: usize, last: usize, c0: usize, c1: usize) {
    for l in first..=last {
        let len = buf.line_len(l);
        if c0 < len {
            let s = buf.idx(Pos::new(l, c0));
            let e = buf.idx(Pos::new(l, c1.saturating_add(1).min(len)));
            buf.remove(s, e);
        }
    }
}

fn change(ed: &mut Editor, range: Range, reg: Option<char>, out: &mut Vec<Action>) {
    match range {
        Range::Chars(a, b) => {
            if a < b {
                let text = ed.doc().buf.slice(a, b);
                store(ed, reg, text, false, false, out);
                ed.doc_mut().buf.remove(a, b);
            }
            let p = ed.doc().buf.pos(a);
            enter(ed, p);
        }
        Range::Lines(a, b) => {
            let buf = &ed.doc().buf;
            let text = lines_text(buf, a, b);
            let indent = buf.indent_of(a);
            store(ed, reg, text, true, false, out);
            let buf = &mut ed.doc_mut().buf;
            replace_lines(buf, a, b, std::slice::from_ref(&indent));
            let col = indent.chars().count();
            enter(ed, Pos::new(a, col));
        }
        Range::Block { first, last, c0, c1 } => {
            let (text, _) = range_text(&ed.doc().buf, range);
            store(ed, reg, text, false, false, out);
            delete_block(&mut ed.doc_mut().buf, first, last, c0, c1);
            super::visual::start_block_insert(ed, first, last, c0, false, false);
        }
    }
}

fn enter(ed: &mut Editor, p: Pos) {
    ed.mode = Mode::Insert;
    ed.doc_mut().view.cursor = p;
    ed.doc_mut().view.want_col = p.col;
    if ed.vim.insert.is_none() {
        super::insert::start_session(ed, InsKind::Plain, 1, None);
    }
}

/// Adds or removes one indent level, `times` times, on lines `a..=b`. Empty lines stay empty.
pub(crate) fn shift_lines(ed: &mut Editor, a: usize, b: usize, right: bool, times: usize) {
    let unit = ed.indent_unit();
    let tw = ed.opts.tab_width.max(1);
    let buf = &mut ed.doc_mut().buf;
    for l in a..=b.min(buf.last_line()) {
        let line = buf.line(l);
        if line.is_empty() {
            continue;
        }
        let new = if right {
            format!("{}{line}", unit.repeat(times))
        } else {
            let mut s = line.as_str();
            for _ in 0..times {
                if let Some(r) = s.strip_prefix('\t') {
                    s = r;
                } else {
                    let n = s.chars().take(tw).take_while(|c| *c == ' ').count();
                    s = &s[n..];
                }
            }
            s.to_string()
        };
        if new != line {
            replace_lines(buf, l, l, &[new]);
        }
    }
}

/// `=`: gives each line the indent of the line above it, one level more after an opening
/// bracket and one less before a closing one.
fn reindent(ed: &mut Editor, a: usize, b: usize) {
    let unit = ed.indent_unit();
    let buf = &mut ed.doc_mut().buf;
    for l in a..=b.min(buf.last_line()) {
        let line = buf.line(l);
        let body = line.trim_start();
        if body.is_empty() {
            continue;
        }
        let prev = (0..l).rev().find(|&p| !buf.line(p).trim().is_empty());
        let mut indent = prev.map(|p| buf.indent_of(p)).unwrap_or_default();
        if let Some(p) = prev {
            let pl = buf.line(p);
            if pl.trim_end().ends_with(['{', '(', '[']) {
                indent.push_str(&unit);
            }
        }
        if body.starts_with(['}', ')', ']']) {
            for _ in 0..unit.chars().count() {
                if indent.ends_with(' ') || indent.ends_with('\t') {
                    indent.pop();
                }
            }
        }
        let new = format!("{indent}{body}");
        if new != line {
            replace_lines(buf, l, l, &[new]);
        }
    }
}

fn case_map(s: &str, op: Op) -> String {
    match op {
        Op::Lower => s.to_lowercase(),
        Op::Upper => s.to_uppercase(),
        _ => s
            .chars()
            .flat_map(|c| -> Vec<char> {
                if c.is_lowercase() { c.to_uppercase().collect() } else { c.to_lowercase().collect() }
            })
            .collect(),
    }
}

fn map_case(ed: &mut Editor, range: Range, op: Op) {
    let buf = &mut ed.doc_mut().buf;
    match range {
        Range::Chars(a, b) => {
            let s = buf.slice(a, b);
            let new = case_map(&s, op);
            if new != s {
                buf.replace(a, b, &new);
            }
        }
        Range::Lines(a, b) => {
            for l in a..=b {
                let s = buf.line(l);
                let new = case_map(&s, op);
                if new != s {
                    replace_lines(buf, l, l, &[new]);
                }
            }
        }
        Range::Block { first, last, c0, c1 } => {
            for l in first..=last {
                let len = buf.line_len(l);
                if c0 < len {
                    let s = buf.idx(Pos::new(l, c0));
                    let e = buf.idx(Pos::new(l, c1.saturating_add(1).min(len)));
                    let t = buf.slice(s, e);
                    let new = case_map(&t, op);
                    if new != t {
                        buf.replace(s, e, &new);
                    }
                }
            }
        }
    }
}

/// The line comment token for a file, by its extension or name.
pub fn comment_token(path: Option<&Path>) -> &'static str {
    let Some(path) = path else { return "//" };
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    match name.as_str() {
        "makefile" | "gnumakefile" | "dockerfile" | "containerfile" | ".gitignore" | ".bashrc" | ".zshrc" | ".profile"
        | ".bash_profile" | ".env" | ".editorconfig" | "cmakelists.txt" | "justfile" | "pkgbuild" => return "#",
        _ => {}
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "py" | "pyi" | "sh" | "bash" | "zsh" | "fish" | "toml" | "yaml" | "yml" | "rb" | "conf" | "cfg" | "pl" | "pm" | "r" | "cmake"
        | "mk" | "nix" | "ex" | "exs" | "jl" | "tf" | "tfvars" | "ps1" | "psm1" | "env" | "desktop" | "gitignore" | "dockerfile"
        | "awk" | "tcl" | "coffee" | "cr" | "nim" | "gd" => "#",
        "lua" | "sql" | "hs" | "elm" | "ada" | "adb" | "ads" | "purs" => "--",
        "lisp" | "clj" | "cljs" | "cljc" | "edn" | "el" | "scm" | "rkt" | "ini" | "asm" | "s" => ";",
        "tex" | "sty" | "erl" | "hrl" | "m" => "%",
        "vim" => "\"",
        _ => "//",
    }
}

/// Comments lines `first..=last` out, or back in when every non-blank one is commented.
pub fn toggle_comment(ed: &mut Editor, first: usize, last: usize) {
    if !ed.has_doc() {
        return;
    }
    let token = comment_token(ed.doc().buf.path.as_deref());
    let cur = ed.cursor();
    let buf = &mut ed.doc_mut().buf;
    let last = last.min(buf.last_line());
    let (first, last) = (first.min(last), last.max(first));
    let lines: Vec<String> = (first..=last).map(|l| buf.line(l)).collect();
    let code: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    if code.is_empty() {
        return;
    }
    let commented = code.iter().all(|l| l.trim_start().starts_with(token));
    let min_indent = code.iter().map(|l| l.chars().take_while(|c| *c == ' ' || *c == '\t').count()).min().unwrap_or(0);
    let started = !buf.in_txn();
    if started {
        buf.begin(cur);
    }
    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let new = if commented {
            let lead = line.len() - line.trim_start().len();
            let rest = &line[lead + token.len()..];
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            format!("{}{rest}", &line[..lead])
        } else {
            let (head, tail): (String, String) = (line.chars().take(min_indent).collect(), line.chars().skip(min_indent).collect());
            format!("{head}{token} {tail}")
        };
        replace_lines(buf, first + i, first + i, &[new]);
    }
    if started {
        buf.commit(cur);
    }
    let p = ed.doc().buf.clamp(cur, ed.mode == Mode::Insert);
    ed.doc_mut().view.cursor = p;
}

/// Moves lines `first..=last` one line down or up, with the cursor and selection.
pub fn move_lines(ed: &mut Editor, first: usize, last: usize, down: bool) {
    if !ed.has_doc() {
        return;
    }
    let lastline = ed.doc().buf.last_line();
    let (first, last) = (first.min(last), last.max(first).min(lastline));
    if (down && last >= lastline) || (!down && first == 0) {
        return;
    }
    let dest = if down { last + 2 } else { first - 1 };
    move_range(ed, first, last, dest);
    let shift = |p: Pos| Pos::new(if down { p.line + 1 } else { p.line - 1 }, p.col);
    let view = &mut ed.doc_mut().view;
    view.cursor = shift(view.cursor);
    if let Some(a) = view.anchor {
        view.anchor = Some(shift(a));
    }
}

/// `:m`: moves lines `a..=b` to below line `dest` (1-based, 0 for the top). Returns false when
/// `dest` is inside the range.
pub(crate) fn move_range(ed: &mut Editor, a: usize, b: usize, dest: usize) -> bool {
    if dest == a || dest == b + 1 {
        return true;
    }
    if dest > a && dest <= b {
        return false;
    }
    let cur = ed.cursor();
    let buf = &mut ed.doc_mut().buf;
    let started = !buf.in_txn();
    if started {
        buf.begin(cur);
    }
    let get = |buf: &Buffer, r: std::ops::RangeInclusive<usize>| -> Vec<String> { r.map(|l| buf.line(l)).collect() };
    let moved = get(buf, a..=b);
    if dest > b {
        let hi = (dest - 1).min(buf.last_line());
        let mut lines = get(buf, b + 1..=hi);
        lines.extend(moved);
        replace_lines(buf, a, hi, &lines);
    } else {
        let mut lines = moved;
        lines.extend(get(buf, dest..=a - 1));
        replace_lines(buf, dest, b, &lines);
    }
    if started {
        buf.commit(cur);
    }
    true
}

/// `J` and `gJ`: joins `n` lines (at least two) from `first`. Returns the join point.
pub(crate) fn join_lines(buf: &mut Buffer, first: usize, n: usize, spaces: bool) -> Option<Pos> {
    let mut at = None;
    for _ in 0..n.max(2) - 1 {
        if first >= buf.last_line() {
            break;
        }
        let cur = buf.line(first);
        let next = buf.line(first + 1);
        let len = cur.chars().count();
        let nl = buf.idx(Pos::new(first, len));
        if spaces {
            let lead = next.chars().take_while(|c| *c == ' ' || *c == '\t').count();
            let rest: String = next.chars().skip(lead).collect();
            let sep = if rest.is_empty() || rest.starts_with(')') || cur.is_empty() || cur.ends_with([' ', '\t']) { "" } else { " " };
            buf.replace(nl, nl + 1 + lead, sep);
            let col = if sep.is_empty() && rest.is_empty() { len.saturating_sub(1) } else { len };
            at = Some(Pos::new(first, col));
        } else {
            buf.remove(nl, nl + 1);
            at = Some(Pos::new(first, len.saturating_sub(if next.is_empty() { 1 } else { 0 })));
        }
    }
    at
}
