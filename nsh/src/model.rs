//! What the screen shows, computed from the core `Editor`: the visible lines with their
//! colours and selection, the cursor, the status line and the command line. This module has
//! no NSH code. The views read a `Frame` and draw it.

use editor::editor::{Editor, Keymap, Mode, VisualKind};
use editor::syntax::highlight::{self, Kind};
use unicode_width::UnicodeWidthChar;

/// Columns of the gutter before the line numbers: room for a sign.
const SIGN_COLS: usize = 2;

/// One piece of a line with one look.
#[derive(Clone, Debug, PartialEq)]
pub struct Seg {
    pub text: String,
    pub kind: Option<Kind>,
    pub selected: bool,
}

/// A row of the text area.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// A line of the buffer.
    Line { number: String, current: bool, segs: Vec<Seg>, cursorline: bool },
    /// A row past the last line, drawn as `~`.
    Past,
}

/// How the cursor is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Block,
    Bar,
    Underline,
}

/// Where the cursor is, in rows and display columns from the text's left edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caret {
    pub row: usize,
    pub col: usize,
    /// Columns the char under it takes: 2 for a wide char.
    pub width: usize,
    pub shape: Shape,
}

/// The mode pill's look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Normal,
    Insert,
    Visual,
    Command,
    Replace,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub mode: &'static str,
    pub tone: Tone,
    pub file: String,
    pub dirty: bool,
    pub lang: &'static str,
    pub pos: String,
}

/// The bottom line: the command being typed, or the last message.
#[derive(Clone, Debug, PartialEq)]
pub enum Bottom {
    Empty,
    Command { text: String, caret: usize },
    Message(String),
    Error(String),
}

/// Everything the editor screen shows.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub rows: Vec<Row>,
    pub caret: Option<Caret>,
    pub gutter: usize,
    pub status: Status,
    pub bottom: Bottom,
    pub empty: bool,
}

/// Gutter width in columns: a sign, the numbers right-aligned, then 2 columns of space.
pub fn gutter_cols(ed: &Editor) -> usize {
    let lines = if ed.has_doc() { ed.buf().line_count() } else { 1 };
    SIGN_COLS + lines.to_string().len().max(3) + 2
}

/// The display column of each char of `line`, and one past the end. Tabs go to the next
/// multiple of `tab`. Wide chars take 2 columns.
pub fn columns(line: &str, tab: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(line.len() + 1);
    let mut c = 0;
    for ch in line.chars() {
        out.push(c);
        c += match ch {
            '\t' => tab.max(1) - c % tab.max(1),
            ch => ch.width().unwrap_or(1).max(1),
        };
    }
    out.push(c);
    out
}

/// The char of `line` at display column `col`, for a click. Past the end gives the length.
pub fn char_at_column(line: &str, tab: usize, col: usize) -> usize {
    let cols = columns(line, tab);
    cols.windows(2).position(|w| col < w[1]).unwrap_or(cols.len() - 1)
}

/// The frame for `rows` rows of text.
pub fn frame(ed: &Editor, rows: usize) -> Frame {
    let status = status(ed);
    let bottom = bottom(ed);
    let gutter = gutter_cols(ed);
    if !ed.has_doc() {
        return Frame { rows: Vec::new(), caret: None, gutter, status, bottom, empty: true };
    }
    let doc = ed.doc();
    let buf = &doc.buf;
    let path = buf.path.as_deref();
    let top = doc.view.top.min(buf.last_line());
    let left = if ed.opts.wrap { 0 } else { doc.view.left };
    let cur = doc.view.cursor;
    let count = rows.min(buf.line_count().saturating_sub(top));
    let runs = highlight::runs(buf, path, top, count);
    let tab = ed.opts.tab_width;
    let vim = ed.keymap == Keymap::Vim;
    let mut out = Vec::with_capacity(rows);
    let mut caret = None;
    for (r, line_runs) in runs.iter().enumerate() {
        let line = top + r;
        let text = buf.line(line);
        let chars: Vec<char> = text.chars().collect();
        let cols = columns(&text, tab);
        let from = left.min(chars.len());
        let sel = selected(ed, line, chars.len());
        // The look of each char from `from` on: its kind and whether it is selected.
        let mut kinds: Vec<Option<Kind>> = vec![None; chars.len()];
        for (range, k) in line_runs {
            for slot in kinds.iter_mut().take(range.end.min(chars.len())).skip(range.start) {
                *slot = Some(*k);
            }
        }
        let mut segs: Vec<Seg> = Vec::new();
        for (i, &ch) in chars.iter().enumerate().skip(from) {
            let on = sel.is_some_and(|(a, b, _)| i >= a && i < b);
            let piece = if ch == '\t' { " ".repeat(cols[i + 1] - cols[i]) } else { ch.to_string() };
            match segs.last_mut() {
                Some(s) if s.kind == kinds[i] && s.selected == on => s.text.push_str(&piece),
                _ => segs.push(Seg { text: piece, kind: kinds[i], selected: on }),
            }
        }
        // A selection past the line end shows one selected space, as Vim does.
        if sel.is_some_and(|(_, _, past)| past) {
            segs.push(Seg { text: " ".into(), kind: None, selected: true });
        }
        let current = line == cur.line;
        let number = if vim && !current { line.abs_diff(cur.line).to_string() } else { (line + 1).to_string() };
        if current {
            let col = cur.col.min(chars.len());
            let width = chars.get(col).map_or(1, |c| c.width().unwrap_or(1).max(1));
            let shown = cols[col].saturating_sub(cols[from]);
            caret = Some(Caret { row: r, col: shown, width, shape: shape(ed) });
        }
        out.push(Row::Line { number, current, segs, cursorline: current && sel.is_none() });
    }
    out.resize(rows, Row::Past);
    if ed.mode == Mode::Command {
        caret = None;
    }
    Frame { rows: out, caret, gutter, status, bottom, empty: false }
}

