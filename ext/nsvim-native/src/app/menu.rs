//! The context menu on a right-click in the text area (`mouse.right_click`). macOS and
//! Windows show the system's menu; Linux shows one the app draws, with the same items.

use super::fx::{PointerEvent, PopupItem, PopupMenu, Tx};
use editor::buffer::Pos;
use editor::editor::{Action, KeyCode, KeyInput, Keymap, Mode, VisualKind};

use super::{App, Overlay};

/// The menu's items: id, title, and the key hint for each keymap (Neovim, classic).
pub const ITEMS: &[(&str, &str, &str, &str)] = &[
    ("cut", "Cut", "d", "Ctrl X"),
    ("copy", "Copy", "y", "Ctrl C"),
    ("paste", "Paste", "P", "Ctrl V"),
    ("all", "Select all", "ggVG", "Ctrl A"),
    ("-", "", "", ""),
    ("definition", "Go to definition", "gd", "F12"),
    ("references", "Find references…", "SPC l r", ""),
    ("rename", "Rename symbol…", "SPC l n", "F2"),
    ("actions", "Code actions…", "SPC l a", "Ctrl ."),
    ("-", "", "", ""),
    ("grep", "Find in project…", "SPC /", "Ctrl ⇧ F"),
];

/// The in-app menu while it shows: where, and the highlighted row.
#[derive(Clone, Debug, PartialEq)]
pub struct Menu {
    pub x: f32,
    pub y: f32,
    pub sel: usize,
}

impl App {
    /// A right-click in the text area: move the cursor there unless the click is inside the
    /// selection, then open the menu.
    pub fn context_menu(&mut self, e: PointerEvent, tx: &mut Tx) {
        if !self.cfg.mouse.enabled || !self.cfg.mouse.right_click || !self.ed.has_doc() || self.on_settings {
            return;
        }
        tx.instant();
        if let Some(pos) = self.pos_at(e.x, e.y) {
            let inside = self.selection_chars().is_some_and(|(a, b)| {
                let i = self.ed.buf().idx(pos);
                i >= a && i < b
            });
            if !inside {
                if self.ed.keymap == Keymap::Vim && self.ed.mode != Mode::Normal {
                    editor::vim::handle_key(&mut self.ed, KeyInput::plain(KeyCode::Esc));
                }
                self.ed.doc_mut().view.anchor = None;
                self.ed.set_cursor(pos);
            }
        }
        let has_sel = self.selection_chars().is_some();
        if super::fx::NATIVE_MENU {
            let vim = self.is_vim();
            let mut menu = PopupMenu::new();
            for &(id, title, vk, ck) in ITEMS {
                if id == "-" {
                    menu = menu.separator();
                    continue;
                }
                let hint = if vim { vk } else { ck };
                let mut item = PopupItem::new(id, format!("{title}    {hint}"));
                if matches!(id, "cut" | "copy") {
                    item = item.enabled(has_sel);
                }
                menu = menu.item(item);
            }
            tx.popup_menu(menu, e.window(), |s: &mut App, picked, tx| {
                if let Some(id) = picked {
                    s.menu_pick(id, tx);
                }
            });
        } else {
            self.overlay = Overlay::Menu(Menu { x: e.window_x, y: e.window_y, sel: first_item() });
        }
    }

    /// Keys while the in-app menu shows.
    pub fn menu_key(&mut self, k: KeyInput, tx: &mut Tx) {
        let Overlay::Menu(m) = &mut self.overlay else { return };
        let ids: Vec<usize> = (0..ITEMS.len()).filter(|&i| ITEMS[i].0 != "-").collect();
        let at = ids.iter().position(|&i| i == m.sel).unwrap_or(0);
        match k.code {
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => m.sel = ids[(at + 1) % ids.len()],
            KeyCode::Up | KeyCode::Char('k') => m.sel = ids[(at + ids.len() - 1) % ids.len()],
            KeyCode::Enter | KeyCode::Char(' ') => {
                let id = ITEMS[m.sel].0;
                self.overlay = Overlay::None;
                self.menu_pick(id, tx);
            }
            _ => self.overlay = Overlay::None,
        }
    }

