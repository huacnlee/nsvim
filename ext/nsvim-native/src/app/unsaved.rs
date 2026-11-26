//! The dialog that asks what to do with unsaved changes before files close: save them, drop
//! them, or keep the files open. The window's close button, Quit, the tab's close button, the
//! tab menu, the app menu and the classic keymap use it, so a mouse user needs no Vim command.
//! The Vim commands (`:q`, `:qa`, `:bd`) keep Vim's E37 message and `!`.

use std::path::PathBuf;

use super::App;
use super::fx::{Confirm, Role, Tx};
use crate::services::AfterSave;

/// The answer to the unsaved changes dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Save,
    Discard,
    Cancel,
}

/// The button ids of the dialog.
pub const SAVE: &str = "save";
pub const DISCARD: &str = "discard";
pub const CANCEL: &str = "cancel";

/// A buffer to find again when the answer comes: by its file, or for a buffer with no file, by
/// its place and name. Tabs can close or move while the dialog shows.
#[derive(Clone, Debug)]
struct DocRef {
    path: Option<PathBuf>,
    index: usize,
    name: String,
}

/// The dialog about the unsaved changes in the buffers `names`.
pub fn dialog(names: &[String]) -> Confirm {
    let (title, save) = match names {
        [one] => (format!("Save changes to \"{one}\"?"), "Save"),
        _ => (format!("Save changes to {} files?", names.len()), "Save all"),
    };
    let listed = match names {
        [] | [_] => String::new(),
        [a, b] => format!("\"{a}\" and \"{b}\" have unsaved changes. "),
        [a, rest @ ..] => format!("\"{a}\" and {} more have unsaved changes. ", rest.len()),
    };
    Confirm::new(title, format!("{listed}If you do not save, your changes are lost."))
        .button(SAVE, save, Role::Default)
        .button(DISCARD, "Don't save", Role::Destructive)
        .button(CANCEL, "Cancel", Role::Cancel)
}

impl App {
    /// The user's choice from the dialog's answer. A dialog that could not show keeps the
    /// files open and tells why.
    fn choice(&mut self, answer: Result<Option<String>, String>) -> Choice {
        match answer {
            Ok(Some(id)) if id == SAVE => Choice::Save,
            Ok(Some(id)) if id == DISCARD => Choice::Discard,
            Ok(_) => Choice::Cancel,
            Err(e) => {
                self.ed.error(format!("could not ask about the unsaved changes: {e}. :wa writes them, :qa! drops them"));
                Choice::Cancel
            }
        }
    }

    fn doc_ref(&self, i: usize) -> DocRef {
        let b = &self.ed.docs[i].buf;
        DocRef { path: b.path.clone(), index: i, name: b.name() }
    }

    fn find_doc(&self, r: &DocRef) -> Option<usize> {
        match &r.path {
            Some(p) => self.doc_index(p),
            None => self.ed.docs.get(r.index).filter(|d| d.buf.path.is_none() && d.buf.name() == r.name).map(|_| r.index),
        }
    }

    /// Asks about the unsaved changes in buffers `dirty`, then closes them as the user
    /// chose: saved and closed, closed without a save, or kept open.
    pub fn close_dirty(&mut self, dirty: &[usize], tx: &mut Tx) {
        let refs: Vec<DocRef> = dirty.iter().map(|&i| self.doc_ref(i)).collect();
        if refs.is_empty() {
            return;
        }
        let names: Vec<String> = refs.iter().map(|r| r.name.clone()).collect();
        tx.confirm(dialog(&names), move |a, answer, _| {
            let choice = a.choice(answer);
            if choice == Choice::Cancel {
                return;
            }
            let mut found: Vec<usize> = refs.iter().filter_map(|r| a.find_doc(r)).collect();
            found.sort_unstable();
            if choice == Choice::Save {
                // Each buffer closes when its write lands. A failed write keeps it open.
                for &j in &found {
                    a.save_doc(j, AfterSave::CloseTab);
                }
                return;
            }
            for &j in found.iter().rev() {
                a.drop_doc(j);
            }
            a.ed.mode = a.ed.rest_mode();
            a.apply_doc_opts();
        });
    }

    /// The window's close button or Quit, with unsaved changes: asks first. Save writes
    /// every modified buffer and quits after the last write. Don't save quits and drops them.
    pub(crate) fn ask_before_quit(&mut self, tx: &mut Tx) {
        if self.asking_quit {
            return;
        }
        self.asking_quit = true;
        tx.confirm(dialog(&self.dirty_names()), |a, answer, tx| {
            a.asking_quit = false;
            match a.choice(answer) {
                Choice::Save => a.save_all_and_quit(tx),
                Choice::Discard => a.quit_all(true, tx),
                Choice::Cancel => {}
            }
        });
    }
}
