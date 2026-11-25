//! The mouse in the text area: click to place the cursor, drag to select (Visual mode in the
//! Neovim keymap), double-click for a word, triple-click or a click on a line number for a
//! line, and the wheel to scroll. With `mouse.enabled` off, only the wheel works.

use super::fx::{Drag, Phase, Tx, WheelEvent};
use editor::buffer::Pos;
use editor::editor::{KeyCode, KeyInput, Keymap, Mode, VisualKind};

use super::App;

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
    /// line's end land on its last char (or after it in Insert mode).
    pub fn pos_at(&self, x: f32, y: f32) -> Option<Pos> {
        if !self.ed.has_doc() {
            return None;
        }
        let doc = self.ed.doc();
        let row = (y / self.m.row_h).floor().max(0.0) as usize;
        let line = (doc.view.top + row).min(doc.buf.last_line());
        let text_x = (x - self.m.gutter).max(0.0);
        let left = if self.cfg.editor.wrap { 0 } else { doc.view.left };
        // Walk the line's display columns, so tabs and wide chars land right.
        let full = doc.buf.line(line);
        let tab = self.ed.opts.tab_width.max(1);
        let mut c = 0usize;
        let mut col = 0usize;
        let insert = self.ed.mode == Mode::Insert;
        for (i, ch) in full.chars().enumerate().skip(left) {
            let w = if ch == '\t' { tab - c % tab } else { unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1).max(1) };
            let x0 = c as f32 * self.m.cw;
            let x1 = (c + w) as f32 * self.m.cw;
            col = i;
            if text_x < x1 {
                if insert && text_x > (x0 + x1) / 2.0 {
                    col = i + 1;
                }
                return Some(doc.buf.clamp(Pos::new(line, col), insert));
            }
            c += w;
            col = i + 1;
        }
        Some(doc.buf.clamp(Pos::new(line, col.max(left)), insert))
    }

    pub fn text_drag(&mut self, d: &mut Drag, tx: &mut Tx) {
        if !self.cfg.mouse.enabled || !self.ed.has_doc() || self.on_settings {
            return;
        }
        self.tree.focus = false;
        tx.instant();
        let Some(pos) = self.pos_at(d.x, d.y) else { return };
        let vim = self.ed.keymap == Keymap::Vim;
        let in_gutter = d.start_x < self.m.gutter;
        match d.phase {
            Phase::Down => {
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
                    self.drag_from = Some(Pos::new(pos.line, 0));
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
                    self.drag_from = Some(self.ed.cursor());
                    d.data = 1.0;
                }
            }
            Phase::Move => {
                if !self.cfg.mouse.drag {
                    return;
                }
                let Some(from) = self.drag_from else { return };
                // The pointer picks the line, so the text under it must hold still: scrolloff
                // would scroll at every move near an edge and run the selection away.
                let top = self.ed.doc().view.top;
                if d.data >= 3.0 {
                    self.select_lines(from.line, pos.line);
                } else if pos != from {
                    if vim && self.ed.mode == Mode::Insert {
                        editor::vim::handle_key(&mut self.ed, KeyInput::plain(KeyCode::Esc));
                    }
                    self.select_range_inclusive(from, pos);
                }
                self.ed.doc_mut().view.top = top;
                // Dragging past the top or bottom edge scrolls.
                if d.y < 0.0 {
                    self.ed.scroll_by(-1);
                } else if d.y > self.m.text[3] {
                    self.ed.scroll_by(1);
                }
                tx.instant();
            }
            Phase::Up => self.drag_from = None,
        }
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
        self.drag_from = Some(a);
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
            if cols != 0 && !self.cfg.editor.wrap {
                let doc = self.ed.doc_mut();
                doc.view.left = (doc.view.left as isize + cols).max(0) as usize;
            }
            return true;
        }
        self.wheel_rest += -e.dy / self.m.row_h;
        let lines = self.wheel_rest.trunc();
        self.wheel_rest -= lines;
        if lines != 0.0 {
            self.ed.scroll_by(lines as isize);
            tx.instant();
        }
        true
    }
}
