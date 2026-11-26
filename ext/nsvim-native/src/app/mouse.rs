//! The mouse in the text area: click to place the cursor, drag to select (Visual mode in the
//! Neovim keymap), double-click for a word, triple-click or a click on a line number for a
//! line, and the wheel to scroll. With `mouse.enabled` off, only the wheel works.

use std::time::Duration;

use super::fx::{Drag, Phase, Tx, WheelEvent};
use editor::buffer::Pos;
use editor::editor::{KeyCode, KeyInput, Keymap, Mode, VisualKind};

use super::App;

/// How often a drag past the top or bottom edge scrolls.
const EDGE_STEP: Duration = Duration::from_millis(50);
/// The most lines one step of that scroll moves.
const EDGE_MAX_LINES: f32 = 8.0;

/// A mouse drag over the text.
#[derive(Default)]
pub struct TextDrag {
    /// Where the drag started, while the button is down.
    pub from: Option<Pos>,
    /// Where the pointer is, in the text area's points. A wheel turn or an edge scroll selects
    /// to the text that comes under it.
    pub at: (f32, f32),
    /// The drag selects whole lines (a triple-click or a press on the line numbers).
    pub lines: bool,
    /// The half cell under the pointer, from [`App::drag_cell`].
    pub cell: (i32, i32),
    /// When the next step of a scroll past an edge is due, on the app's clock.
    pub scroll_at: Option<f64>,
}

impl App {
    /// The text area's rect, from layout. Sets the rows and columns the keymaps scroll by.
    pub fn text_rect(&mut self, r: [f32; 4]) {
        self.m.text = r;
        self.m.row_h = crate::geometry::row_height(self);
        self.update_grid();
    }

    /// The editor font's advance, from the probe.
    pub fn char_width(&mut self, w: f32) {
        if w > 0.5 && (w - self.m.cw).abs() > 0.001 {
            self.m.cw = w;
            self.update_grid();
        }
    }

    fn update_grid(&mut self) {
        let gutter = crate::geometry::gutter_cols(self) as f32 * self.m.cw;
        self.m.gutter = gutter;
        self.ed.rows = ((self.m.text[3] / self.m.row_h).floor() as usize).max(1);
        self.ed.cols = (((self.m.text[2] - gutter) / self.m.cw).floor() as usize).max(4);
        if self.ed.has_doc() {
            self.ed.scroll_to_cursor();
        }
    }

    /// The buffer position under a point in the text area's own coordinates. Clicks past a
    /// line's end land on its last char (or after it in Insert mode). Wrapped lines take
    /// several rows.
    pub fn pos_at(&self, x: f32, y: f32) -> Option<Pos> {
        let row = (y / self.m.row_h).floor().max(0.0) as usize;
        crate::model::pos_at(&self.ed, row, (x - self.m.gutter).max(0.0) / self.m.cw)
    }

    /// The row and half column under a point of a drag. Moves in one half cell select the same
    /// text. Past an edge, the row also sets how fast the text scrolls.
    pub fn drag_cell(&self, x: f32, y: f32) -> (i32, i32) {
        ((y / self.m.row_h).floor() as i32, (2.0 * (x - self.m.gutter) / self.m.cw).floor() as i32)
    }

    /// True when a move of the drag to `(x, y)` changes nothing: it is in the same half cell.
    pub fn drag_holds(&self, x: f32, y: f32) -> bool {
        self.drag.from.is_some() && self.drag_cell(x, y) == self.drag.cell
    }

