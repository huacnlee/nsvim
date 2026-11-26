//! The context menu of a tree row (a right-click in the file tree): make a file or a folder,
//! rename, delete, and copy the path. Names come from a native dialog with a text field, and
//! a delete asks in a native alert, so the mouse never lands on the command line. The tree's
//! keys (`a`, `A`, `r`, `d`) keep the command line and the `y` answer.

use std::path::PathBuf;

use super::App;
use super::fx::{Confirm, PopupItem, PopupMenu, Role, Tx};

/// The menu's items: id, title and the tree key that does the same. `-` is a separator.
pub const TREE_ITEMS: &[(&str, &str, &str)] = &[
    ("new-file", "New file…", "a"),
    ("new-folder", "New folder…", "A"),
    ("-", "", ""),
    ("rename", "Rename…", "r"),
    ("delete", "Delete…", "d"),
    ("-", "", ""),
    ("copy-path", "Copy path", ""),
    ("copy-relative-path", "Copy relative path", ""),
];

/// The id of the dialogs' button that does the change.
pub const OK: &str = "ok";

impl App {
    /// A right-click on the tree row `path` (from the root, with `/`): the system's menu at
    /// `at`, in the window. The row gets the keyboard, so the menu's target shows.
    pub fn tree_menu(&mut self, path: String, at: (f32, f32), tx: &mut Tx) {
        tx.instant();
        self.settle_tree_top();
        self.tree.sel = Some(path.clone());
        self.tree.focus = true;
        let mut menu = PopupMenu::new();
        for &(id, title, key) in TREE_ITEMS {
            if id == "-" {
                menu = menu.separator();
                continue;
            }
            let title = if key.is_empty() { title.to_string() } else { format!("{title}    {key}") };
            menu = menu.item(PopupItem::new(id, title));
        }
        tx.popup_menu(menu, at, move |s: &mut App, picked, tx| {
            if let Some(id) = picked {
                s.tree_menu_pick(&path, id, tx);
            }
        });
    }

    /// Runs menu item `id` on the tree row `path`.
    pub fn tree_menu_pick(&mut self, path: &str, id: &str, tx: &mut Tx) {
        let full = self.ed.root.join(path);
        match id {
            "new-file" => self.ask_new(path, false, tx),
            "new-folder" => self.ask_new(path, true, tx),
            "rename" => self.ask_rename_path(path, tx),
            "delete" => self.ask_delete(path, tx),
            "copy-path" => {
                let text = full.display().to_string();
                self.ed.message(format!("copied {text}"));
                tx.copy(text);
            }
            "copy-relative-path" => {
                self.ed.message(format!("copied {path}"));
                tx.copy(path.to_string());
            }
            _ => {}
        }
    }

    /// The folder of the tree row `path`, with a `/` at the end: the row itself when it is a
    /// folder, else the folder it is in. Empty for the root.
    fn row_folder(&self, path: &str) -> String {
        let dir = if self.tree.index.children.contains_key(path) { path } else { path.rsplit_once('/').map_or("", |(d, _)| d) };
        if dir.is_empty() { String::new() } else { format!("{dir}/") }
    }

    /// Asks for the name of a new file or folder in the folder of row `path`. A name that ends
    /// with `/` makes a folder, as `:NewFile` does. Where no dialog can show, the command line
    /// asks.
    fn ask_new(&mut self, path: &str, folder: bool, tx: &mut Tx) {
        let dir = self.row_folder(path);
        let (title, cmd) = if folder { ("New folder", "Mkdir") } else { ("New file", "NewFile") };
        let place = if dir.is_empty() { "In the project folder.".to_string() } else { format!("In \"{dir}\".") };
        let dialog = Confirm::new(title, place).button(OK, "Create", Role::Default).button("cancel", "Cancel", Role::Cancel);
        tx.prompt(dialog, "", move |a, typed, _| match typed {
            Ok(Some(name)) => {
                let name = name.trim();
                if name.is_empty() {
                    return;
                }
                let target = a.typed_path(&format!("{dir}{name}"));
                let op = if folder || name.ends_with('/') || name.ends_with('\\') {
                    crate::services::io::FsOp::NewFolder(target)
                } else {
                    crate::services::io::FsOp::NewFile(target)
                };
                a.fs(op);
            }
            Ok(None) => {}
            Err(_) => editor::vim::open_cmdline(&mut a.ed, ':', &format!("{cmd} {dir}")),
        });
    }

    /// Asks for the new name of row `path`, in the same folder. Where no dialog can show, the
    /// command line asks.
    fn ask_rename_path(&mut self, path: &str, tx: &mut Tx) {
        let from = self.ed.root.join(path);
        let (dir, name) = match path.rsplit_once('/') {
            Some((d, n)) => (format!("{d}/"), n.to_string()),
            None => (String::new(), path.to_string()),
        };
        let what = if self.tree.index.children.contains_key(path) { "folder" } else { "file" };
        let dialog = Confirm::new(format!("Rename \"{name}\""), format!("A new name for the {what}. Open files follow it."))
            .button(OK, "Rename", Role::Default)
            .button("cancel", "Cancel", Role::Cancel);
        let rel = path.to_string();
        tx.prompt(dialog, name.clone(), move |a, typed, _| match typed {
            Ok(Some(new)) => {
                let new = new.trim().trim_end_matches(['/', '\\']);
                if new.is_empty() || new == name {
                    return;
                }
                let to = a.typed_path(&format!("{dir}{new}"));
                if to != from {
                    a.fs(crate::services::io::FsOp::Move { from, to });
                }
            }
            Ok(None) => {}
            Err(_) => editor::vim::open_cmdline(&mut a.ed, ':', &format!("Move {rel}")),
        });
    }

    /// Asks before row `path` goes from the disk. Where no dialog can show, the `y` key
    /// answers, as after `d`.
    fn ask_delete(&mut self, path: &str, tx: &mut Tx) {
        let full: PathBuf = self.ed.root.join(path);
        let name = path.rsplit_once('/').map_or(path, |(_, n)| n);
        let what = if self.tree.index.children.contains_key(path) {
            format!("The folder \"{path}\" and all in it go from the disk.")
        } else {
            format!("\"{path}\" goes from the disk.")
        };
        let dirty = if self.dirty_under(&full).is_empty() { "" } else { " Its unsaved changes go too." };
        let dialog = Confirm::new(format!("Delete \"{name}\"?"), format!("{what}{dirty} You cannot undo this."))
            .button(OK, "Delete", Role::Destructive)
            .button("cancel", "Cancel", Role::Cancel);
        let rel = path.to_string();
        tx.confirm(dialog, move |a, answer, _| match answer {
            Ok(Some(id)) if id == OK => a.delete(full, true),
            Ok(_) => {}
            Err(_) => {
                a.ed.message(format!("Delete \"{rel}\"? y to delete, any other key to keep it"));
                a.tree.confirm = Some(full);
            }
        });
    }
}

#[cfg(test)]
#[path = "tree_menu_tests.rs"]
mod tests;