    pub fn menu_pick(&mut self, id: &str, tx: &mut Tx) {
        self.overlay = Overlay::None;
        match id {
            "cut" | "copy" => {
                let Some((a, b)) = self.selection_chars() else { return };
                let text = self.ed.buf().slice(a, b);
                let linewise = self.ed.mode == Mode::Visual(VisualKind::Line);
                self.ed.regs.set(None, editor::editor::Reg { text: text.clone(), linewise }, id == "copy");
                tx.copy(text);
                if id == "cut" {
                    let doc = self.ed.doc_mut();
                    let at = doc.buf.pos(a);
                    doc.buf.begin(at);
                    doc.buf.remove(a, b);
                    doc.buf.commit(at);
                    self.edited();
                }
                self.leave_selection(if id == "cut" { Some(a) } else { None });
            }
            "paste" => {
                if let Some((a, b)) = self.selection_chars() {
                    let doc = self.ed.doc_mut();
                    let at = doc.buf.pos(a);
                    doc.buf.begin(at);
                    doc.buf.remove(a, b);
                    doc.buf.commit(at);
                    self.leave_selection(Some(a));
                }
                self.run_actions(vec![Action::Paste { before: true, count: 1 }], tx);
            }
            "all" => {
                let last = self.ed.buf().last_line();
                let end = Pos::new(last, self.ed.buf().line_len(last));
                match self.ed.keymap {
                    Keymap::Vim => {
                        self.ed.mode = Mode::Visual(VisualKind::Line);
                        self.ed.doc_mut().view.anchor = Some(Pos::new(0, 0));
                    }
                    Keymap::Classic => self.ed.doc_mut().view.anchor = Some(Pos::new(0, 0)),
                }
                self.ed.set_cursor(end);
            }
            "definition" => self.run_actions(vec![Action::GotoDefinition], tx),
            "references" => self.references(),
            "rename" => self.start_rename(),
            "actions" => self.run_actions(vec![Action::CodeActions], tx),
            "grep" => {
                let cur = self.ed.cursor();
                let (a, b) = editor::vim::word_range_at(self.ed.buf(), cur);
                let word = self.selection_chars().map(|(a, b)| self.ed.buf().slice(a, b)).unwrap_or_else(|| self.ed.buf().slice(a, b));
                self.open_picker(super::PickKind::Grep);
                if !word.contains('\n') {
                    for c in word.chars() {
                        self.picker_key(KeyInput::ch(c), tx);
                    }
                }
            }
            _ => {}
        }
    }

    /// The selection as a char range `[a, b)`, for the active keymap and mode.
    pub fn selection_chars(&self) -> Option<(usize, usize)> {
        if !self.ed.has_doc() {
            return None;
        }
        let doc = self.ed.doc();
        let anchor = doc.view.anchor?;
        let cur = doc.view.cursor;
        let (s, e) = if anchor <= cur { (anchor, cur) } else { (cur, anchor) };
        let buf = &doc.buf;
        match (self.ed.keymap, self.ed.mode) {
            (Keymap::Vim, Mode::Visual(VisualKind::Line)) => {
                let end = if e.line < buf.last_line() { buf.idx(Pos::new(e.line + 1, 0)) } else { buf.len_chars() };
                Some((buf.idx(Pos::new(s.line, 0)), end))
            }
            (Keymap::Vim, Mode::Visual(_)) => Some((buf.idx(s), (buf.idx(e) + 1).min(buf.len_chars()))),
            (Keymap::Classic, _) => (s != e).then(|| (buf.idx(s), buf.idx(e))),
            _ => None,
        }
    }

    /// Ends Visual mode or the classic selection, with the cursor at char `at` when given.
    fn leave_selection(&mut self, at: Option<usize>) {
        if self.ed.keymap == Keymap::Vim && matches!(self.ed.mode, Mode::Visual(_)) {
            self.ed.mode = Mode::Normal;
        }
        self.ed.doc_mut().view.anchor = None;
        if let Some(a) = at {
            let p = self.ed.buf().pos(a);
            self.ed.set_cursor(p);
        }
    }
}

fn first_item() -> usize {
    ITEMS.iter().position(|i| i.0 != "-").unwrap_or(0)
}
