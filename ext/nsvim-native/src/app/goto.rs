//! The Go menu's own places: Go to Line (`Ctrl G`, `⌘ L` on macOS), Go to Symbol in File
//! (`Ctrl ⇧ O`, `⇧ ⌘ O`) and Back and Forward on the jump list (`⌃ -` on macOS, `Ctrl Alt -`
//! elsewhere, `Ctrl ⇧ -`, the mouse's back and forward buttons). Both pickers share the finder overlay.

use editor::buffer::Pos;
use editor::editor::{KeyCode, KeyInput};

use super::fx::Tx;
use super::picker::{PickKind, Picker};
use super::{App, Overlay};

/// Where `query` (`42` or `42:7`, one-based) goes in a buffer of `lines` lines. `None` while
/// it names no line. The line and the column stop at the buffer's end.
pub fn parse_line(query: &str, lines: usize) -> Option<(usize, Option<usize>)> {
    let (l, c) = match query.split_once(':') {
        Some((l, c)) => (l, Some(c)),
        None => (query, None),
    };
    let line = l.trim().parse::<usize>().ok()?.clamp(1, lines.max(1)) - 1;
    let col = c.and_then(|c| c.trim().parse::<usize>().ok()).map(|c| c.max(1) - 1);
    Some((line, col))
}

/// The Go to Line overlay's hint: where Enter goes, or what to type.
pub fn line_hint(query: &str, lines: usize) -> String {
    match parse_line(query, lines) {
        Some((l, Some(c))) => format!("Line {}, column {} of {lines}", l + 1, c + 1),
        Some((l, None)) => format!("Line {} of {lines}", l + 1),
        None => format!("{lines} lines"),
    }
}

/// The classic keymap's Go keys: Go to Line (`Ctrl G`, also `⌘ L` on macOS) and Go to Symbol
/// in File (`Ctrl ⇧ O`, `⇧ ⌘ O` on macOS).
pub fn classic_go_key(k: &KeyInput, mac: bool) -> Option<&'static str> {
    let KeyCode::Char(c) = k.code else { return None };
    if k.alt {
        return None;
    }
    match c.to_ascii_lowercase() {
        'o' if k.cmd && k.shift => Some("symbols"),
        'g' if k.ctrl && !k.shift => Some("line"),
        'l' if mac && k.cmd && !k.ctrl && !k.shift => Some("line"),
        _ => None,
    }
}

/// Back and Forward, in every keymap, as in VS Code: `⌃ -` and `⌃ ⇧ -` on macOS, `Ctrl Alt -`
/// and `Ctrl ⇧ -` on Windows and Linux. There `Ctrl -` is the shortcut key's zoom out, so
/// Back takes Alt too. Control is the shortcut key when it also sets `cmd`.
pub fn jump_key(k: &KeyInput) -> Option<&'static str> {
    if !k.ctrl {
        return None;
    }
    match k.code {
        KeyCode::Char('_') if !k.alt => Some("forward"),
        KeyCode::Char('-') if k.shift && !k.alt => Some("forward"),
        KeyCode::Char('-') if !k.shift && k.alt == k.cmd => Some("back"),
        _ => None,
    }
}

impl App {
    /// Opens Go to Line. The cursor and the scroll position before it come back on Esc.
    pub fn open_go_to_line(&mut self) {
        let mut p = Picker::new(PickKind::Line);
        p.back = Some((self.ed.cursor(), self.ed.doc().view.top));
        self.overlay = Overlay::Pick(Box::new(p));
    }

    /// Shows the place Go to Line's query names, as it is typed. An empty query shows the
    /// place before it again.
    pub(super) fn preview_line(&mut self) {
        let Overlay::Pick(p) = &self.overlay else { return };
        let Some((at, top)) = p.back else { return };
        match parse_line(&p.query, self.ed.buf().line_count()) {
            Some((line, col)) => self.show_line(line, col),
            None => {
                self.ed.set_cursor(at);
                self.ed.doc_mut().view.top = top;
            }
        }
    }

    /// Puts the cursor on `line`, at `col` or the first non-blank, and scrolls the line to a
    /// third of the window from the top when it is off screen.
    fn show_line(&mut self, line: usize, col: Option<usize>) {
        let text = self.ed.buf().line(line);
        let col = col.unwrap_or_else(|| text.chars().take_while(|c| *c == ' ' || *c == '\t').count());
        self.ed.set_cursor(Pos::new(line, col));
        self.center(line);
    }

    fn center(&mut self, line: usize) {
        let rows = self.ed.rows.max(1);
        let doc = self.ed.doc_mut();
        if line < doc.view.top || line >= doc.view.top + rows {
            doc.view.top = line.saturating_sub(rows / 3);
        }
    }

