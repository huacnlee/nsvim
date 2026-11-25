//! Key routing. A key goes to the first of: setup, Settings while it shows, an open overlay
//! (leader menu, picker), the command line, the focused file tree, then the active keymap.

use super::fx::Tx;
use editor::editor::{Action, KeyCode, KeyInput, Keymap, Mode};

use super::{App, Overlay, Screen};

/// The leader menu's entries: key, label, and whether it opens a group.
pub const LEADER: &[(char, &str, bool)] = &[
    ('f', "Find file…", false),
    ('e', "File tree", false),
    ('/', "Grep project…", false),
    ('b', "Buffers…", false),
    ('w', "Write", false),
    ('q', "Close buffer", false),
    ('d', "Diagnostics…", false),
    (',', "Settings…", false),
    ('?', "Every key…", false),
    ('g', "+Git", true),
    ('l', "+LSP", true),
    ('a', "+Agent", true),
];

pub const LEADER_AGENT: &[(char, &str, bool)] =
    &[('l', "Outside changes…", false), ('c', "Clear change marks", false), ('p', "Next proposal", false), ('n', "Clear the note", false)];

pub const LEADER_GIT: &[(char, &str, bool)] =
    &[('n', "Next change", false), ('p', "Previous change", false), ('s', "Status in tree", false)];
pub const LEADER_LSP: &[(char, &str, bool)] = &[
    ('d', "Definition", false),
    ('r', "References…", false),
    ('h', "Hover", false),
    ('n', "Rename…", false),
    ('a', "Code actions…", false),
];

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

    fn leader_key(&mut self, group: Option<char>, k: KeyInput, tx: &mut Tx) {
        self.overlay = Overlay::None;
        let KeyCode::Char(c) = k.code else { return };
        let actions = match (group, c) {
            (None, 'f') => vec![Action::FindFile],
            (None, 'e') => {
                // Show and focus the tree; from the tree, hide it again.
                if self.tree.show && self.tree.focus {
                    self.tree.show = false;
                    self.tree.focus = false;
                } else {
                    self.focus_tree();
                }
                return;
            }
            (None, '/') => vec![Action::GrepProject],
            (None, '?') => vec![Action::KeyList(String::new())],
            (None, 'b') => vec![Action::PickBuffer],
            (None, 'w') => vec![Action::Save(None)],
            (None, 'q') => vec![Action::CloseBuffer],
            (None, 'd') => vec![Action::Diagnostics],
            (None, ',') => vec![Action::Settings],
            (None, 'g') | (None, 'l') | (None, 'a') => {
                self.overlay = Overlay::Leader(Some(c));
                return;
            }
            (Some('g'), 'n') => {
                self.jump_sign(true);
                return;
            }
            (Some('g'), 'p') => {
                self.jump_sign(false);
                return;
            }
            (Some('g'), 's') => vec![Action::FocusTree],
            (Some('l'), 'd') => vec![Action::GotoDefinition],
            (Some('l'), 'r') => {
                self.references();
                return;
            }
            (Some('l'), 'n') => {
                self.start_rename();
                return;
            }
            (Some('l'), 'a') => vec![Action::CodeActions],
            (Some('a'), 'l') => {
                self.show_outside();
                return;
            }
            (Some('a'), 'c') => {
                self.clear_outside(None);
                self.ed.message("change marks cleared");
                return;
            }
            (Some('a'), 'p') => {
                match self.proposals.first().map(|p| p.path.clone()) {
                    Some(p) => self.open_path(p, None),
                    None => self.ed.message("no proposals"),
                }
                return;
            }
            (Some('a'), 'n') => {
                self.note = None;
                return;
            }
            (Some('l'), 'h') => vec![Action::Hover],
            _ => Vec::new(),
        };
        self.run_actions(actions, tx);
    }

    /// Moves to the next (or previous) git change in the gutter.
    fn jump_sign(&mut self, forward: bool) {
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
