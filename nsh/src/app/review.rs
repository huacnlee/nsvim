//! Reviewing what changes the text from outside: edits an agent or a git checkout made on disk
//! (marked in the gutter and listed under `SPC a`), and changes an agent proposes through the
//! control socket, shown as a diff to accept with `y` or reject with `n`.

use std::path::PathBuf;

use editor::buffer::Pos;
use editor::editor::{KeyCode, KeyInput};
use similar::{DiffOp, TextDiff};

use super::App;

/// One changed block of a file that changed on disk.
#[derive(Clone, Debug, PartialEq)]
pub struct Hunk {
    pub path: PathBuf,
    /// The first line of the block in the new text, zero-based.
    pub line: usize,
    pub added: usize,
    pub removed: usize,
    /// The block's first line, for the list.
    pub preview: String,
    /// When it happened, on the app clock.
    pub at: f64,
}

/// The first line of `lines[at..at + n]` with text in it, for the list.
fn first<'a>(lines: &[&'a str], at: usize, n: usize) -> &'a str {
    lines.iter().skip(at).take(n.max(1)).find(|l| !l.trim().is_empty()).copied().unwrap_or("")
}

/// The diff of `old` and `new` as hunks, with the new text's changed lines.
pub fn hunks(path: &std::path::Path, old: &str, new: &str, at: f64) -> (Vec<Hunk>, Vec<usize>) {
    let diff = TextDiff::configure().timeout(std::time::Duration::from_millis(200)).diff_lines(old, new);
    let new_lines: Vec<&str> = diff.new_slices().to_vec();
    let old_lines: Vec<&str> = diff.old_slices().to_vec();
    let mut out = Vec::new();
    let mut lines = Vec::new();
    for op in diff.ops() {
        let (line, added, removed, preview) = match *op {
            DiffOp::Equal { .. } => continue,
            DiffOp::Insert { new_index, new_len, .. } => (new_index, new_len, 0, first(&new_lines, new_index, new_len)),
            DiffOp::Delete { old_index, old_len, new_index } => (new_index, 0, old_len, first(&old_lines, old_index, old_len)),
            DiffOp::Replace { old_len, new_index, new_len, .. } => (new_index, new_len, old_len, first(&new_lines, new_index, new_len)),
        };
        lines.extend(line..line + added.max(1));
        out.push(Hunk { path: path.to_path_buf(), line, added, removed, preview: preview.trim().chars().take(120).collect(), at });
    }
    (out, lines)
}

/// A row of a proposal's diff view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Same {
        new: usize,
        text: String,
    },
    Added {
        new: usize,
        text: String,
    },
    Removed {
        text: String,
    },
    /// Unchanged lines folded away.
    Fold(usize),
}

/// A change an agent proposed: the whole new text, and the diff against the buffer.
#[derive(Clone, Debug)]
pub struct Proposal {
    pub path: PathBuf,
    pub label: String,
    pub text: String,
    /// The buffer version the diff was made against.
    pub base: u64,
    pub rows: Vec<Row>,
    /// The row each changed block starts at.
    pub starts: Vec<usize>,
    /// The first row on screen.
    pub top: usize,
    pub added: usize,
    pub removed: usize,
}

/// Unchanged lines kept around each change in the diff view.
const CONTEXT: usize = 3;

impl Proposal {
    pub fn new(path: PathBuf, label: String, old: &str, text: String, base: u64) -> Proposal {
        let mut p = Proposal { path, label, text, base, rows: Vec::new(), starts: Vec::new(), top: 0, added: 0, removed: 0 };
        p.rediff(old, base);
        p
    }