fn shape(ed: &Editor) -> Shape {
    let vim = ed.keymap == Keymap::Vim;
    if vim && editor::vim::replacing(&ed.vim) {
        Shape::Underline
    } else if ed.mode == Mode::Insert || !vim {
        Shape::Bar
    } else {
        Shape::Block
    }
}

/// The selected chars of `line` as `[a, b)`, and whether the selection runs past its end.
fn selected(ed: &Editor, line: usize, len: usize) -> Option<(usize, usize, bool)> {
    let doc = ed.doc();
    let anchor = doc.view.anchor?;
    let cur = doc.view.cursor;
    let (s, e) = if anchor <= cur { (anchor, cur) } else { (cur, anchor) };
    if line < s.line || line > e.line {
        return None;
    }
    match (ed.keymap, ed.mode) {
        (Keymap::Vim, Mode::Visual(VisualKind::Line)) => Some((0, len, true)),
        (Keymap::Vim, Mode::Visual(VisualKind::Block)) => {
            let (c0, c1) = (anchor.col.min(cur.col), anchor.col.max(cur.col));
            Some((c0.min(len), (c1 + 1).min(len), false))
        }
        (Keymap::Vim, Mode::Visual(VisualKind::Char)) => {
            let a = if line == s.line { s.col } else { 0 };
            let (b, past) = if line == e.line { ((e.col + 1).min(len), e.col >= len) } else { (len, true) };
            Some((a.min(len), b, past))
        }
        (Keymap::Classic, _) => {
            let a = if line == s.line { s.col } else { 0 };
            let (b, past) = if line == e.line { (e.col.min(len), false) } else { (len, true) };
            (a < b || past).then_some((a.min(len), b, past))
        }
        _ => None,
    }
}

fn status(ed: &Editor) -> Status {
    let vim = ed.keymap == Keymap::Vim;
    let replacing = vim && editor::vim::replacing(&ed.vim);
    let (mode, tone) = match ed.mode {
        _ if replacing => ("REPLACE", Tone::Replace),
        Mode::Normal => (ed.mode.label(), Tone::Normal),
        Mode::Insert => (ed.mode.label(), Tone::Insert),
        Mode::Visual(_) => (ed.mode.label(), Tone::Visual),
        Mode::Command => (ed.mode.label(), Tone::Command),
    };
    if !ed.has_doc() {
        return Status { mode, tone, file: String::new(), dirty: false, lang: "", pos: String::new() };
    }
    let doc = ed.doc();
    let path = doc.buf.path.as_deref();
    let file = path.map(|p| p.strip_prefix(&ed.root).unwrap_or(p).to_string_lossy().into_owned()).unwrap_or_else(|| "[No Name]".into());
    let pos = format!("{}:{}", doc.view.cursor.line + 1, doc.view.cursor.col + 1);
    Status { mode, tone, file, dirty: doc.buf.is_dirty(), lang: editor::syntax::language_name(path), pos }
}

fn bottom(ed: &Editor) -> Bottom {
    if let (Mode::Command, Some(c)) = (ed.mode, &ed.cmd) {
        return Bottom::Command { text: format!("{}{}", c.kind, c.text), caret: c.caret + 1 };
    }
    match &ed.msg {
        Some(m) if m.error => Bottom::Error(m.text.clone()),
        Some(m) => Bottom::Message(m.text.clone()),
        None => Bottom::Empty,
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