    /// Goes where an overlay previewed: back to the place before it first, so the jump list
    /// records that place for Back.
    fn land(&mut self, back: Option<(Pos, usize)>, to: Pos, keep_col: bool) {
        if let Some((at, top)) = back {
            self.ed.set_cursor(at);
            self.ed.doc_mut().view.top = top;
        }
        editor::vim::remember_jump(&mut self.ed);
        if keep_col {
            self.ed.set_cursor(to);
            self.center(to.line);
        } else {
            self.show_line(to.line, None);
        }
    }

    /// Enter in Go to Line: goes there, or back when the query names no line.
    pub(super) fn accept_line(&mut self) {
        let Overlay::Pick(p) = std::mem::replace(&mut self.overlay, Overlay::None) else { return };
        let Some((at, top)) = p.back else { return };
        match parse_line(&p.query, self.ed.buf().line_count()) {
            Some((line, col)) => self.land(p.back, Pos::new(line, col.unwrap_or(0)), col.is_some()),
            None => {
                self.ed.set_cursor(at);
                self.ed.doc_mut().view.top = top;
            }
        }
    }

    /// Esc in Go to Line: the cursor goes back where it was.
    pub(super) fn cancel_line(&mut self) {
        if let Overlay::Pick(p) = std::mem::replace(&mut self.overlay, Overlay::None)
            && let Some((at, top)) = p.back
        {
            self.ed.set_cursor(at);
            self.ed.doc_mut().view.top = top;
        }
    }

    /// Opens Go to Symbol in File with the tree-sitter symbols of the file.
    pub fn open_symbols(&mut self) {
        let syms = editor::syntax::symbols(self.ed.buf(), self.ed.buf().path.as_deref());
        let mut p = Picker::new(PickKind::Symbols);
        if let Overlay::Pick(old) = &self.overlay {
            p.fit = old.fit;
        }
        p.set_symbols(syms);
        p.refilter(&self.files);
        self.overlay = Overlay::Pick(Box::new(p));
        self.ask_preview();
    }

    /// Enter in Go to Symbol in File.
    pub(super) fn accept_symbol(&mut self) {
        let Overlay::Pick(p) = std::mem::replace(&mut self.overlay, Overlay::None) else { return };
        let Some(s) = p.matches.get(p.sel).and_then(|m| p.symbols.get(m.index)) else { return };
        self.land(None, s.pos, true);
    }

    /// Back or Forward on the jump list.
    pub fn jump_list(&mut self, forward: bool, tx: &mut Tx) {
        if !self.ed.has_doc() || self.on_settings {
            return;
        }
        let actions = editor::vim::jump_list(&mut self.ed, if forward { 1 } else { -1 });
        if actions.is_empty() {
            let line = self.ed.cursor().line;
            self.center(line);
        }
        self.run_actions(actions, tx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: char, ctrl: bool, alt: bool, shift: bool, cmd: bool) -> KeyInput {
        KeyInput { code: KeyCode::Char(code), ctrl, alt, shift, cmd }
    }

    #[test]
    fn line_queries_parse_and_stop_at_the_end() {
        assert_eq!(parse_line("12", 40), Some((11, None)));
        assert_eq!(parse_line("12:7", 40), Some((11, Some(6))));
        assert_eq!(parse_line("99", 40), Some((39, None)));
        assert_eq!(parse_line("0", 40), Some((0, None)));
        assert_eq!(parse_line("", 40), None);
        assert_eq!(parse_line(":3", 40), None);
        assert_eq!(line_hint("12:7", 40), "Line 12, column 7 of 40");
    }

    #[test]
    fn go_keys_follow_the_platform() {
        // Windows and Linux: Control is the shortcut key, so it sets `cmd` too.
        assert_eq!(classic_go_key(&key('g', true, false, false, true), false), Some("line"));
        assert_eq!(classic_go_key(&key('O', true, false, true, true), false), Some("symbols"));
        assert_eq!(classic_go_key(&key('l', true, false, false, true), false), None);
        // macOS: ⌘ L and ⇧ ⌘ O, and ⌃ G too.
        assert_eq!(classic_go_key(&key('l', false, false, false, true), true), Some("line"));
        assert_eq!(classic_go_key(&key('o', false, false, true, true), true), Some("symbols"));
        assert_eq!(classic_go_key(&key('g', true, false, false, false), true), Some("line"));
        // Back and Forward: `Ctrl -` stays the zoom out of Windows and Linux.
        assert_eq!(jump_key(&key('-', true, false, false, true)), None);
        assert_eq!(jump_key(&key('-', true, true, false, true)), Some("back"));
        assert_eq!(jump_key(&key('-', true, false, true, true)), Some("forward"));
        assert_eq!(jump_key(&key('-', true, false, false, false)), Some("back"));
        assert_eq!(jump_key(&key('_', true, false, true, false)), Some("forward"));
        assert_eq!(jump_key(&key('-', false, false, false, true)), None);
    }
}
