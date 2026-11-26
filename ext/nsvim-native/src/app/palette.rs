//! The command palette and the leader menu: one list of commands. The palette runs them by
//! name, the leader menu (`SPC` in the Neovim keymap) by keys, so both hold every command.
//! `Ctrl ⇧ P` (`⇧ ⌘ P` on macOS) opens the palette in every keymap, `SPC p` and `:Commands` in
//! the Neovim keymap. On Linux and Windows nsvim's own title bar hides the window's menu bar,
//! and the terminal editor has no menu, so the palette must hold every item of the menu.
//! `app_menu_tests` checks this.

use editor::editor::{Action, Keymap, Mode};

use super::App;
use super::fx::Tx;

/// One command: a row of the palette and a key of the leader menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    /// What the palette shows and filters on.
    pub label: &'static str,
    /// What `run_command` gets.
    pub id: &'static str,
    /// The keys after the leader: `f`, or a group and a key, `g n`. See `GROUPS`.
    pub leader: &'static str,
    /// The Neovim keymap's own keys, when it has some besides the leader (`u`, `gd`). The
    /// palette shows them before the leader keys.
    pub vim: &'static str,
    /// The keys in the classic keymap. `Mod` is `⌘` on macOS and `Ctrl` elsewhere.
    pub classic: &'static str,
}

const fn cmd(label: &'static str, id: &'static str, leader: &'static str, vim: &'static str, classic: &'static str) -> Command {
    Command { label, id, leader, vim, classic }
}

/// The leader menu's groups: the key that opens one, and its name. A command whose label
/// starts with the name and `: ` shows without that part in the group.
pub const GROUPS: &[(char, &str)] = &[('g', "Git"), ('l', "LSP"), ('a', "Agent"), ('t', "Tab"), ('y', "Edit"), ('u', "UI")];

/// `SPC p`: the leader menu's key for the palette itself, which the palette does not list.
pub const PALETTE_KEY: char = 'p';

/// The commands, in the order the empty palette and the leader menu show them.
pub const COMMANDS: &[Command] = &[
    cmd("Find file…", "find", "f", "", "Mod P"),
    cmd("Open project…", "project", "o", "", "Mod O"),
    cmd("Open folder…", "folder", "O", "", ""),
    cmd("Toggle file tree", "tree", "e", "", "Mod B"),
    cmd("Find in project…", "grep", "/", "", "Mod ⇧ F"),
    cmd("Buffers…", "buffers", "b", "", ""),
    cmd("Save", "save", "w", "", "Mod S"),
    cmd("Save all", "save-all", "W", ":wa", ""),
    cmd("Close tab", "close", "q", "", "Mod W"),
    cmd("Quit", "quit", "Q", ":qa", "Mod Q"),
    cmd("Diagnostics…", "diagnostics", "d", "", ""),
    cmd("Open settings…", "settings", ",", "", "Mod ,"),
    cmd("App Setup…", "setup", "u S", ":Setup", ""),
    cmd("Show every key…", "keys", "?", "", "F1"),
    cmd("Git: next change", "git-next", "g n", "", ""),
    cmd("Git: previous change", "git-prev", "g p", "", ""),
    cmd("Git: status in the tree", "git-status", "g s", "", "Mod ⇧ E"),
    cmd("Go to definition", "definition", "l d", "gd", "F12"),
    cmd("Find references…", "references", "l r", "", ""),
    cmd("Hover", "hover", "l h", "K", ""),
    cmd("Rename symbol…", "rename", "l n", "", "F2"),
    cmd("Code actions…", "actions", "l a", "", "Mod ."),
    cmd("Agent: list outside changes…", "outside", "a l", "", ""),
    cmd("Agent: clear change marks", "clear-outside", "a c", "", ""),
    cmd("Agent: show the next proposal", "proposal", "a p", "", ""),
    cmd("Agent: clear the note", "clear-note", "a n", "", ""),
    cmd("Next tab", "next", "t n", "gt", "Ctrl Tab"),
    cmd("Previous tab", "prev", "t p", "gT", "Ctrl ⇧ Tab"),
    cmd("Close other tabs", "close-others", "t o", "", ""),
    cmd("Close tabs to the left", "close-left", "t l", "", ""),
    cmd("Close tabs to the right", "close-right", "t r", "", ""),
    cmd("Close clean tabs", "close-clean", "t c", "", ""),
    cmd("Close all tabs", "close-all", "t a", "", ""),
    cmd("Undo", "undo", "y u", "u", "Mod Z"),
    cmd("Redo", "redo", "y r", "Ctrl r", "Mod ⇧ Z"),
    cmd("Cut", "cut", "y x", "d", "Mod X"),
    cmd("Copy", "copy", "y y", "y", "Mod C"),
    cmd("Paste", "paste", "y p", "p", "Mod V"),
    cmd("Select all", "all", "y a", "ggVG", "Mod A"),
    cmd("Toggle soft wrap", "wrap", "u w", "", ""),
    cmd("Keymap: Neovim", "keymap=neovim", "u v", "", ""),
    cmd("Keymap: classic", "keymap=classic", "u c", "", ""),
    cmd("Theme: dark", "theme=dark", "u d", "", ""),
    cmd("Theme: light", "theme=light", "u l", "", ""),
    cmd("Theme: follow the OS", "theme=system", "u s", "", ""),
];