    pub fn text_drag(&mut self, d: &mut Drag, tx: &mut Tx) {
        self.drag.cell = self.drag_cell(d.x, d.y);
        if !self.cfg.mouse.enabled || !self.ed.has_doc() || self.on_settings {
            return;
        }
        self.tree.focus = false;
        tx.instant();
        let vim = self.ed.keymap == Keymap::Vim;
        let in_gutter = d.start_x < self.m.gutter;
        match d.phase {
            Phase::Down => {
                let Some(pos) = self.pos_at(d.x, d.y) else { return };
                self.drag = TextDrag { at: (d.x, d.y), cell: self.drag.cell, ..TextDrag::default() };
                if self.ed.mode == Mode::Command {
                    return;
                }
                if vim && self.ed.mode == Mode::Insert && (d.clicks > 1 || in_gutter) {
                    editor::vim::handle_key(&mut self.ed, KeyInput::plain(KeyCode::Esc));
                }
                let shift = tx.mods().shift;
                // Classic keymap: Alt-click adds a cursor, or removes the one under the pointer.
                if !vim && tx.mods().alt && self.cfg.keys.multi_cursor && d.clicks == 1 {
                    let main = self.ed.cursor();
                    let doc = self.ed.doc_mut();
                    if let Some(i) = doc.extra.iter().position(|c| c.pos == pos) {
                        doc.extra.remove(i);
                    } else if pos != main {
                        doc.extra.push(editor::editor::Cursor { pos, anchor: None });
                    }
                    d.data = 0.0;
                    return;
                }
                if in_gutter || d.clicks >= 3 {
                    self.select_lines(pos.line, pos.line);
                    self.drag.from = Some(Pos::new(pos.line, 0));
                    self.drag.lines = true;
                    d.data = 3.0;
                } else if d.clicks == 2 && self.cfg.mouse.double_click {
                    let (a, b) = editor::vim::word_range_at(&self.ed.doc().buf, pos);
                    let (pa, pb) = (self.ed.buf().pos(a), self.ed.buf().pos(b));
                    self.select_range(pa, pb);
                    d.data = 2.0;
                } else if shift {
                    let from = self.ed.doc().view.anchor.unwrap_or(self.ed.cursor());
                    self.select_range_inclusive(from, pos);
                } else if self.cfg.mouse.click {
                    if vim && matches!(self.ed.mode, Mode::Visual(_)) {
                        self.ed.mode = Mode::Normal;
                    }
                    self.ed.doc_mut().extra.clear();
                    self.ed.doc_mut().view.anchor = None;
                    self.ed.set_cursor(pos);
                    self.drag.from = Some(self.ed.cursor());
                    d.data = 1.0;
                }
            }
            Phase::Move => {
                if !self.cfg.mouse.drag {
                    return;
                }
                if self.drag.from.is_none() {
                    return;
                }
                self.drag.at = (d.x, d.y);
                self.drag_select();
                // Past the top or bottom edge, a timer scrolls also while the pointer holds still.
                if self.drag.scroll_at.is_none() {
                    self.drag_scroll(self.clock, tx);
                }
                tx.instant();
            }
            Phase::Up => self.drag = TextDrag { cell: self.drag.cell, ..TextDrag::default() },
        }
    }

    /// One step of the scroll of a drag past the top or bottom edge, when it is time. The
    /// further past the edge, the more lines a step scrolls. Only the view moves: the selection
    /// then goes to the text under the pointer. Returns true when it scrolled.
    pub fn drag_scroll(&mut self, clock: f64, tx: &mut Tx) -> bool {
        let y = self.drag.at.1;
        let past = if y < 0.0 { y } else { (y - self.m.text[3]).max(0.0) };
        if self.drag.from.is_none() || past == 0.0 {
            self.drag.scroll_at = None;
            return false;
        }
        let at = self.drag.scroll_at.unwrap_or(clock);
        if clock < at {
            tx.wake_after(Duration::from_secs_f64(at - clock));
            return false;
        }
        let lines = (1.0 + (past.abs() / self.m.row_h).floor()).min(EDGE_MAX_LINES) as isize;
        self.ed.scroll_view(if past < 0.0 { -lines } else { lines });
        self.drag_select();
        self.drag.scroll_at = Some(clock + EDGE_STEP.as_secs_f64());
        tx.wake_after(EDGE_STEP);
        true
    }

