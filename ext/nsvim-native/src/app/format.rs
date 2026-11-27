//! Format Document and Format Selection through the language server, and format on save
//! (`format.on_save`). The LSP worker sends the request. A save waits for the reply for at most
//! `SAVE_WAIT` seconds, then writes the file as it is.

use std::path::PathBuf;
use std::time::Duration;

use editor::buffer::Pos;

use super::App;
use super::fx::Tx;
use crate::services::AfterSave;
use crate::services::lsp::{FormatJob, Job, TextEdit};

/// How long a save waits for the language server to format the file.
pub const SAVE_WAIT: f64 = 2.0;

/// A save that waits for the formatter.
pub struct FormatSave {
    pub path: PathBuf,
    pub version: u64,
    pub then: AfterSave,
    /// The clock time when the save stops waiting.
    pub until: f64,
}

impl App {
    /// Format Document, or with `selection` Format Selection.
    pub fn format_doc(&mut self, selection: bool) {
        let Some((path, _)) = self.here() else { return };
        let range = if selection {
            let Some((a, b)) = self.selection_chars() else {
                self.ed.message("select the text to format first");
                return;
            };
            self.leave_selection(None);
            Some((self.ed.buf().pos(a), self.ed.buf().pos(b)))
        } else {
            None
        };
        let i = self.ed.active;
        self.send_format(i, path, range, false);
    }

    /// `:Format`: the lines `first..=last` of a range, else the whole file.
    pub fn format_lines(&mut self, lines: Option<(usize, usize)>) {
        let Some((path, _)) = self.here() else { return };
        let range = lines.map(|(a, b)| {
            let buf = self.ed.buf();
            let end = if b < buf.last_line() { Pos::new(b + 1, 0) } else { Pos::new(b, buf.line_len(b)) };
            (Pos::new(a, 0), end)
        });
        let i = self.ed.active;
        self.send_format(i, path, range, false);
    }

    fn send_format(&mut self, i: usize, path: PathBuf, range: Option<(Pos, Pos)>, save: bool) {
        self.flush_lsp(&path);
        let (tab_size, spaces) = self.indent_of(i);
        let version = self.ed.docs[i].buf.version();
        if let Some(svc) = &self.svc {
            svc.lsp(Job::Format(FormatJob { path, version, range, tab_size, spaces, save }));
        }
    }

    /// The indent of document `i`: the editor's options for the active file (`:set` and
    /// `.editorconfig` included), else config.toml with the file's `.editorconfig` on top.
    fn indent_of(&self, i: usize) -> (usize, bool) {
        if i == self.ed.active {
            return (self.ed.opts.tab_width, self.ed.opts.spaces);
        }
        let o = self.cfg.opts();
        let ec = self.ed.docs[i].buf.path.as_ref().and_then(|p| self.meta.get(p)).map(|m| &m.ec);
        (ec.and_then(|e| e.indent_size).unwrap_or(o.tab_width), ec.and_then(|e| e.indent_spaces).unwrap_or(o.spaces))
    }

    /// With `format.on_save`, asks the server to format document `i` and writes it when the
    /// reply comes. `false`: the save goes on now.
    pub(super) fn format_before_save(&mut self, i: usize, then: AfterSave) -> bool {
        let doc = &self.ed.docs[i];
        let Some(path) = doc.buf.path.clone() else { return false };
        if !self.cfg.format.on_save || self.svc.is_none() || self.meta.get(&path).and_then(|m| m.lang).is_none() {
            return false;
        }
        let version = doc.buf.version();
        if self.format_saves.iter().any(|w| w.path == path) {
            // A save of this file already waits: it writes the newest text.
            return true;
        }
        self.format_saves.push(FormatSave { path: path.clone(), version, then, until: self.clock + SAVE_WAIT });
        self.send_format(i, path, None, true);
        true
    }

    /// Writes the files whose formatter did not answer in time, and wakes in time for the next.
    pub(super) fn format_timeouts(&mut self, tx: &mut Tx) {
        let now = self.clock;
        let (late, wait): (Vec<FormatSave>, Vec<FormatSave>) =
            std::mem::take(&mut self.format_saves).into_iter().partition(|w| w.until <= now);
        self.format_saves = wait;
        if let Some(next) = self.format_saves.iter().map(|w| w.until).reduce(f64::min) {
            tx.wake_after(Duration::from_secs_f64((next - now).max(0.01)));
        }
        for w in late {
            if let Some(i) = self.doc_index(&w.path) {
                self.write_doc(i, w.then);
            }
        }
    }

    /// The reply to a format request.
    pub fn formatted(&mut self, path: PathBuf, version: u64, edits: Vec<TextEdit>, save: bool, error: Option<String>) {
        let wait = if save {
            match self.format_saves.iter().position(|w| w.path == path && w.version == version) {
                Some(j) => Some(self.format_saves.remove(j)),
                // The save stopped waiting and wrote the file already.
                None => return,
            }
        } else {
            None
        };
        let Some(i) = self.doc_index(&path) else { return };
        let same = self.ed.docs[i].buf.version() == version;
        if same && !edits.is_empty() {
            self.apply_format(i, &edits);
        }
        if !save {
            match error {
                Some(e) => self.ed.message(e),
                None if !same => self.ed.message("the file changed while the language server formatted it"),
                None if edits.is_empty() => self.ed.message("the file is already formatted"),
                None => self.ed.message(format!("formatted: {} changes", edits.len())),
            }
        }
        if let Some(w) = wait {
            self.write_doc(i, w.then);
        }
    }

    /// Applies a format reply to document `i` as one undo step. The cursor stays on its line.
    fn apply_format(&mut self, i: usize, edits: &[TextEdit]) {
        let doc = &mut self.ed.docs[i];
        let cur = doc.view.cursor;
        let line = line_after(cur.line, edits);
        super::apply_edits(&mut doc.buf, edits);
        doc.view.cursor = doc.buf.clamp(Pos::new(line, cur.col), false);
        if let Some(a) = doc.view.anchor {
            doc.view.anchor = Some(doc.buf.clamp(a, false));
        }
        self.edited();
    }
}

/// Where line `line` is after `edits`: edits above it move it by the lines they add or remove.
/// An edit over the line keeps it inside the new text.
pub fn line_after(line: usize, edits: &[TextEdit]) -> usize {
    let mut out = line as isize;
    for e in edits {
        if e.start.line >= line {
            continue;
        }
        let added = e.text.matches('\n').count() as isize;
        let span = (e.end.line - e.start.line) as isize;
        let off = (line - e.start.line) as isize;
        out += if off >= span { added - span } else { off.min(added) - off };
    }
    out.max(0) as usize
}
