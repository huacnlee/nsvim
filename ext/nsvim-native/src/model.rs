//! What the screen shows, computed from the core `Editor`: the visible rows with their
//! colours, selection, search matches, diagnostics and gutter signs, the cursors, the status
//! line and the command line. This module has no NSH code. The views read a `Frame` and draw
//! it.
//!
//! `frame` reads the core `Editor` and a `Look`: the options and marks the app keeps beside
//! the core (config.toml's look, git signs and blame, diagnostics, an agent's note). `Look::of` makes
//! one from the app.

use editor::buffer::Pos;
use editor::editor::{Editor, Keymap, Mode, VisualKind};
use editor::syntax::highlight::{self, Kind};
use unicode_width::UnicodeWidthChar;

use crate::app::App;
use crate::services::git::{BlameLine, Sign as GitSign, SignKind};
use crate::services::lsp::Diag;

/// Columns of the gutter before the line numbers: room for a sign.
const SIGN_COLS: usize = 2;

/// Chars of a diagnostic's message shown after its line.
const MESSAGE_CHARS: usize = 160;

/// How bad a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Info,
}

impl Level {
    /// The level of an LSP severity: 1 error, 2 warning, 3 and 4 information.
    pub fn of(severity: u8) -> Level {
        match severity {
            0 | 1 => Level::Error,
            2 => Level::Warning,
            _ => Level::Info,
        }
    }
}

/// One piece of a row with one look.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Seg {
    pub text: String,
    pub kind: Option<Kind>,
    pub selected: bool,
    /// Inside a match of the last search.
    pub matched: bool,
    /// A whitespace mark (`·` or `→`), drawn faint.
    pub ws: bool,
    /// Under a diagnostic.
    pub diag: Option<Level>,
    /// Chars two cells wide (CJK). The view spaces them out to the grid, as a font's own
    /// advance for them is seldom two cells.
    pub wide: bool,
}

/// The mark in a row's gutter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    /// Git: lines added, changed, or removed after this line.
    Added,
    Modified,
    Deleted,
    /// The line changed on disk from outside the editor.
    Outside,
    /// The worst diagnostic on the line.
    Diag(Level),
}

impl Sign {
    /// The glyph the gutter shows.
    pub fn glyph(self) -> &'static str {
        match self {
            Sign::Added | Sign::Modified => "▎",
            Sign::Deleted => "▁",
            Sign::Outside => "┃",
            Sign::Diag(Level::Error) => "E",
            Sign::Diag(Level::Warning) => "W",
            Sign::Diag(Level::Info) => "I",
        }
    }
}

/// Text after the end of a row: a diagnostic's message, an agent's note, or who last changed
/// the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum After {
    Diag(Level, String),
    Note(String),
    Blame(String),
}

/// A row of the text area.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// A line of the buffer, or one piece of a wrapped line.
    Line {
        /// The line number, or empty on a wrapped piece and with numbers off.
        number: String,
        /// The cursor's line: the number shows bold.
        current: bool,
        segs: Vec<Seg>,
        /// Fill the row as the cursor line.
        cursorline: bool,
        sign: Option<Sign>,
        /// The whole line is selected (Visual line mode, or a selection across it).
        full: bool,
        /// The line is inside an agent's note: tint it.
        tint: bool,
        after: Option<After>,
        /// Display columns of the extra cursors on this row.
        carets: Vec<usize>,
    },
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
    /// The char under it, which a block cursor draws over itself.
    pub ch: char,
    /// The keymap is Vim: an Insert bar takes the Insert colour.
    pub vim: bool,
    /// Visual mode: a block takes the Visual colour.
    pub visual: bool,
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
    /// The ruler's display column from the text's left edge.
    pub ruler: Option<usize>,
    pub status: Status,
    pub bottom: Bottom,
    pub empty: bool,
}

/// An agent's note on lines of the active file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoteMark<'a> {
    pub first: usize,
    pub last: usize,
    pub text: &'a str,
}