    /// Selects from the drag's start to the text under its pointer.
    fn drag_select(&mut self) {
        let Some(from) = self.drag.from else { return };
        // A pointer past an edge selects in the full row at that edge.
        let (x, y) = self.drag.at;
        let y = y.clamp(0.0, (self.ed.rows as f32 * self.m.row_h - 1.0).max(0.0));
        let Some(pos) = self.pos_at(x, y) else { return };
        // The pointer picks the line, so the text under it must hold still: scrolloff
        // would scroll at every move near an edge and run the selection away.
        let top = self.ed.doc().view.top;
        if self.drag.lines {
            self.select_lines(from.line, pos.line);
        } else if pos != from {
            if self.ed.keymap == Keymap::Vim && self.ed.mode == Mode::Insert {
                editor::vim::handle_key(&mut self.ed, KeyInput::plain(KeyCode::Esc));
            }
            self.select_range_inclusive(from, pos);
        }
        self.ed.doc_mut().view.top = top;
    }

    /// Selects from `a` to `b`, both included (Visual), or `a` to after `b` (classic).
    fn select_range_inclusive(&mut self, a: Pos, b: Pos) {
        match self.ed.keymap {
            Keymap::Vim => {
                self.ed.mode = Mode::Visual(VisualKind::Char);
                self.ed.doc_mut().view.anchor = Some(a);
                self.ed.set_cursor(b);
            }
            Keymap::Classic => {
                self.ed.doc_mut().view.anchor = Some(a);
                self.ed.set_cursor(b);
            }
        }
    }

    /// Selects `[a, b)`: a word from a double-click.
    fn select_range(&mut self, a: Pos, b: Pos) {
        match self.ed.keymap {
            Keymap::Vim => {
                if b == a {
                    return;
                }
                self.ed.mode = Mode::Visual(VisualKind::Char);
                self.ed.doc_mut().view.anchor = Some(a);
                let end = self.ed.buf().pos(self.ed.buf().idx(b).saturating_sub(1));
                self.ed.set_cursor(end);
            }
            Keymap::Classic => {
                self.ed.doc_mut().view.anchor = Some(a);
                self.ed.set_cursor(b);
            }
        }
        self.drag.from = Some(a);
    }

    fn select_lines(&mut self, a: usize, b: usize) {
        match self.ed.keymap {
            Keymap::Vim => {
                self.ed.mode = Mode::Visual(VisualKind::Line);
                self.ed.doc_mut().view.anchor = Some(Pos::new(a, 0));
                self.ed.set_cursor(Pos::new(b, 0));
            }
            Keymap::Classic => {
                let (lo, hi) = (a.min(b), a.max(b));
                self.ed.doc_mut().view.anchor = Some(Pos::new(lo, 0));
                let last = self.ed.buf().last_line();
                let end = if hi < last { Pos::new(hi + 1, 0) } else { Pos::new(hi, self.ed.buf().line_len(hi)) };
                self.ed.set_cursor(end);
            }
        }
    }

    pub fn text_wheel(&mut self, e: WheelEvent, tx: &mut Tx) -> bool {
        if !self.cfg.mouse.scroll || !self.ed.has_doc() {
            return false;
        }
        if e.mods.shift || e.dx.abs() > e.dy.abs() {
            let d = if e.mods.shift { e.dy } else { e.dx };
            let cols = (-d / self.m.cw).round() as isize;
            if cols != 0 && !self.ed.opts.wrap {
                let doc = self.ed.doc_mut();
                doc.view.left = (doc.view.left as isize + cols).max(0) as usize;
            }
            return true;
        }
        self.wheel_rest += -e.dy / self.m.row_h;
        let lines = self.wheel_rest.trunc();
        self.wheel_rest -= lines;
        if lines != 0.0 {
            // A selection, a drag or the classic keymap keeps the cursor where it is, so the
            // wheel does not change what is selected. Vim's Normal mode keeps it on screen.
            let keep = self.drag.from.is_some() || self.ed.keymap == Keymap::Classic || matches!(self.ed.mode, Mode::Visual(_));
            if keep {
                self.ed.scroll_view(lines as isize);
                self.drag_select();
            } else {
                self.ed.scroll_by(lines as isize);
            }
            tx.instant();
        }
        true
    }
}
