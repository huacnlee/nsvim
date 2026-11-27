//! Key routing. A key goes to the first of: setup, Settings while it shows, an open overlay
//! (leader menu, picker), the command line, the focused file tree, then the active keymap.

use super::fx::Tx;
use editor::editor::{Action, KeyCode, KeyInput, Keymap, Mode};

use super::{App, Overlay, Screen};

/// The classic keymap's window shortcuts. They work with no file open. Settings keeps them out.
pub fn classic_window_key(k: &KeyInput) -> Option<Action> {
    if !k.cmd {
        return None;
    }
    // Some platforms send `Ctrl ⇧ P` as an uppercase `P`.
    let code = match k.code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        c => c,
    };
    Some(match code {
        KeyCode::Char('p') if k.shift => Action::Palette,
        KeyCode::Char('e') if k.shift => Action::FocusTree,
        KeyCode::Char('p') => Action::FindFile,
        KeyCode::Char('o') => Action::OpenProject,
        KeyCode::Char('f') if k.shift => Action::GrepProject,
        KeyCode::Char('b') => Action::ToggleTree,
        KeyCode::Char(',') => Action::Settings,
        KeyCode::Char('q') => Action::QuitAll { force: false },
        KeyCode::Tab if k.shift => Action::PrevBuffer,
        KeyCode::Tab => Action::NextBuffer,
        _ => return None,
    })
}

impl App {
    pub fn key(&mut self, mut k: KeyInput, tx: &mut Tx) {
        if let Screen::Setup(_) = self.screen {
            self.setup_key(k, tx);
            return;
        }
        if self.ed.keymap == Keymap::Vim && k.ctrl && k.code == KeyCode::Char('[') {
            k = KeyInput::plain(KeyCode::Esc);
        }
        // Settings covers the whole window, so it takes every key while it shows.
        if self.on_settings {
            self.settings_key(k, tx);
            return;
        }
        // F1 shows every key from anywhere, as Vim's F1 shows its help. F1 again closes it.
        if k.code == KeyCode::F(1) {
            let open = matches!(&self.overlay, Overlay::Pick(p) if p.kind == super::picker::PickKind::Keys);
            self.overlay = Overlay::None;
            if !open {
                self.open_key_list(String::new());
            }
            return;
        }
        // `Ctrl ⇧ P` (`⇧ ⌘ P` on macOS) opens the command palette in every keymap, as in Zed.
        // Again closes it. The palette holds the menu, which Linux and Windows do not show.
        if k.cmd && k.shift && matches!(k.code, KeyCode::Char('p' | 'P')) {
            let open = matches!(&self.overlay, Overlay::Pick(p) if p.kind == super::picker::PickKind::Commands);
            self.overlay = Overlay::None;
            if !open {
                self.open_picker(super::picker::PickKind::Commands);
            }
            return;
        }
        // Zoom and Reopen Closed Tab work in every keymap, with or without a file, as in a
        // desktop editor.
        if let Some(z) = super::view_cmds::Zoom::from_key(&k) {
            self.overlay = Overlay::None;
            self.zoom(z);
            return;
        }
        if super::view_cmds::is_reopen_key(&k) {
            self.overlay = Overlay::None;
            self.reopen_closed();
            return;
        }
        match &self.overlay {
            Overlay::Leader(group) => {
                let group = *group;
                self.leader_key(group, k, tx);
                return;
            }
            Overlay::Pick(_) => {
                self.picker_key(k, tx);
                return;
            }
            Overlay::Menu(_) => {
                self.menu_key(k, tx);
                return;
            }
            Overlay::None => {}
        }
        if self.find.is_some() && self.ed.has_doc() && self.find_key(k, tx) {
            return;
        }
        // Classic `⌘ F` and `⌘ ⌥ F` open the find bar in a window. The terminal keeps the
        // search line and `:%s/`.
        if self.ed.keymap == Keymap::Classic
            && k.cmd
            && !k.shift
            && !crate::terminal()
            && self.ed.has_doc()
            && self.ed.mode != Mode::Command
            && matches!(k.code, KeyCode::Char('f' | 'F'))
        {
            self.open_find(k.alt);
            return;
        }
        // An agent's proposal takes y, n and scrolling while it shows.
        if self.ed.mode != Mode::Command && self.proposal_key(k) {
            return;
        }
        if self.note.is_some() && k.code == KeyCode::Esc && self.ed.mode != Mode::Insert {
            self.note = None;
        }
        if self.ed.mode == Mode::Command {
            let actions = editor::vim::cmdline_key(&mut self.ed, k);
            self.after_keymap(actions, tx);
            return;
        }
        // A delete waits for `y` only while the tree keeps the keyboard.
        if !(self.tree.focus && self.tree.show) {
            self.tree.confirm = None;
        }
        if self.tree.focus && self.tree.show && self.tree_key(k, tx) {
            return;
        }
        if !self.ed.has_doc() {
            self.docless_key(k, tx);
            return;
        }
        if self.popup_key(k, tx) {
            return;
        }
        // LSP keys every keymap shares: F2 renames, Ctrl+. (⌘. on macOS) lists code actions.
        if k.code == KeyCode::F(2) {
            self.start_rename();
            return;
        }
        if k.cmd && k.code == KeyCode::Char('.') && self.ed.keymap == Keymap::Classic {
            self.run_actions(vec![Action::CodeActions], tx);
            return;
        }
        let before = self.ed.buf().version();
        let actions = match self.ed.keymap {
            Keymap::Vim => editor::vim::handle_key(&mut self.ed, k),
            Keymap::Classic => editor::classic::handle_key(&mut self.ed, k),
        };
        if self.ed.has_doc() && self.ed.buf().version() != before {
            self.edited();
            self.after_typing(k);
        } else if !matches!(k.code, KeyCode::Char(_)) || k.ctrl {
            self.completion = None;
        }
        self.after_keymap(actions, tx);
    }