/// What the text area shows beside the core's state: config.toml's look and the marks of the
/// active file.
#[derive(Clone, Copy, Debug)]
pub struct Look<'a> {
    /// `hybrid`, `relative`, `absolute` or `off`.
    pub numbers: &'a str,
    /// The Normal mode cursor: `block`, `bar` or `underline`.
    pub cursor: &'a str,
    pub cursorline: bool,
    /// The ruler's column, 0 for none.
    pub ruler: usize,
    /// Show leading spaces as `·` and leading tabs as `→`.
    pub whitespace: bool,
    pub signs: &'a [GitSign],
    /// Who last changed each line. Empty when line blame is off.
    pub blame: &'a [BlameLine],
    pub diags: &'a [Diag],
    /// Lines changed on disk from outside.
    pub outside: &'a [usize],
    pub note: Option<NoteMark<'a>>,
    /// Draw no cursor: the tree has the keys.
    pub hide_caret: bool,
}

impl Default for Look<'_> {
    fn default() -> Self {
        Look {
            numbers: "hybrid",
            cursor: "block",
            cursorline: true,
            ruler: 0,
            whitespace: false,
            signs: &[],
            blame: &[],
            diags: &[],
            outside: &[],
            note: None,
            hide_caret: false,
        }
    }
}

impl<'a> Look<'a> {
    /// The look of the app's active file.
    pub fn of(app: &'a App) -> Look<'a> {
        Look::of_doc(app, app.ed.active)
    }

    /// The look of document `ix`.
    pub fn of_doc(app: &'a App, ix: usize) -> Look<'a> {
        let e = &app.cfg.editor;
        let path = app.ed.docs.get(ix).and_then(|d| d.buf.path.as_deref());
        let meta = path.and_then(|p| app.meta.get(p));
        let note = app.note.as_ref().filter(|n| Some(n.path.as_path()) == path).map(|n| NoteMark {
            first: n.first,
            last: n.last,
            text: n.text.as_str(),
        });
        Look {
            numbers: e.numbers.as_str(),
            cursor: e.cursor.as_str(),
            cursorline: e.cursorline,
            ruler: e.ruler as usize,
            whitespace: e.whitespace,
            signs: meta.map_or(&[], |m| m.signs.as_slice()),
            blame: meta.filter(|_| app.blame).map_or(&[], |m| m.blame.lines.as_slice()),
            diags: meta.map_or(&[], |m| m.diags.as_slice()),
            outside: meta.map_or(&[], |m| m.outside.as_slice()),
            note,
            hide_caret: app.tree.focus,
        }
    }
}

/// The text area's frame for the app.
pub fn app_frame(app: &App, rows: usize) -> Frame {
    frame_with(&app.ed, &Look::of(app), rows)
}

/// The frame of a pane that shows document `id` without the keys, `rows` by `cols`.
pub fn pane_frame(app: &App, id: u64, rows: usize, cols: usize) -> Frame {
    let ix = app.ed.docs.iter().position(|d| d.id == id).unwrap_or(usize::MAX);
    frame_of(&app.ed, &Look::of_doc(app, ix), rows, ix, cols, false)
}

/// Gutter width in columns: a sign, the numbers right-aligned, then 3 columns of space. Only
/// the sign with numbers off.
pub fn gutter_cols(ed: &Editor, numbers: &str) -> usize {
    gutter_width(if ed.has_doc() { ed.buf().line_count() } else { 1 }, numbers)
}

/// `gutter_cols` for a document of `lines` lines.
pub fn gutter_width(lines: usize, numbers: &str) -> usize {
    if numbers == "off" {
        return SIGN_COLS;
    }
    SIGN_COLS + lines.to_string().len().max(3) + 3
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

/// Where each screen row of a line starts, as a char index, for a line wrapped at `width`
/// columns. `cols` is the line's `columns`. A row holds at least one char.
pub fn wrap_starts(cols: &[usize], width: usize) -> Vec<usize> {
    let width = width.max(1);
    let mut out = vec![0];
    let mut start = 0;
    for i in 0..cols.len().saturating_sub(1) {
        if cols[i + 1] - cols[start] > width && i > start {
            out.push(i);
            start = i;
        }
    }
    out
}

/// The screen rows of `line`: where each starts, as a char index.
fn row_starts(ed: &Editor, line: usize) -> Vec<usize> {
    let doc = ed.doc();
    if !ed.opts.wrap {
        return vec![doc.view.left];
    }
    wrap_starts(&columns(&doc.buf.line(line), ed.opts.tab_width), ed.cols.max(1))
}

/// The line and the first char shown on screen row `row` of the text area. Rows past the end
/// give the last row of the last line.
pub fn row_at(ed: &Editor, row: usize) -> Option<(usize, usize)> {
    if !ed.has_doc() {
        return None;
    }
    let doc = ed.doc();
    let last = doc.buf.last_line();
    let mut left = row;
    let mut line = doc.view.top.min(last);
    loop {
        let starts = row_starts(ed, line);
        if left < starts.len() || line == last {
            return Some((line, starts[left.min(starts.len() - 1)]));
        }
        left -= starts.len();
        line += 1;
    }
}

/// The buffer position under display column `col` (in cells, with a fraction) of screen row
/// `row`. Clicks past a line's end land on its end, or on its last char outside Insert mode.
/// In Insert mode, a click on the right half of a char goes after it.
pub fn pos_at(ed: &Editor, row: usize, col: f32) -> Option<Pos> {
    let (line, from) = row_at(ed, row)?;
    let doc = ed.doc();
    let insert = ed.mode == Mode::Insert;
    let cols = columns(&doc.buf.line(line), ed.opts.tab_width);
    let len = cols.len() - 1;
    let from = from.min(len);
    let next = if ed.opts.wrap { row_starts(ed, line).into_iter().find(|&s| s > from).unwrap_or(len) } else { len };
    let x = cols[from] as f32 + col.max(0.0);
    let mut at = (from..next).find(|&i| x < cols[i + 1] as f32).unwrap_or(next);
    if insert && at < next && 2.0 * x > (cols[at] + cols[at + 1]) as f32 {
        at += 1;
    }
    Some(doc.buf.clamp(Pos::new(line, at), insert))
}

/// The frame for `rows` rows of text, with the default look.
pub fn frame(ed: &Editor, rows: usize) -> Frame {
    frame_with(ed, &Look::default(), rows)
}

/// The look of each char of a line: everything but its text.
#[derive(Clone, Copy, Default, PartialEq)]
struct Ink {
    kind: Option<Kind>,
    selected: bool,
    matched: bool,
    diag: Option<Level>,
}

/// The frame for `rows` rows of text with look `look`.
pub fn frame_with(ed: &Editor, look: &Look, rows: usize) -> Frame {
    frame_of(ed, look, rows, ed.active, ed.cols, true)
}

/// The frame of document `ix` in a pane of `rows` rows and `cols` columns. Only the `live` pane,
/// the one that has the keys, draws the cursor and the selection.
pub fn frame_of(ed: &Editor, look: &Look, rows: usize, ix: usize, cols: usize, live: bool) -> Frame {
    let status = status(ed);
    let bottom = bottom(ed);
    let Some(doc) = ed.docs.get(ix) else {
        let gutter = gutter_cols(ed, look.numbers);
        return Frame { rows: Vec::new(), caret: None, gutter, ruler: None, status, bottom, empty: true };
    };
    let buf = &doc.buf;
    let gutter = gutter_width(buf.line_count(), look.numbers);
    let path = buf.path.as_deref();
    let top = doc.view.top.min(buf.last_line());
    let wrap = ed.opts.wrap;
    let left = if wrap { 0 } else { doc.view.left };
    let width = cols.max(4);
    let cur = doc.view.cursor;
    let count = rows.min(buf.line_count().saturating_sub(top));
    let runs = highlight::runs(buf, path, top, count);
    let tab = ed.opts.tab_width;
    let vim = ed.keymap == Keymap::Vim;
    let numbers = match (vim, look.numbers) {
        (true, n) => n,
        (false, "off") => "off",
        (false, _) => "absolute",
    };
    let search = if ed.search.pattern.is_empty() || !ed.vim.hlsearch {
        Vec::new()
    } else {
        editor::vim::search_matches(buf, &ed.search.pattern, ed.case(), top, top + count)
    };
    let show_caret = live && ed.mode != Mode::Command && !look.hide_caret;
    let shape = shape(ed, look.cursor);
    let mut out = Vec::with_capacity(rows);
    let mut caret = None;
    for (r, line_runs) in runs.iter().enumerate() {
        if out.len() >= rows {
            break;
        }
        let line = top + r;
        let text = buf.line(line);
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let cols = columns(&text, tab);
        let mut ink = vec![Ink::default(); len];
        for (range, k) in line_runs {
            for slot in ink.iter_mut().take(range.end.min(len)).skip(range.start) {
                slot.kind = Some(*k);
            }
        }
        let sel = if live { selected(ed, line, len) } else { None };
        let mut paint = |a: usize, b: usize, f: &dyn Fn(&mut Ink)| ink.iter_mut().take(b.min(len)).skip(a).for_each(f);
        if let Some((a, b, _)) = sel {
            paint(a, b, &|i| i.selected = true);
        }
        for c in doc.extra.iter().filter(|_| live) {
            if let Some((a, b)) = extra_range(c, line, len) {
                paint(a, b, &|i| i.selected = true);
            }
        }
        for &(a, b) in &search {
            let (pa, pb) = (buf.pos(a), buf.pos(b));
            if pa.line == line {
                paint(pa.col, if pb.line == line { pb.col } else { len }, &|i| i.matched = true);
            }
        }
        // Worst last, so it wins.
        let mut diags: Vec<&Diag> = look.diags.iter().filter(|d| d.line <= line && d.end_line >= line).collect();
        diags.sort_by_key(|d| std::cmp::Reverse(d.severity));
        for d in &diags {
            let a = if d.line == line { d.col } else { 0 };
            let b = if d.end_line == line { d.end_col.max(a + 1) } else { len };
            let level = Level::of(d.severity);
            paint(a, b, &|i| i.diag = Some(level));
        }
        let lead = if look.whitespace { chars.iter().take_while(|c| **c == ' ' || **c == '\t').count() } else { 0 };
        let full = sel.is_some_and(|(a, b, past)| a == 0 && b >= len && past);
        let starts = if wrap { wrap_starts(&cols, width) } else { vec![left] };
        let worst = diags.last().filter(|d| d.line == line).copied();
        let current = line == cur.line;
        for (pi, &from) in starts.iter().enumerate() {
            if out.len() >= rows {
                break;
            }
            let last_piece = pi + 1 == starts.len();
            let from = from.min(len);
            let to = if wrap {
                starts.get(pi + 1).copied().unwrap_or(len)
            } else {
                // Only the columns that fit, and one more.
                (from..len).find(|&i| cols[i] > cols[from] + width + 1).unwrap_or(len)
            };
            let mut segs: Vec<Seg> = Vec::new();
            for i in from..to {
                let ch = chars[i];
                let ws = i < lead;
                let piece = match ch {
                    '\t' if ws => format!("→{}", " ".repeat(cols[i + 1] - cols[i] - 1)),
                    '\t' => " ".repeat(cols[i + 1] - cols[i]),
                    ' ' if ws => "·".into(),
                    ch => ch.to_string(),
                };
                let k = ink[i];
                let wide = ch != '\t' && cols[i + 1] - cols[i] == 2;
                match segs.last_mut() {
                    Some(s)
                        if s.kind == k.kind
                            && s.selected == k.selected
                            && s.matched == k.matched
                            && s.ws == ws
                            && s.diag == k.diag
                            && s.wide == wide =>
                    {
                        s.text.push_str(&piece)
                    }
                    _ => segs.push(Seg { text: piece, kind: k.kind, selected: k.selected, matched: k.matched, ws, diag: k.diag, wide }),
                }
            }
            // A selection past the line end shows one selected space, as Vim does.
            if last_piece && !full && sel.is_some_and(|(_, _, past)| past) {
                segs.push(Seg { text: " ".into(), selected: true, ..Seg::default() });
            }
            let number = match numbers {
                _ if pi > 0 => String::new(),
                "off" => String::new(),
                "relative" => line.abs_diff(cur.line).to_string(),
                "absolute" => (line + 1).to_string(),
                _ if current => (line + 1).to_string(),
                _ => line.abs_diff(cur.line).to_string(),
            };
            let in_piece = |col: usize| col >= from && (col < to || last_piece);
            if current && show_caret && in_piece(cur.col) {
                let col = cur.col.min(len);
                let ch = chars.get(col).copied().filter(|c| *c != '\t').unwrap_or(' ');
                caret = Some(Caret {
                    row: out.len(),
                    col: cols[col] - cols[from],
                    width: ch.width().unwrap_or(1).max(1),
                    shape,
                    ch,
                    vim,
                    visual: matches!(ed.mode, Mode::Visual(_)),
                });
            }
            let carets = if show_caret {
                doc.extra
                    .iter()
                    .filter(|c| c.pos.line == line && in_piece(c.pos.col))
                    .map(|c| cols[c.pos.col.min(len)] - cols[from])
                    .collect()
            } else {
                Vec::new()
            };
            let noted = look.note.filter(|n| line >= n.first && line <= n.last);
            let sign = if pi > 0 {
                None
            } else if let Some(d) = worst {
                Some(Sign::Diag(Level::of(d.severity)))
            } else if look.outside.binary_search(&line).is_ok() {
                Some(Sign::Outside)
            } else {
                look.signs.iter().find(|s| s.line == line).map(|s| match s.kind {
                    SignKind::Added => Sign::Added,
                    SignKind::Modified => Sign::Modified,
                    SignKind::Deleted => Sign::Deleted,
                })
            };
            let after = match (noted, worst) {
                (Some(n), _) if pi == 0 && n.first == line && !n.text.is_empty() => Some(After::Note(n.text.to_string())),
                (_, Some(d)) if last_piece && d.severity <= 2 => {
                    Some(After::Diag(Level::of(d.severity), d.message.lines().next().unwrap_or("").chars().take(MESSAGE_CHARS).collect()))
                }
                // The cursor's line also shows the commit's summary.
                _ if last_piece => look.blame.get(line).filter(|b| !b.who.is_empty()).map(|b| match current && !b.summary.is_empty() {
                    true => After::Blame(format!("{} · {}", b.who, b.summary).chars().take(MESSAGE_CHARS).collect()),
                    false => After::Blame(b.who.clone()),
                }),
                _ => None,
            };
            out.push(Row::Line {
                number,
                current: current && pi == 0,
                segs,
                cursorline: current && pi == 0 && look.cursorline && sel.is_none() && show_caret,
                sign,
                full,
                tint: noted.is_some() && !full,
                after,
                carets,
            });
        }
    }
    out.resize(rows, Row::Past);
    let ruler = (look.ruler > left && !wrap).then(|| look.ruler - left);
    Frame { rows: out, caret, gutter, ruler, status, bottom, empty: false }
}

/// The chars an extra cursor's selection covers on `line`, as `[a, b)`.
fn extra_range(c: &editor::editor::Cursor, line: usize, len: usize) -> Option<(usize, usize)> {
    let anchor = c.anchor?;
    let (s, e) = if anchor <= c.pos { (anchor, c.pos) } else { (c.pos, anchor) };
    if line < s.line || line > e.line {
        return None;
    }
    let a = if line == s.line { s.col } else { 0 };
    let b = if line == e.line { e.col } else { len };
    (b > a).then_some((a, b))
}

fn shape(ed: &Editor, normal: &str) -> Shape {
    let vim = ed.keymap == Keymap::Vim;
    if vim && editor::vim::replacing(&ed.vim) {
        Shape::Underline
    } else if ed.mode == Mode::Insert || !vim {
        Shape::Bar
    } else if matches!(ed.mode, Mode::Visual(_)) {
        Shape::Block
    } else {
        match normal {
            "bar" => Shape::Bar,
            "underline" => Shape::Underline,
            _ => Shape::Block,
        }
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