/// The leader menu of `group` (`None`: the top level): key, label, and whether it opens a group.
pub fn leader_menu(group: Option<char>) -> Vec<(char, String, bool)> {
    let mut out = Vec::new();
    match group {
        None => {
            for c in COMMANDS {
                if let [k] = c.leader.chars().collect::<Vec<_>>()[..] {
                    out.push((k, c.label.to_string(), false));
                    if k == 'f' {
                        out.push((PALETTE_KEY, "Commands…".to_string(), false));
                    }
                }
            }
            out.extend(GROUPS.iter().map(|&(k, name)| (k, format!("+{name}"), true)));
        }
        Some(g) => {
            let name = GROUPS.iter().find(|x| x.0 == g).map_or("", |x| x.1);
            for c in COMMANDS {
                if let Some(k) = c.leader.strip_prefix(g).and_then(|r| r.strip_prefix(' ')).and_then(|r| r.chars().next()) {
                    out.push((k, short_label(c.label, name), false));
                }
            }
        }
    }
    out
}

/// The command that leader key `key` runs in `group`.
pub fn leader_command(group: Option<char>, key: char) -> Option<&'static Command> {
    let path = match group {
        Some(g) => format!("{g} {key}"),
        None => key.to_string(),
    };
    COMMANDS.iter().find(|c| c.leader == path)
}

/// `Git: next change` in the Git group is `Next change`.
fn short_label(label: &str, group: &str) -> String {
    let Some(rest) = label.strip_prefix(group).and_then(|r| r.strip_prefix(": ")) else { return label.to_string() };
    let mut chars = rest.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// The commands that work on the open file. They do nothing with no file open.
const ON_FILE: &[&str] = &["undo", "redo", "cut", "copy", "paste", "all", "definition", "references", "hover", "rename", "actions"];

impl Command {
    /// The keys to show for `keymap`, with `leader` for `SPC` and the platform's modifier. The
    /// Neovim keymap shows its own keys, then the leader keys.
    pub fn keys(&self, keymap: Keymap, leader: char, mac: bool) -> String {
        match keymap {
            Keymap::Vim => {
                let spc = if leader == ' ' { "SPC".to_string() } else { leader.to_string() };
                let lead = if self.leader.is_empty() { String::new() } else { format!("{spc} {}", self.leader) };
                [self.vim, lead.as_str()].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(", ")
            }
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
        if let Some(which) = super::tab_menu::CloseTabs::from_id(id) {
            if self.ed.has_doc() && !self.on_settings {
                self.close_tabs(self.ed.active, which, tx);
            }
            return;
        }
        let action = match id {
            "palette" => Action::Palette,
            "find" => Action::FindFile,
            "grep" => Action::GrepProject,
            "project" => Action::OpenProject,
            "folder" => return self.choose_folder(tx),
            "save" => Action::Save(None),
            "save-all" => Action::SaveAll,
            "tree" => Action::ToggleTree,
            "buffers" => Action::PickBuffer,
            "diagnostics" => Action::Diagnostics,
            "git-next" | "git-prev" => return self.jump_sign(id == "git-next"),
            "git-status" => Action::FocusTree,
            "hover" => Action::Hover,
            "settings" => Action::Settings,
            "setup" => Action::Setup,
            "close" => return self.close_tab(self.ed.active, tx),
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
            "clear-note" => {
                self.note = None;
                return;
            }
            "quit" => return self.close_requested(tx),
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