    fn after_keymap(&mut self, actions: Vec<Action>, tx: &mut Tx) {
        self.run_actions(actions, tx);
    }

    /// Keys with no file open: the leader, `:`, and the classic shortcuts that open things.
    fn docless_key(&mut self, k: KeyInput, tx: &mut Tx) {
        // `Ctrl-w h`, `w`, `p` or `Ctrl-w` give the keyboard to the tree, as in a file.
        if std::mem::take(&mut self.tree.window_key) {
            if let KeyCode::Char('h' | 'w' | 'p') = k.code {
                self.focus_tree();
            }
            return;
        }
        if self.ed.keymap == Keymap::Vim && k.ctrl && k.code == KeyCode::Char('w') {
            self.tree.window_key = true;
            return;
        }
        let actions = match (self.ed.keymap, k.code) {
            (Keymap::Vim, KeyCode::Char(c)) if c == self.ed.vim.leader && !k.ctrl => vec![Action::LeaderMenu],
            (Keymap::Vim, KeyCode::Char(':')) => {
                editor::vim::open_cmdline(&mut self.ed, ':', "");
                Vec::new()
            }
            (Keymap::Classic, _) => classic_window_key(&k).into_iter().collect(),
            _ => Vec::new(),
        };
        self.run_actions(actions, tx);
    }

    /// A key of the leader menu: a group opens, any other key runs its command. `SPC e` shows
    /// and focuses the tree, and from the tree hides it. The leader menu and the palette run
    /// the same commands, see `palette`.
    fn leader_key(&mut self, group: Option<char>, k: KeyInput, tx: &mut Tx) {
        self.overlay = Overlay::None;
        let KeyCode::Char(c) = k.code else { return };
        if group.is_none() {
            if c == super::palette::PALETTE_KEY {
                return self.run_actions(vec![Action::Palette], tx);
            }
            if super::palette::GROUPS.iter().any(|g| g.0 == c) {
                self.overlay = Overlay::Leader(Some(c));
                return;
            }
            if c == 'e' {
                if self.tree.show && self.tree.focus {
                    self.tree.show = false;
                    self.tree.focus = false;
                } else {
                    self.focus_tree();
                }
                return;
            }
        }
        if let Some(cmd) = super::palette::leader_command(group, c) {
            // `SPC l n` is typed: it renames on the command line, as F2 does. The palette and
            // the menus ask in a dialog.
            if cmd.id == "rename" {
                return self.start_rename();
            }
            self.run_command(cmd.id, tx);
        }
    }

    /// Moves to the next (or previous) git change in the gutter.
    pub(super) fn jump_sign(&mut self, forward: bool) {
        if !self.ed.has_doc() {
            return;
        }
        let Some(m) = self.ed.buf().path.as_ref().and_then(|p| self.meta.get(p)) else {
            self.ed.message("no changes against HEAD");
            return;
        };
        let line = self.ed.cursor().line;
        // A change is a run of lines with signs: jump between the first lines of runs.
        let lines: std::collections::BTreeSet<usize> = m.signs.iter().map(|s| s.line).collect();
        let starts: Vec<usize> = lines.iter().copied().filter(|&l| l == 0 || !lines.contains(&(l - 1))).collect();
        let target = if forward {
            starts.iter().find(|&&l| l > line).or(starts.first())
        } else {
            starts.iter().rev().find(|&&l| l < line).or(starts.last())
        };
        match target.copied() {
            Some(l) => self.ed.set_cursor(editor::buffer::Pos::new(l, 0)),
            None => self.ed.message("no changes against HEAD"),
        }
    }
}