    /// Diffs the proposed text against `old`, the buffer at `version`.
    pub fn rediff(&mut self, old: &str, version: u64) {
        let diff = TextDiff::configure().timeout(std::time::Duration::from_millis(500)).diff_lines(old, &self.text);
        let olds = diff.old_slices();
        let news = diff.new_slices();
        let line = |s: &str| s.trim_end_matches('\n').to_string();
        let mut rows = Vec::new();
        let mut starts = Vec::new();
        let ops = diff.ops();
        let (mut added, mut removed) = (0, 0);
        for (i, op) in ops.iter().enumerate() {
            match *op {
                DiffOp::Equal { new_index, len, .. } => {
                    let keep_head = if i > 0 { CONTEXT.min(len) } else { 0 };
                    let keep_tail = if i + 1 < ops.len() { CONTEXT.min(len) } else { 0 };
                    if keep_head + keep_tail + 1 >= len {
                        rows.extend((new_index..new_index + len).map(|n| Row::Same { new: n, text: line(news[n]) }));
                    } else {
                        rows.extend((new_index..new_index + keep_head).map(|n| Row::Same { new: n, text: line(news[n]) }));
                        rows.push(Row::Fold(len - keep_head - keep_tail));
                        rows.extend((new_index + len - keep_tail..new_index + len).map(|n| Row::Same { new: n, text: line(news[n]) }));
                    }
                }
                DiffOp::Delete { old_index, old_len, .. } => {
                    starts.push(rows.len());
                    removed += old_len;
                    rows.extend((old_index..old_index + old_len).map(|o| Row::Removed { text: line(olds[o]) }));
                }
                DiffOp::Insert { new_index, new_len, .. } => {
                    starts.push(rows.len());
                    added += new_len;
                    rows.extend((new_index..new_index + new_len).map(|n| Row::Added { new: n, text: line(news[n]) }));
                }
                DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                    starts.push(rows.len());
                    removed += old_len;
                    added += new_len;
                    rows.extend((old_index..old_index + old_len).map(|o| Row::Removed { text: line(olds[o]) }));
                    rows.extend((new_index..new_index + new_len).map(|n| Row::Added { new: n, text: line(news[n]) }));
                }
            }
        }
        self.rows = rows;
        self.top = starts.first().map_or(0, |&s| s.saturating_sub(CONTEXT));
        self.starts = starts;
        self.base = version;
        self.added = added;
        self.removed = removed;
    }

    /// The char-range edits that turn `old` into the proposed text, last first.
    pub fn edits(&self, old: &str) -> Vec<(usize, usize, String)> {
        let diff = TextDiff::configure().timeout(std::time::Duration::from_millis(500)).diff_lines(old, &self.text);
        let olds = diff.old_slices();
        let news = diff.new_slices();
        // Char offset where each old line starts, plus the end.
        let mut at = Vec::with_capacity(olds.len() + 1);
        let mut n = 0;
        for l in olds {
            at.push(n);
            n += l.chars().count();
        }
        at.push(n);
        let mut out = Vec::new();
        for op in diff.ops() {
            let (oi, ol, ni, nl) = match *op {
                DiffOp::Equal { .. } => continue,
                DiffOp::Delete { old_index, old_len, .. } => (old_index, old_len, 0, 0),
                DiffOp::Insert { old_index, new_index, new_len } => (old_index, 0, new_index, new_len),
                DiffOp::Replace { old_index, old_len, new_index, new_len } => (old_index, old_len, new_index, new_len),
            };
            out.push((at[oi], at[oi + ol], news[ni..ni + nl].concat()));
        }
        out.reverse();
        out
    }
}

impl App {
    /// Records the outside change to `path` from `old` to `new`, marks it in the gutter and
    /// returns its line counts.
    pub fn record_outside(&mut self, path: &std::path::Path, old: &str, new: &str) -> (usize, usize) {
        let (found, lines) = hunks(path, old, new, self.clock);
        let added = found.iter().map(|h| h.added).sum();
        let removed = found.iter().map(|h| h.removed).sum();
        self.outside.retain(|h| h.path != path);
        for h in found.into_iter().rev() {
            self.outside.insert(0, h);
        }
        self.outside.truncate(500);
        if let Some(m) = self.meta.get_mut(path) {
            m.outside = lines;
        }
        (added, removed)
    }

