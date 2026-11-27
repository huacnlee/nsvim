//! The application menu: the menu bar on macOS, and on Windows and Linux the menus of the
//! window, which nsvim's own title bar hides. The terminal editor has none. So the command
//! palette holds every item. Each item names a palette command by id, so the menu is data that
//! tests can walk, and `app_menu_pick` carries an id out.

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
    /// macOS's Hide: the app's windows hide, and the Dock icon or `⌘ Tab` shows them again.
    /// The label and the shortcut, as `nsh/keymap` writes it (`None`: the platform's `⌘ H`).
    Hide(String, Option<String>),
    /// macOS's Hide Others.
    HideOthers,
    /// macOS's Show All.
    ShowAll,
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
    /// The shortcut of macOS's Close Window: `⌘ W` as in every Mac app, unless something in the
    /// window takes it (the classic keymap closes the tab with it, Settings closes itself).
    /// Then `⇧ ⌘ W`, as in VS Code.
    pub fn close_window_key(&self) -> &'static str {
        if self.ed.keymap == editor::editor::Keymap::Vim && !self.on_settings { "mod+w" } else { "mod+shift+w" }
    }

    /// The menus, with `recent` as the projects of File › Open Recent. `mac` adds the Mac's
    /// window items: Hide, Hide Others, Show All, and Close Window, which hides the app and
    /// does not quit it.
    pub fn app_menu(&self, recent: &[PathBuf], mac: bool) -> Vec<Entry> {
        let mut open_recent: Vec<Entry> = recent.iter().map(|r| Entry::Item(recent_id(r), super::project::shown(r))).collect();
        if open_recent.is_empty() {
            open_recent.push(item("none", "No recent projects"));
        }
        let mut file = vec![
            item("cmd:new", "New File"),
            Entry::Separator,
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
        if mac {
            file.push(Entry::Hide("Close Window".into(), Some(self.close_window_key().into())));
        }
        let edit = vec![
            item("undo", "Undo"),
            item("redo", "Redo"),
            Entry::Separator,
            item("edit:cut", "Cut"),
            item("edit:copy", "Copy"),
            item("edit:paste", "Paste"),
            item("edit:all", "Select All"),
            Entry::Separator,
            item("cmd:find-in-file", "Find…"),
            item("cmd:replace-in-file", "Replace…"),
            item("cmd:grep", "Find in Project…"),
        ];
        let view = vec![
            item("cmd:palette", "Command Palette…"),
            item("cmd:tree", "Toggle File Tree"),
            item("cmd:wrap", "Toggle Soft Wrap"),
            item("cmd:git-blame", "Toggle Line Blame"),
            Entry::Separator,
            item("cmd:next", "Next Tab"),
            item("cmd:prev", "Previous Tab"),
            Entry::Separator,
            item("cmd:split-right", "Split Right"),
            item("cmd:split-down", "Split Down"),
            item("cmd:pane-next", "Focus Next Pane"),
            item("cmd:pane-prev", "Focus Previous Pane"),
            item("cmd:pane-close", "Close Pane"),
        ];
        let go = vec![
            item("cmd:back", "Back"),
            item("cmd:forward", "Forward"),
            Entry::Separator,
            item("cmd:line", "Go to Line…"),
            item("cmd:symbols", "Go to Symbol in File…"),
            Entry::Separator,
            item("edit:definition", "Go to Definition"),
            item("edit:references", "Find References…"),
            item("edit:rename", "Rename Symbol…"),
            item("edit:actions", "Code Actions…"),
        ];
        let help = vec![item("cmd:keys", "Keyboard Shortcuts…")];
        let mut app = vec![item("cmd:settings", "Settings…"), item("cmd:setup", "App Setup…"), Entry::Separator];
        app.insert(0, item("cmd:update", "Check for Updates…"));
        if mac {
            app.extend([Entry::Hide("Hide NsVim".into(), None), Entry::HideOthers, Entry::ShowAll, Entry::Separator]);
        }
        app.push(Entry::Quit);
        vec![
            // The app menu on macOS; elsewhere a menu named after the app.
            Entry::Menu("NsVim".into(), app),
            Entry::Menu("File".into(), file),
            Entry::Menu("Edit".into(), edit),
            Entry::Menu("View".into(), view),
            Entry::Menu("Go".into(), go),
            Entry::Menu("Help".into(), help),
        ]
    }

    /// Carries out the menu item `id`. Every item but Open Recent is a palette command.
    pub fn app_menu_pick(&mut self, id: &str, tx: &mut Tx) {
        // A menu is chosen over whatever shows: close a picker or the in-app menu first.
        self.overlay = super::Overlay::None;
        if id == "cmd:palette" {
            return self.open_picker(super::picker::PickKind::Commands);
        }
        if let Some(root) = id.strip_prefix("recent:") {
            return self.open_project(PathBuf::from(root), tx);
        }
        let cmd = id.strip_prefix("cmd:").or_else(|| id.strip_prefix("edit:")).unwrap_or(id);
        self.run_command(cmd, tx);
    }
}
