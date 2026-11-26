//! Saving a buffer that has no file yet. The menu, the palette, `Cmd S` in the classic keymap
//! and the unsaved changes dialog show the system's save dialog for it, so a mouse user never
//! meets E32. The Vim commands keep Vim's way: `:w` with no name says E32, `:w NAME` names it.

use std::path::PathBuf;

use super::App;
use super::fx::Tx;
use super::unsaved::DocRef;
use crate::services::AfterSave;

/// What runs once every buffer has a file: the buffers, found again by their new files.
type Named = Box<dyn FnOnce(&mut App, Vec<DocRef>, &mut Tx)>;

impl App {
    /// Save from the menu or the palette: the active buffer, after the save dialog when it
    /// has no file. Cancel leaves it as it was.
    pub fn save_or_ask(&mut self, tx: &mut Tx) {
        if !self.ed.has_doc() || self.ed.buf().path.is_some() {
            return self.save_active(None, AfterSave::Nothing);
        }
        let r = self.doc_ref(self.ed.active);
        self.name_untitled(vec![r], tx, |a, refs, _| {
            if let Some(j) = refs.first().and_then(|r| a.find_doc(r)) {
                a.save_doc(j, AfterSave::Nothing);
            }
        });
    }

    /// Save All from the menu or the palette: names the modified buffers that have no file,
    /// one dialog after another, then writes every modified buffer.
    pub fn save_all_or_ask(&mut self, tx: &mut Tx) {
        let dirty: Vec<DocRef> = (0..self.ed.docs.len()).filter(|&i| self.ed.docs[i].buf.is_dirty()).map(|i| self.doc_ref(i)).collect();
        self.name_untitled(dirty, tx, |a, _, _| {
            a.save_all(false);
        });
    }

    /// Asks where to save each buffer of `refs` that has no file, one dialog after another,
    /// and gives it the path chosen. Then runs `done` with `refs`, which now find the buffers
    /// by their files. A cancel or a dialog that cannot show stops it: `done` does not run.
    pub(super) fn name_untitled(&mut self, refs: Vec<DocRef>, tx: &mut Tx, done: impl FnOnce(&mut App, Vec<DocRef>, &mut Tx) + 'static) {
        self.name_from(refs, 0, tx, Box::new(done));
    }

    fn name_from(&mut self, mut refs: Vec<DocRef>, from: usize, tx: &mut Tx, done: Named) {
        let Some(k) = (from..refs.len()).find(|&k| refs[k].path.is_none()) else {
            return done(self, refs, tx);
        };
        tx.choose_save_path(self.ed.root.clone(), move |a, answer, tx| match answer {
            Ok(Some(path)) => {
                let Some(j) = a.find_doc(&refs[k]) else { return };
                a.set_doc_path(j, path.clone());
                refs[k].path = Some(path);
                a.name_from(refs, k + 1, tx, done);
            }
            Ok(None) => {}
            Err(e) => a.ed.error(format!("could not show the save dialog: {e}. :w NAME writes the file")),
        });
    }

    /// Gives buffer `i` the file `path`, relative to the project unless absolute.
    pub(crate) fn set_doc_path(&mut self, i: usize, path: PathBuf) {
        let p = if path.is_absolute() { path } else { self.ed.root.join(path) };
        self.ed.docs[i].buf.path = Some(p.clone());
        self.meta.entry(p.clone()).or_default().lang = Some(crate::highlight::lang_for(Some(&p)));
    }

    /// A new empty buffer with no file (`:enew`, File › New File, `Cmd N` in the classic
    /// keymap).
    pub fn new_buffer(&mut self) {
        self.ed.docs.push(editor::editor::Doc::new(editor::buffer::Buffer::empty()));
        self.ed.active = self.ed.docs.len() - 1;
        self.ed.mode = self.ed.rest_mode();
        self.apply_doc_opts();
    }
}
