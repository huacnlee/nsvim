//! The application menu: the menu bar on macOS, and on Windows and Linux the menus of the
//! window (which nsvim's own title bar hides, so there they are the same commands as the
//! palette). The terminal editor has none. Each item names a command by id, so the menu is
//! data that tests can walk, and `app_menu_pick` carries an id out.

use std::path::PathBuf;

use super::App;
use super::fx::Tx;

/// An item of the application menu.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    /// A top-level menu or a submenu.
    Menu(String, Vec<Entry>),
    /// An item: what `app_menu_pick` gets, and what it shows.
    Item(String, String),
    /// The platform's Quit item, which asks first as the close button does.
    Quit,
    Separator,
}

fn item(id: &str, label: &str) -> Entry {
    Entry::Item(id.to_string(), label.to_string())
}

/// The id of a recent project's item.
fn recent_id(root: &std::path::Path) -> String {
    format!("recent:{}", root.display())
}

impl App {
    /// The menus, with `recent` as the projects of File › Open Recent.
    pub fn app_menu(&self, recent: &[PathBuf]) -> Vec<Entry> {
        let mut open_recent: Vec<Entry> = recent.iter().map(|r| Entry::Item(recent_id(r), super::project::shown(r))).collect();
        if open_recent.is_empty() {
            open_recent.push(item("none", "No recent projects"));
        }
        let file = vec![
            item("cmd:project", "Open Project…"),
            item("folder", "Open Folder…"),
            Entry::Menu("Open Recent".into(), open_recent),
            Entry::Separator,
            item("cmd:find", "Find File…"),
            Entry::Separator,
            item("cmd:save", "Save"),
            item("cmd:save-all", "Save All"),
            Entry::Separator,
            item("cmd:close", "Close Tab"),
        ];
        let edit = vec![
            item("undo", "Undo"),
            item("redo", "Redo"),
            Entry::Separator,
            item("edit:cut", "Cut"),
            item("edit:copy", "Copy"),
            item("edit:paste", "Paste"),
            item("edit:all", "Select All"),
            Entry::Separator,
            item("cmd:grep", "Find in Project…"),
        ];
        let view = vec![
            item("cmd:palette", "Command Palette…"),
            item("cmd:tree", "Toggle File Tree"),
            item("cmd:wrap", "Toggle Soft Wrap"),
            Entry::Separator,
            item("cmd:next", "Next Tab"),
            item("cmd:prev", "Previous Tab"),
        ];
        let go = vec![
            item("edit:definition", "Go to Definition"),
            item("edit:references", "Find References…"),
            item("edit:rename", "Rename Symbol…"),
            item("edit:actions", "Code Actions…"),
        ];
        let help = vec![item("cmd:keys", "Keyboard Shortcuts…")];
        vec![
            // The app menu on macOS; elsewhere a menu named after the app.
            Entry::Menu("nsvim".into(), vec![item("cmd:settings", "Settings…"), Entry::Separator, Entry::Quit]),
            Entry::Menu("File".into(), file),
            Entry::Menu("Edit".into(), edit),
            Entry::Menu("View".into(), view),
            Entry::Menu("Go".into(), go),
            Entry::Menu("Help".into(), help),
        ]
    }

    /// Carries out the menu item `id`.
    pub fn app_menu_pick(&mut self, id: &str, tx: &mut Tx) {
        // A menu is chosen over whatever shows: close a picker or the in-app menu first.
        if !matches!(self.overlay, super::Overlay::None) {
            self.overlay = super::Overlay::None;
        }
        if let Some(cmd) = id.strip_prefix("cmd:") {
            if cmd == "palette" {
                return self.open_picker(super::picker::PickKind::Commands);
            }
            return self.run_command(cmd, tx);
        }
        if let Some(edit) = id.strip_prefix("edit:") {
            if !self.ed.has_doc() || self.on_settings {
                return;
            }
            return self.menu_pick(edit, tx);
        }
        if let Some(root) = id.strip_prefix("recent:") {
            return self.open_project(PathBuf::from(root), tx);
        }
        match id {
            "folder" => self.choose_folder(tx),
            "undo" | "redo" => {
                if !self.ed.has_doc() || self.on_settings {
                    return;
                }
                let buf = &mut self.ed.doc_mut().buf;
                let at = if id == "undo" { buf.undo() } else { buf.redo() };
                match at {
                    Some(at) => {
                        if self.ed.keymap == editor::editor::Keymap::Vim {
                            self.ed.mode = editor::editor::Mode::Normal;
                        }
                        self.ed.doc_mut().view.anchor = None;
                        self.ed.set_cursor(at);
                        self.edited();
                    }
                    None => self.ed.message(if id == "undo" { "Already at oldest change" } else { "Already at newest change" }),
                }
            }
            _ => {}
        }
    }
}
