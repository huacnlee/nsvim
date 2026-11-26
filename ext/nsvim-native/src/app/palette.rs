//! The command palette: every command of the application menu and more, by name, with the
//! keys of the active keymap beside each one. `Ctrl ⇧ P` (`⇧ ⌘ P` on macOS) opens it in every
//! keymap, `SPC :` and `:Commands` in the Neovim keymap. On Linux and Windows nsvim's own title
//! bar hides the window's menu bar, and the terminal editor has no menu, so the palette must
//! hold every item of the menu. `app_menu_tests` checks this.

use editor::editor::{Action, Keymap, Mode};

use super::App;
use super::fx::Tx;

/// One entry of the palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    /// What the palette shows and filters on.
    pub label: &'static str,
    /// What `run_command` gets.
    pub id: &'static str,
    /// The keys in the Neovim keymap. `SPC` is the leader. Empty: no key.
    pub vim: &'static str,
    /// The keys in the classic keymap. `Mod` is `⌘` on macOS and `Ctrl` elsewhere.
    pub classic: &'static str,
}

const fn cmd(label: &'static str, id: &'static str, vim: &'static str, classic: &'static str) -> Command {
    Command { label, id, vim, classic }
}

/// The palette's entries, in the order the empty query shows them.
pub const COMMANDS: &[Command] = &[
    cmd("Find file…", "find", "SPC f", "Mod P"),
    cmd("Find in project…", "grep", "SPC /", "Mod ⇧ F"),
    cmd("Open project…", "project", "SPC p", "Mod O"),
    cmd("Open folder…", "folder", "", ""),
    cmd("Save", "save", "SPC w", "Mod S"),
    cmd("Save all", "save-all", ":wa", ""),
    cmd("Close tab", "close", "SPC q", "Mod W"),
    cmd("Next tab", "next", "gt", "Ctrl Tab"),
    cmd("Previous tab", "prev", "gT", "Ctrl ⇧ Tab"),
    cmd("Undo", "undo", "u", "Mod Z"),
    cmd("Redo", "redo", "Ctrl r", "Mod ⇧ Z"),
    cmd("Cut", "cut", "d", "Mod X"),
    cmd("Copy", "copy", "y", "Mod C"),
    cmd("Paste", "paste", "p", "Mod V"),
    cmd("Select all", "all", "ggVG", "Mod A"),
    cmd("Go to definition", "definition", "gd", "F12"),
    cmd("Find references…", "references", "SPC l r", ""),
    cmd("Rename symbol…", "rename", "SPC l n", "F2"),
    cmd("Code actions…", "actions", "SPC l a", "Mod ."),
    cmd("Toggle file tree", "tree", "SPC e", "Mod B"),
    cmd("Toggle soft wrap", "wrap", "", ""),
    cmd("Open settings…", "settings", "SPC ,", "Mod ,"),
    cmd("Keymap: Neovim", "keymap=neovim", "", ""),
    cmd("Keymap: classic", "keymap=classic", "", ""),
    cmd("Theme: dark", "theme=dark", "", ""),
    cmd("Theme: light", "theme=light", "", ""),
    cmd("Theme: follow the OS", "theme=system", "", ""),
    cmd("Show every key…", "keys", "SPC ?", "F1"),
    cmd("Agent: list outside changes…", "outside", "SPC a l", ""),
    cmd("Agent: clear change marks", "clear-outside", "SPC a c", ""),
    cmd("Agent: show the next proposal", "proposal", "SPC a p", ""),
    cmd("Quit", "quit", ":qa", "Mod Q"),
];

/// The commands that work on the open file. They do nothing with no file open.
const ON_FILE: &[&str] = &["undo", "redo", "cut", "copy", "paste", "all", "definition", "references", "rename", "actions"];

impl Command {
    /// The keys to show for `keymap`, with `leader` for `SPC` and the platform's modifier.
    pub fn keys(&self, keymap: Keymap, leader: char, mac: bool) -> String {
        match keymap {
            Keymap::Vim if leader == ' ' => self.vim.to_string(),
            Keymap::Vim => self.vim.replace("SPC", &leader.to_string()),
            Keymap::Classic => self.classic.replace("Mod", if mac { "⌘" } else { "Ctrl" }),
        }
    }
}

impl App {
    /// The keys of palette entry `index` in the active keymap.
    pub fn command_keys(&self, index: usize) -> String {
        COMMANDS.get(index).map(|c| c.keys(self.ed.keymap, self.ed.vim.leader, cfg!(target_os = "macos"))).unwrap_or_default()
    }

    /// Runs a palette or menu command by id.
    pub fn run_command(&mut self, id: &str, tx: &mut Tx) {
        if ON_FILE.contains(&id) && (!self.ed.has_doc() || self.on_settings) {
            if !self.on_settings {
                self.ed.message("no file is open");
            }
            return;
        }
        let action = match id {
            "find" => Action::FindFile,
            "grep" => Action::GrepProject,
            "project" => Action::OpenProject,
            "folder" => return self.choose_folder(tx),
            "save" => Action::Save(None),
            "save-all" => Action::SaveAll,
            "tree" => Action::ToggleTree,
            "settings" => Action::Settings,
            "close" => Action::CloseBuffer,
            "next" => Action::NextBuffer,
            "prev" => Action::PrevBuffer,
            "undo" | "redo" => return self.undo_redo(id == "undo"),
            "cut" | "copy" | "paste" | "all" | "definition" | "references" | "rename" | "actions" => return self.menu_pick(id, tx),
            "wrap" => Action::SetOption("wrap".into(), (!self.cfg.editor.wrap).to_string()),
            "keys" => Action::KeyList(String::new()),
            "outside" => {
                self.show_outside();
                return;
            }
            "clear-outside" => {
                self.clear_outside(None);
                self.ed.message("change marks cleared");
                return;
            }
            "proposal" => {
                match self.proposals.first().map(|p| p.path.clone()) {
                    Some(p) => self.open_path(p, None),
                    None => self.ed.message("no proposals"),
                }
                return;
            }
            "quit" => Action::QuitAll { force: false },
            other => match other.split_once('=') {
                Some((k, v)) => Action::SetOption(k.into(), v.into()),
                None => return,
            },
        };
        self.run_actions(vec![action], tx);
    }

    /// Undo or redo one step, as `u` and `Ctrl r` do. The Neovim keymap goes back to Normal mode.
    fn undo_redo(&mut self, undo: bool) {
        let buf = &mut self.ed.doc_mut().buf;
        let at = if undo { buf.undo() } else { buf.redo() };
        match at {
            Some(at) => {
                if self.ed.keymap == Keymap::Vim {
                    self.ed.mode = Mode::Normal;
                }
                self.ed.doc_mut().view.anchor = None;
                self.ed.set_cursor(at);
                self.edited();
            }
            None => self.ed.message(if undo { "Already at oldest change" } else { "Already at newest change" }),
        }
    }
}
