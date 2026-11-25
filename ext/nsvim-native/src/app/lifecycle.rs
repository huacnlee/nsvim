//! The editor's life around the text: the session it reopens, config.toml read again when it
//! changes on disk, and the way out when the user closes the window with unsaved edits.

use editor::buffer::Pos;

use super::fx::Tx;
use super::{App, Screen, session};
use crate::config::Config;

/// A second close within this many seconds closes the window over unsaved edits.
pub const CLOSE_AGAIN_SECS: f64 = 5.0;

impl App {
    /// Opens the files of the last session in this project, when no other file is to open.
    /// Reads a small file: call it at startup only.
    pub fn restore_session(&mut self) {
        if !self.to_open.is_empty() {
            return;
        }
        let Some(s) = session::load(&self.ed.root) else { return };
        self.restore_active = s.active.clone();
        self.tree.open.extend(s.tree_open);
        self.to_open = s.files.into_iter().map(|f| (f.path, Some(Pos::new(f.line, f.col)))).collect();
    }

    /// The names of the buffers with unsaved edits.
    pub fn dirty_names(&self) -> Vec<String> {
        self.ed.docs.iter().filter(|d| d.buf.is_dirty()).map(|d| d.buf.name()).collect()
    }

    /// The user asked to close the window or to quit the app. With no unsaved edits, the
    /// editor quits. With unsaved edits, the first ask shows a warning and keeps the window.
    /// A second ask within `CLOSE_AGAIN_SECS` quits. The edits then stay in swap files, and
    /// the next launch recovers them.
    pub fn close_requested(&mut self, tx: &mut Tx) {
        let dirty = self.dirty_names();
        if dirty.is_empty() {
            return self.quit(tx);
        }
        let again = self.close_asked.is_some_and(|t| self.clock - t <= CLOSE_AGAIN_SECS);
        if !again {
            self.close_asked = Some(self.clock);
            // `dirty` is not empty here.
            let what = match dirty.len() {
                1 => format!("\"{}\" has", dirty[0]),
                n => format!("\"{}\" and {} more have", dirty[0], n - 1),
            };
            self.ed.error(format!("E37: {what} unsaved changes. Close again to quit and keep them for recovery, :wqa writes them"));
            return;
        }
        for d in self.ed.docs.iter().filter(|d| d.buf.is_dirty()) {
            if let Some(p) = &d.buf.path {
                crate::services::io::write_swap(p.clone(), d.buf.text());
            }
        }
        self.quit(tx);
    }

    /// Writes the session, answers the `open --wait` requests and removes the control
    /// socket. Runs once, on the way out, however the editor ends.
    pub fn exit(&mut self) {
        if self.exited {
            return;
        }
        self.exited = true;
        session::save(self);
        self.release_waiters();
        if let Some(svc) = &self.svc {
            svc.close_socket();
        }
    }

    /// config.toml changed on disk, and the watcher read it again. The new config applies at
    /// once. Settings writes each change at once, so it has no changes of its own to keep.
    pub fn config_reloaded(&mut self, loaded: Result<Config, String>) {
        let cfg = match loaded {
            Ok(c) => c,
            Err(e) => return self.ed.error(e),
        };
        // The editor's own write, or a file the setup screen is about to write.
        if cfg == self.disk_cfg || matches!(self.screen, Screen::Setup(_)) {
            return;
        }
        self.cfg = cfg.clone();
        self.disk_cfg = cfg;
        self.config_changed();
        self.ed.message("config.toml changed on disk: settings applied");
    }
}
