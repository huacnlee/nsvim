//! Line blame: each line of the active file shows who last changed it and when, after its
//! text. The git worker runs `git blame` on the buffer's text, so unsaved lines show as not
//! committed yet. `Git: toggle line blame` turns it on and off.

use std::path::PathBuf;
use std::time::Duration;

use super::App;
use super::fx::Tx;
use crate::services::{BlameLine, GitInfo};

/// The blame of one file.
#[derive(Clone, Debug, Default)]
pub struct DocBlame {
    pub lines: Vec<BlameLine>,
    /// The buffer version the lines are for.
    pub version: u64,
    /// The version last sent to the git worker, plus one. 0: ask again.
    pub asked: u64,
}

impl App {
    /// Shows or hides who last changed each line.
    pub fn toggle_blame(&mut self, tx: &mut Tx) {
        self.blame = !self.blame;
        self.ed.message(if self.blame { "line blame on" } else { "line blame off" });
        if self.blame {
            self.blame_stale();
            self.ask_blame(self.clock, tx);
        }
    }

    /// The branch and status changed: a commit or a checkout can change any line's author.
    pub(super) fn git_info(&mut self, info: GitInfo) {
        self.git = info;
        self.blame_stale();
    }

    /// Asks git again for every file.
    fn blame_stale(&mut self) {
        for m in self.meta.values_mut() {
            m.blame.asked = 0;
        }
    }

    pub(super) fn blame_done(&mut self, path: PathBuf, version: u64, lines: Vec<BlameLine>) {
        if let Some(m) = self.meta.get_mut(&path)
            && version >= m.blame.version
        {
            m.blame.lines = lines;
            m.blame.version = version;
        }
    }

    /// Asks git who changed each line of the active file, once it has rested for 300 ms after
    /// an edit, as `ask_signs` does.
    pub(super) fn ask_blame(&mut self, clock: f64, tx: &mut Tx) {
        if !self.blame || !self.ed.has_doc() || self.on_settings {
            return;
        }
        let doc = self.ed.doc();
        let Some(path) = doc.buf.path.clone() else { return };
        let version = doc.buf.version();
        let Some(m) = self.meta.get_mut(&path) else { return };
        if m.blame.asked == version + 1 {
            return;
        }
        if version != m.blame.version && clock - self.last_edit < 0.3 && version > 0 {
            tx.wake_after(Duration::from_secs_f64(0.32 - (clock - self.last_edit)));
            return;
        }
        m.blame.asked = version + 1;
        if doc.buf.len_bytes() > super::BIG {
            return;
        }
        if let Some(svc) = &self.svc {
            svc.git_blame(path, doc.buf.text(), version);
        }
    }
}
