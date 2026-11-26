//! The dialogs about a file whose disk copy and buffer differ: the file changed on disk while
//! the buffer has unsaved edits (Vim's W12), and a crash left unsaved edits in a swap file
//! (Vim's E325). A mouse user picks a button and needs no Vim command; `:e!` and `:w` still
//! work, and the message line still names them.

use std::path::{Path, PathBuf};

use super::App;
use super::fx::{Confirm, Role, Tx};

/// The button ids of the dialogs.
pub const RELOAD: &str = "reload";
pub const KEEP: &str = "keep";
pub const DISCARD: &str = "discard";

/// The dialog about `name`, changed on disk while the buffer has unsaved edits.
pub fn changed_dialog(name: &str) -> Confirm {
    Confirm::new(
        format!("\"{name}\" changed on disk"),
        "Another program changed this file while you have unsaved edits. Reload from disk drops your edits. \
         Keep mine leaves your edits, and the next save writes them over the disk copy.",
    )
    .button(RELOAD, "Reload from disk", Role::Destructive)
    .button(KEEP, "Keep mine", Role::Cancel)
}

/// The dialog about edits to `name` recovered from a crash.
pub fn recovered_dialog(name: &str) -> Confirm {
    Confirm::new(
        format!("Recover unsaved edits to \"{name}\"?"),
        "nsvim closed before these edits were saved. Keep them to go on where you left off, \
         or discard them to open the copy on disk.",
    )
    .button(KEEP, "Keep recovered edits", Role::Default)
    .button(DISCARD, "Discard", Role::Destructive)
}

impl App {
    /// Asks what to do about `path`, changed on disk while its buffer has unsaved edits. One
    /// dialog at a time per file: more changes while it shows wait for its answer.
    pub(super) fn ask_about_disk_change(&mut self, path: PathBuf, tx: &mut Tx) {
        if !self.asking_disk.insert(path.clone()) {
            return;
        }
        let name = self.rel(&path);
        tx.confirm(changed_dialog(&name), move |a, answer, _| {
            a.asking_disk.remove(&path);
            match answer {
                Ok(Some(id)) if id == RELOAD => a.reload_from_disk(&path),
                Ok(_) => {
                    if a.doc_index(&path).is_some() {
                        a.ed.message(format!("kept your edits to \"{name}\". A save writes them over the disk copy"));
                    }
                }
                Err(e) => a.ed.error(format!("could not ask about \"{name}\": {e}. :e! loads the disk copy, :w keeps yours")),
            }
        });
    }

    /// Asks whether to keep the edits to `path` recovered from its swap file. The buffer
    /// holds them until the answer; Discard loads the disk copy.
    pub(super) fn ask_about_recovered(&mut self, path: PathBuf, tx: &mut Tx) {
        let name = self.rel(&path);
        tx.confirm(recovered_dialog(&name), move |a, answer, _| match answer {
            Ok(Some(id)) if id == DISCARD => a.reload_from_disk(&path),
            Ok(_) => {
                if a.doc_index(&path).is_some() {
                    a.ed.message(format!("kept the recovered edits to \"{name}\". Save to write them"));
                }
            }
            Err(e) => a.ed.error(format!("could not ask about \"{name}\": {e}. :w keeps the recovered edits, :e! drops them")),
        });
    }

    /// Drops the unsaved edits of `path` and loads its disk copy, as `:e!` does.
    pub(super) fn reload_from_disk(&mut self, path: &Path) {
        let Some(i) = self.doc_index(path) else { return };
        let Some(svc) = &self.svc else { return };
        // Mark the buffer clean so the read applies as a reload, not a conflict.
        let v = self.ed.docs[i].buf.version();
        self.ed.docs[i].buf.mark_saved(v, None);
        svc.unswap(path.to_path_buf());
        svc.read_disk(path.to_path_buf());
        if let Some(m) = self.meta.get_mut(path) {
            m.conflict = false;
            m.swapped = 0;
        }
    }
}