    /// Clears the outside-change marks of `path` (after a save, or `SPC a c`).
    pub fn clear_outside(&mut self, path: Option<&std::path::Path>) {
        match path {
            Some(p) => {
                self.outside.retain(|h| h.path != p);
                if let Some(m) = self.meta.get_mut(p) {
                    m.outside.clear();
                }
            }
            None => {
                self.outside.clear();
                for m in self.meta.values_mut() {
                    m.outside.clear();
                }
            }
        }
    }

    /// Lists the outside changes in the picker, newest first.
    pub fn show_outside(&mut self) {
        if self.outside.is_empty() {
            self.ed.message("no changes from outside the editor");
            return;
        }
        let locs = self
            .outside
            .iter()
            .map(|h| crate::services::lsp::Loc {
                path: h.path.clone(),
                pos: Pos::new(h.line, 0),
                text: format!("+{} -{}  {}", h.added, h.removed, h.preview),
            })
            .collect();
        self.show_locations("changes", locs);
    }

    /// Opens the proposal's file and shows the diff, or queues it behind the one showing.
    pub fn propose(&mut self, p: Proposal) {
        self.open_path(p.path.clone(), None);
        self.proposals.push(p);
        self.ed.message(self.proposal_hint());
    }

    /// The proposal showing now: the first one whose file is the active buffer.
    pub fn proposal_here(&self) -> Option<usize> {
        if !self.ed.has_doc() || self.on_settings {
            return None;
        }
        let path = self.ed.buf().path.as_deref()?;
        self.proposals.iter().position(|p| p.path == path)
    }

    fn proposal_hint(&self) -> String {
        match self.proposals.first() {
            Some(p) => format!("agent proposes: {} (+{} -{}). y accepts, n rejects, ]c next change", p.label, p.added, p.removed),
            None => String::new(),
        }
    }

    /// Keys while a proposal shows. True when the key was used.
    pub fn proposal_key(&mut self, k: KeyInput) -> bool {
        let Some(i) = self.proposal_here() else { return false };
        let rows = self.ed.rows.max(1);
        let p = &mut self.proposals[i];
        let last = p.rows.len().saturating_sub(1);
        match (k.code, k.ctrl) {
            (KeyCode::Char('y'), false) | (KeyCode::Enter, _) => {
                self.accept_proposal(i);
                return true;
            }
            (KeyCode::Char('n'), false) | (KeyCode::Esc, _) => {
                let p = self.proposals.remove(i);
                self.ed.message(format!("rejected: {}", p.label));
                return true;
            }
            (KeyCode::Char('j') | KeyCode::Down, false) => p.top = (p.top + 1).min(last),
            (KeyCode::Char('k') | KeyCode::Up, false) => p.top = p.top.saturating_sub(1),
            (KeyCode::Char('d'), true) | (KeyCode::PageDown, _) => p.top = (p.top + rows / 2).min(last),
            (KeyCode::Char('u'), true) | (KeyCode::PageUp, _) => p.top = p.top.saturating_sub(rows / 2),
            (KeyCode::Char('g'), false) => p.top = 0,
            (KeyCode::Char('G'), false) => p.top = last.saturating_sub(rows.saturating_sub(1)),
            (KeyCode::Char(']'), false) | (KeyCode::Tab, _) => {
                if let Some(&s) = p.starts.iter().find(|&&s| s > p.top + CONTEXT) {
                    p.top = s.saturating_sub(CONTEXT);
                }
            }
            (KeyCode::Char('['), false) => {
                if let Some(&s) = p.starts.iter().rev().find(|&&s| s + CONTEXT < p.top + CONTEXT) {
                    p.top = s.saturating_sub(CONTEXT);
                }
            }
            (KeyCode::Char('c'), false) => {}
            _ => {
                self.ed.message(self.proposal_hint());
                return true;
            }
        }
        true
    }

    /// A file changed on disk: reload a clean buffer and mark the changed lines, or warn when
    /// the buffer has edits of its own.
    pub(super) fn disk_text(&mut self, path: PathBuf, text: String, mtime: Option<std::time::SystemTime>) {
        let Some(i) = self.doc_index(&path) else { return };
        let dirty = self.ed.docs[i].buf.is_dirty();
        let same = self.ed.docs[i].buf.text().replace("\r\n", "\n") == text.replace("\r\n", "\n");
        if same {
            self.ed.docs[i].buf.disk_mtime = mtime;
            return;
        }
        // The editor's own save, read back after more typing: the disk holds what the buffer
        // saved last, so nothing changed from outside.
        if dirty && mtime.is_some() && mtime == self.ed.docs[i].buf.disk_mtime {
            return;
        }
        let name = self.rel(&path);
        if dirty {
            if let Some(m) = self.meta.get_mut(&path) {
                m.conflict = true;
            }
            self.ed.docs[i].buf.disk_mtime = mtime;
            self.ed.error(format!("W12: \"{name}\" changed on disk and here. :e! loads the disk copy, :w keeps yours"));
            return;
        }
        let cursor = self.ed.docs[i].view.cursor;
        let old = self.ed.docs[i].buf.text();
        self.ed.docs[i].buf.replace_from_disk(&text, cursor, mtime);
        let doc = &mut self.ed.docs[i];
        doc.view.cursor = doc.buf.clamp(cursor, false);
        let new = doc.buf.text();
        let (added, removed) = self.record_outside(&path, &old, &new);
        let lead = if self.is_vim() { "SPC a l" } else { "the palette" };
        self.ed.message(format!("\"{name}\" changed outside: +{added} -{removed} lines. {lead} lists changes, u undoes the reload"));
    }

    /// Applies proposal `i` as one undo step. When the buffer changed since the diff, the
    /// diff is made again and shown, to review before accepting.
    fn accept_proposal(&mut self, i: usize) {
        let Some(d) = self.doc_index(&self.proposals[i].path.clone()) else { return };
        let old = self.ed.docs[d].buf.text();
        let version = self.ed.docs[d].buf.version();
        if version != self.proposals[i].base {
            self.proposals[i].rediff(&old, version);
            self.ed.error("the file changed since the proposal: review the new diff, then y or n");
            return;
        }
        let p = self.proposals.remove(i);
        let edits = p.edits(&old);
        let doc = &mut self.ed.docs[d];
        let at = doc.view.cursor;
        doc.buf.begin(at);
        for (a, b, text) in &edits {
            doc.buf.replace(*a, *b, text);
        }
        doc.buf.commit(at);
        doc.view.cursor = doc.buf.clamp(at, false);
        self.edited();
        self.ed.message(format!("accepted: {} (+{} -{}). u undoes it", p.label, p.added, p.removed));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_proposal_turns_the_old_text_into_the_new_one() {
        let old = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n";
        let new = "a\nB\nc\nd\ne\nf\ng\nh\ni\nj\nk\n";
        let p = Proposal::new("/x".into(), "edit".into(), old, new.into(), 1);
        assert_eq!((p.added, p.removed), (2, 1));
        let mut text = old.to_string();
        for (a, b, t) in p.edits(old) {
            let (ab, bb) = (text.char_indices().nth(a).map_or(text.len(), |c| c.0), text.char_indices().nth(b).map_or(text.len(), |c| c.0));
            text.replace_range(ab..bb, &t);
        }
        assert_eq!(text, new);
        assert!(p.rows.iter().any(|r| matches!(r, Row::Fold(_))), "long unchanged runs fold");
    }

    #[test]
    fn hunks_mark_the_changed_lines() {
        let (h, lines) = hunks(std::path::Path::new("/x"), "a\nb\nc\n", "a\nB\nc\nd\n", 0.0);
        assert_eq!(h.len(), 2);
        assert_eq!(lines, vec![1, 3]);
    }
}
