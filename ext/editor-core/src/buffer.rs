//! The text of one file: a rope, its path, its line endings and its undo history.
//!
//! Positions are zero-based lines and char columns. The rope always holds `\n` line ends.
//! A file with CRLF line ends is converted on load and written back with CRLF on save.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ropey::Rope;
use serde::{Deserialize, Serialize};

/// A position in a buffer: zero-based line and char column (not counting the line end).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

impl Pos {
    pub const fn new(line: usize, col: usize) -> Pos {
        Pos { line, col }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineEnding {
    #[default]
    Lf,
    CrLf,
}

impl LineEnding {
    pub fn label(self) -> &'static str {
        match self {
            LineEnding::Lf => "unix",
            LineEnding::CrLf => "dos",
        }
    }
}

/// What `save` does to the text on the way to disk.
#[derive(Clone, Copy, Debug)]
pub struct SaveOpts {
    pub trim_trailing_whitespace: bool,
    pub final_newline: bool,
}

/// One change: `removed` was at char index `at`, and `inserted` took its place.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Edit {
    at: usize,
    removed: String,
    inserted: String,
}

/// Edits that undo and redo as one step, with the cursor before and after them.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Txn {
    /// Names the text state this step leads to, for telling whether the buffer is saved.
    /// Histories stored before ids existed load with 0 and get an id then.
    #[serde(default)]
    id: u64,
    edits: Vec<Edit>,
    before: Pos,
    after: Pos,
}

/// A buffer's undo and redo steps and the hash of the text they lead to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct History {
    pub hash: u64,
    undo: Vec<Txn>,
    redo: Vec<Txn>,
}

pub struct Buffer {
    text: Rope,
    pub path: Option<PathBuf>,
    pub line_ending: LineEnding,
    version: u64,
    saved_version: u64,
    /// The undo step the file on disk matches (`state`), when known. Undo back to it, and the
    /// buffer is clean again.
    saved_state: Option<u64>,
    /// The id the next undo step gets.
    next_id: u64,
    undo: Vec<Txn>,
    redo: Vec<Txn>,
    open: Option<Txn>,
    /// The file's modification time when it was read or written, to tell outside changes apart.
    pub disk_mtime: Option<std::time::SystemTime>,
    /// Named marks as char indices. Every edit, undo and redo moves them with the text.
    marks: std::collections::BTreeMap<char, usize>,
}

/// Undo keeps at most this many steps per buffer.
const UNDO_LIMIT: usize = 1000;

impl Buffer {
    pub fn empty() -> Buffer {
        Buffer::from_str("")
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Buffer {
        let (text, line_ending) = normalize(s);
        Buffer {
            text: Rope::from_str(&text),
            path: None,
            line_ending,
            version: 0,
            saved_version: 0,
            saved_state: Some(0),
            next_id: 1,
            undo: Vec::new(),
            redo: Vec::new(),
            open: None,
            disk_mtime: None,
            marks: std::collections::BTreeMap::new(),
        }
    }

    /// Reads `path`. A missing file gives an empty buffer that saves to that path.
    pub fn open(path: &Path) -> io::Result<Buffer> {
        let mut buf = match fs::read(path) {
            Ok(bytes) => Buffer::from_str(&String::from_utf8_lossy(&bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Buffer::empty(),
            Err(e) => return Err(e),
        };
        buf.path = Some(path.to_path_buf());
        buf.disk_mtime = fs::metadata(path).and_then(|m| m.modified()).ok();
        Ok(buf)
    }

    /// Replaces the text with what is on disk now, as a reload after an outside change. The
    /// reload is one undo step, so `u` brings the old text back.
    pub fn reload(&mut self, cursor: Pos) -> io::Result<()> {
        let Some(path) = self.path.clone() else { return Ok(()) };
        let bytes = fs::read(&path)?;
        let (text, ending) = normalize(&String::from_utf8_lossy(&bytes));
        let old = self.text.to_string();
        if old != text {
            self.begin(cursor);
            let len = self.text.len_chars();
            self.apply(Edit { at: 0, removed: old, inserted: String::new() }, len);
            self.apply(Edit { at: 0, removed: String::new(), inserted: text }, 0);
            self.commit(cursor);
        }
        self.line_ending = ending;
        self.saved_version = self.version;
        self.saved_state = Some(self.state());
        self.disk_mtime = fs::metadata(&path).and_then(|m| m.modified()).ok();
        Ok(())
    }

    /// Writes the buffer to its path and returns the line and byte counts written.
    pub fn save(&mut self, opts: SaveOpts) -> io::Result<(usize, usize)> {
        let (path, bytes, version) = self.prepare_save(opts)?;
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir)?;
        }
        fs::write(&path, &bytes)?;
        self.mark_saved(version, fs::metadata(&path).and_then(|m| m.modified()).ok());
        Ok((self.line_count(), bytes.len()))
    }

    /// Applies `opts` to the text (as one undo step) and returns what a save writes: the path,
    /// the bytes and the version they are. The app writes them off the UI thread, then calls
    /// [`Self::mark_saved`].
    pub fn prepare_save(&mut self, opts: SaveOpts) -> io::Result<(PathBuf, Vec<u8>, u64)> {
        self.prepare_save_at(opts, Pos::default())
    }

    /// [`Self::prepare_save`], with the cursor an undo of the trim returns to.
    pub fn prepare_save_at(&mut self, opts: SaveOpts, cursor: Pos) -> io::Result<(PathBuf, Vec<u8>, u64)> {
        let Some(path) = self.path.clone() else {
            return Err(io::Error::other("E32: No file name"));
        };
        // Trim line by line, last line first, so each edit is small and earlier indices hold.
        if opts.trim_trailing_whitespace {
            for l in (0..self.line_count()).rev() {
                let line = self.text.line(l);
                let len = self.line_len(l);
                let mut keep = len;
                while keep > 0 && matches!(line.char(keep - 1), ' ' | '\t') {
                    keep -= 1;
                }
                if keep < len {
                    let start = self.text.line_to_char(l);
                    self.begin(cursor);
                    self.remove(start + keep, start + len);
                }
            }
        }
        let n = self.text.len_chars();
        if opts.final_newline && n > 0 && self.text.char(n - 1) != '\n' {
            self.begin(cursor);
            self.insert_at(n, "\n");
        }
        self.commit(cursor);
        let out = self.text.to_string();
        let disk = match self.line_ending {
            LineEnding::Lf => out,
            LineEnding::CrLf => out.replace('\n', "\r\n"),
        };
        Ok((path, disk.into_bytes(), self.version))
    }

    /// Records that `version` is what the file on disk holds now.
    pub fn mark_saved(&mut self, version: u64, mtime: Option<std::time::SystemTime>) {
        self.saved_version = version;
        // Edits made while the file was written leave the saved state unknown.
        self.saved_state = (version == self.version && self.state() != u64::MAX).then(|| self.state());
        self.disk_mtime = mtime;
    }

    /// Replaces the text with `text` read from disk, as one undo step, and marks it saved.
    pub fn replace_from_disk(&mut self, text: &str, cursor: Pos, mtime: Option<std::time::SystemTime>) {
        let (text, ending) = normalize(text);
        let old = self.text.to_string();
        if old != text {
            self.begin(cursor);
            let len = self.text.len_chars();
            self.apply(Edit { at: 0, removed: old, inserted: String::new() }, len);
            self.apply(Edit { at: 0, removed: String::new(), inserted: text }, 0);
            self.commit(cursor);
        }
        self.line_ending = ending;
        self.saved_version = self.version;
        self.saved_state = Some(self.state());
        self.disk_mtime = mtime;
    }

    pub fn rope(&self) -> &Rope {
        &self.text
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    /// The buffer differs from the file: an edit since the save, unless undo or redo came back
    /// to the saved step.
    pub fn is_dirty(&self) -> bool {
        self.version != self.saved_version && self.saved_state != Some(self.state())
    }

    /// The undo step the text is at now: 0 before any, `u64::MAX` inside an unfinished step.
    fn state(&self) -> u64 {
        if self.open.as_ref().is_some_and(|t| !t.edits.is_empty()) {
            return u64::MAX;
        }
        self.undo.last().map_or(0, |t| t.id)
    }

    pub fn len_chars(&self) -> usize {
        self.text.len_chars()
    }

    pub fn len_bytes(&self) -> usize {
        self.text.len_bytes()
    }

    /// Lines, counting an empty last line after a final `\n` as Vim does not: a file
    /// `"a\nb\n"` has 2 lines.
    pub fn line_count(&self) -> usize {
        let n = self.text.len_lines();
        if n > 1 && self.text.len_chars() > 0 && self.text.char(self.text.len_chars() - 1) == '\n' { n - 1 } else { n }
    }

    pub fn last_line(&self) -> usize {
        self.line_count().saturating_sub(1)
    }

    /// The text of `line` without its line end. Empty past the last line.
    pub fn line(&self, line: usize) -> String {
        if line >= self.text.len_lines() {
            return String::new();
        }
        let mut s = self.text.line(line).to_string();
        if s.ends_with('\n') {
            s.pop();
        }
        s
    }

    /// The chars of `line` without its line end, in `out`, which is cleared first. A loop over
    /// many lines reuses one `out`, with no new string per line.
    pub fn line_chars(&self, line: usize, out: &mut Vec<char>) {
        out.clear();
        if line >= self.text.len_lines() {
            return;
        }
        out.extend(self.text.line(line).chars());
        if out.last() == Some(&'\n') {
            out.pop();
        }
    }

    /// Chars in `line`, not counting its line end.
    pub fn line_len(&self, line: usize) -> usize {
        if line >= self.text.len_lines() {
            return 0;
        }
        let l = self.text.line(line);
        let n = l.len_chars();
        if n > 0 && l.char(n - 1) == '\n' { n - 1 } else { n }
    }

    /// The char at `p`, or `None` at or past the line end.
    pub fn char_at(&self, p: Pos) -> Option<char> {
        if p.line >= self.line_count() || p.col >= self.line_len(p.line) {
            return None;
        }
        Some(self.text.char(self.text.line_to_char(p.line) + p.col))
    }

    /// The char index of `p`, clamped to the text.
    pub fn idx(&self, p: Pos) -> usize {
        let line = p.line.min(self.text.len_lines().saturating_sub(1));
        let start = self.text.line_to_char(line);
        (start + p.col.min(self.line_len(line))).min(self.text.len_chars())
    }

    /// The position of char index `idx`.
    pub fn pos(&self, idx: usize) -> Pos {
        let idx = idx.min(self.text.len_chars());
        let line = self.text.char_to_line(idx);
        Pos { line, col: idx - self.text.line_to_char(line) }
    }

    /// The char at index `idx`, `\n` included.
    pub fn char_idx(&self, idx: usize) -> Option<char> {
        (idx < self.text.len_chars()).then(|| self.text.char(idx))
    }

    /// The text from char index `a` to `b`.
    pub fn slice(&self, a: usize, b: usize) -> String {
        let (a, b) = (a.min(b), a.max(b));
        self.text.slice(a.min(self.len_chars())..b.min(self.len_chars())).to_string()
    }

    /// `p` moved onto the text: a real line, and a column at most the line's length, or one
    /// less when `past_end` is false (Normal mode rests on a char).
    pub fn clamp(&self, p: Pos, past_end: bool) -> Pos {
        let line = p.line.min(self.last_line());
        let len = self.line_len(line);
        let max = if past_end { len } else { len.saturating_sub(1) };
        Pos { line, col: p.col.min(max) }
    }

    /// The first non-blank column of `line`.
    pub fn first_non_blank(&self, line: usize) -> usize {
        self.line(line).chars().take_while(|c| *c == ' ' || *c == '\t').count()
    }

    /// The leading whitespace of `line`.
    pub fn indent_of(&self, line: usize) -> String {
        self.line(line).chars().take_while(|c| *c == ' ' || *c == '\t').collect()
    }

    // Editing. Every edit goes through `apply`, inside a transaction.

    /// Starts a transaction, if none is open, with the cursor before it.
    pub fn begin(&mut self, cursor: Pos) {
        if self.open.is_none() {
            self.open = Some(Txn { id: 0, edits: Vec::new(), before: cursor, after: cursor });
        }
    }

    /// Ends the open transaction with the cursor after it. An empty one leaves no undo step.
    pub fn commit(&mut self, cursor: Pos) {
        if let Some(mut t) = self.open.take()
            && !t.edits.is_empty()
        {
            t.after = cursor;
            t.id = self.take_id();
            self.undo.push(t);
            if self.undo.len() > UNDO_LIMIT {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
    }

    fn take_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id - 1
    }

    pub fn in_txn(&self) -> bool {
        self.open.is_some()
    }

    fn apply(&mut self, e: Edit, remove_len: usize) {
        self.shift_marks(e.at, remove_len, e.inserted.chars().count());
        if remove_len > 0 {
            self.text.remove(e.at..e.at + remove_len);
        }
        if !e.inserted.is_empty() {
            self.text.insert(e.at, &e.inserted);
        }
        self.version += 1;
        if let Some(t) = &mut self.open {
            t.edits.push(e);
        } else {
            let id = self.take_id();
            self.undo.push(Txn { id, edits: vec![e], before: Pos::default(), after: Pos::default() });
            self.redo.clear();
        }
    }

    /// Inserts `s` at char index `at` and returns the index after it.
    pub fn insert_at(&mut self, at: usize, s: &str) -> usize {
        if s.is_empty() {
            return at;
        }
        let at = at.min(self.len_chars());
        let s = normalize(s).0;
        let n = s.chars().count();
        self.apply(Edit { at, removed: String::new(), inserted: s }, 0);
        at + n
    }

    /// Inserts `s` at `p` and returns the position after it.
    pub fn insert(&mut self, p: Pos, s: &str) -> Pos {
        let at = self.idx(p);
        let end = self.insert_at(at, s);
        self.pos(end)
    }

    /// Removes chars `a..b` and returns them.
    pub fn remove(&mut self, a: usize, b: usize) -> String {
        let (a, b) = (a.min(b), a.max(b).min(self.len_chars()));
        if a >= b {
            return String::new();
        }
        let removed = self.text.slice(a..b).to_string();
        self.apply(Edit { at: a, removed: removed.clone(), inserted: String::new() }, b - a);
        removed
    }

    /// Replaces chars `a..b` with `s`, and returns the index after the new text.
    pub fn replace(&mut self, a: usize, b: usize, s: &str) -> usize {
        self.remove(a, b);
        self.insert_at(a.min(b), s)
    }

    /// Undoes one step and returns where the cursor goes.
    pub fn undo(&mut self) -> Option<Pos> {
        self.commit(Pos::default());
        let t = self.undo.pop()?;
        for e in t.edits.iter().rev() {
            let n = e.inserted.chars().count();
            self.shift_marks(e.at, n, e.removed.chars().count());
            if n > 0 {
                self.text.remove(e.at..e.at + n);
            }
            if !e.removed.is_empty() {
                self.text.insert(e.at, &e.removed);
            }
        }
        self.version += 1;
        let at = t.before;
        self.redo.push(t);
        Some(at)
    }

    /// Redoes one step and returns where the cursor goes.
    pub fn redo(&mut self) -> Option<Pos> {
        let t = self.redo.pop()?;
        let edits = t.edits.clone();
        for e in &edits {
            let n = e.removed.chars().count();
            self.shift_marks(e.at, n, e.inserted.chars().count());
            if n > 0 {
                self.text.remove(e.at..e.at + n);
            }
            if !e.inserted.is_empty() {
                self.text.insert(e.at, &e.inserted);
            }
        }
        self.version += 1;
        let at = t.after;
        self.undo.push(t);
        Some(at)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty() || self.open.as_ref().is_some_and(|t| !t.edits.is_empty())
    }

    /// Sets mark `name` at `p`.
    pub fn set_mark(&mut self, name: char, p: Pos) {
        let i = self.idx(p);
        self.marks.insert(name, i);
    }

    /// Where mark `name` is now, after the edits since it was set.
    pub fn mark(&self, name: char) -> Option<Pos> {
        self.marks.get(&name).map(|&i| self.pos(i))
    }

    /// Every mark, by name.
    pub fn marks(&self) -> impl Iterator<Item = (char, Pos)> + '_ {
        self.marks.iter().map(|(&c, &i)| (c, self.pos(i)))
    }

    pub fn clear_mark(&mut self, name: char) {
        self.marks.remove(&name);
    }

    /// Moves marks for `removed` chars at `at` replaced by `inserted` chars. A mark inside the
    /// removed text moves to its start.
    fn shift_marks(&mut self, at: usize, removed: usize, inserted: usize) {
        for i in self.marks.values_mut() {
            if *i >= at + removed {
                *i = *i - removed + inserted;
            } else if *i > at {
                *i = at;
            }
        }
    }

    /// A hash of the text, which a stored undo history must match to apply.
    pub fn text_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for chunk in self.text.chunks() {
            chunk.hash(&mut h);
        }
        h.finish()
    }

    /// The undo history, to keep across restarts.
    pub fn history(&self) -> History {
        History { hash: self.text_hash(), undo: self.undo.clone(), redo: self.redo.clone() }
    }

    /// Takes back a stored history when it was made for this exact text.
    pub fn restore_history(&mut self, h: History) -> bool {
        if h.hash != self.text_hash() || !self.undo.is_empty() {
            return false;
        }
        self.undo = h.undo;
        self.redo = h.redo;
        // Steps from before ids existed get one now, and new steps never reuse one.
        self.next_id = self.undo.iter().chain(&self.redo).map(|t| t.id).max().unwrap_or(0) + 1;
        for t in self.undo.iter_mut().chain(self.redo.iter_mut()).filter(|t| t.id == 0) {
            t.id = self.next_id;
            self.next_id += 1;
        }
        // The text is the file's: the restored top step is the saved one.
        self.saved_state = Some(self.state());
        true
    }

    /// Replaces the whole text as one undo step that leaves the buffer modified, as a crash
    /// recovery does: the file on disk keeps its text until the user saves.
    pub fn recover(&mut self, text: &str) {
        let (text, _) = normalize(text);
        let old = self.text.to_string();
        if old == text {
            return;
        }
        self.begin(Pos::default());
        let len = self.text.len_chars();
        self.apply(Edit { at: 0, removed: old, inserted: String::new() }, len);
        self.apply(Edit { at: 0, removed: String::new(), inserted: text }, 0);
        self.commit(Pos::default());
    }

    /// The whole text, for small buffers and tests.
    pub fn text(&self) -> String {
        self.text.to_string()
    }

    /// The file's name, or `[No Name]`.
    pub fn name(&self) -> String {
        self.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "[No Name]".into())
    }
}

/// `s` with `\r\n` and lone `\r` turned into `\n`, and the line ending it mostly used.
fn normalize(s: &str) -> (String, LineEnding) {
    if !s.contains('\r') {
        return (s.to_string(), LineEnding::Lf);
    }
    let crlf = s.matches("\r\n").count();
    let lf = s.matches('\n').count() - crlf;
    let text = s.replace("\r\n", "\n").replace('\r', "\n");
    (text, if crlf > lf { LineEnding::CrLf } else { LineEnding::Lf })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_do_not_count_the_empty_line_after_a_final_newline() {
        assert_eq!(Buffer::from_str("a\nb\n").line_count(), 2);
        assert_eq!(Buffer::from_str("a\nb").line_count(), 2);
        assert_eq!(Buffer::from_str("").line_count(), 1);
    }

    #[test]
    fn crlf_loads_as_lf_and_is_remembered() {
        let b = Buffer::from_str("a\r\nb\r\n");
        assert_eq!(b.text(), "a\nb\n");
        assert_eq!(b.line_ending, LineEnding::CrLf);
    }

    #[test]
    fn a_transaction_undoes_and_redoes_as_one_step() {
        let mut b = Buffer::from_str("hello");
        b.begin(Pos::new(0, 0));
        b.insert(Pos::new(0, 5), " world");
        b.remove(0, 1);
        b.commit(Pos::new(0, 9));
        assert_eq!(b.text(), "ello world");
        assert_eq!(b.undo(), Some(Pos::new(0, 0)));
        assert_eq!(b.text(), "hello");
        assert_eq!(b.redo(), Some(Pos::new(0, 9)));
        assert_eq!(b.text(), "ello world");
    }

    #[test]
    fn dirty_follows_the_saved_version() {
        let mut b = Buffer::from_str("x");
        assert!(!b.is_dirty());
        b.insert(Pos::new(0, 1), "y");
        assert!(b.is_dirty());
        b.undo();
        assert!(!b.is_dirty(), "undo back to the saved text is clean, as in Vim");
        b.redo();
        assert!(b.is_dirty());
        b.mark_saved(b.version(), None);
        assert!(!b.is_dirty());
        b.undo();
        assert!(b.is_dirty(), "undo past the save is a change");
        b.redo();
        assert!(!b.is_dirty(), "redo back to the save is clean");
        b.undo();
        b.insert(Pos::new(0, 0), "z");
        b.undo();
        assert!(b.is_dirty(), "a new edit drops the redo, so the saved text is gone");
    }

    #[test]
    fn a_step_made_in_parts_is_dirty_until_undone() {
        let mut b = Buffer::from_str("x");
        b.begin(Pos::default());
        b.insert(Pos::new(0, 0), "a");
        assert!(b.is_dirty(), "an open step is a change");
        b.commit(Pos::new(0, 1));
        assert!(b.is_dirty());
        b.undo();
        assert!(!b.is_dirty());
    }

    #[test]
    fn marks_move_with_edits_undo_and_redo() {
        let mut b = Buffer::from_str("one\ntwo\nthree\n");
        b.set_mark('a', Pos::new(2, 1));
        b.insert(Pos::new(0, 0), "zero\n");
        assert_eq!(b.mark('a'), Some(Pos::new(3, 1)));
        b.undo();
        assert_eq!(b.mark('a'), Some(Pos::new(2, 1)));
        b.redo();
        assert_eq!(b.mark('a'), Some(Pos::new(3, 1)));
        let a = b.idx(Pos::new(3, 0));
        b.remove(a, a + 6);
        assert_eq!(b.mark('a'), Some(Pos::new(3, 0)), "a mark in removed text moves to its start");
    }

    #[test]
    fn history_restores_only_onto_the_same_text() {
        let mut a = Buffer::from_str("one");
        a.insert(Pos::new(0, 3), " two");
        let h = a.history();
        let mut b = Buffer::from_str("one two");
        assert!(b.restore_history(h.clone()));
        b.undo();
        assert_eq!(b.text(), "one");
        let mut c = Buffer::from_str("other");
        assert!(!c.restore_history(h));
    }

    #[test]
    fn trimming_on_save_is_one_small_undo_step() {
        let mut b = Buffer::from_str("a  \nb\t\nc");
        b.path = Some(PathBuf::from("unused.txt"));
        let (_, bytes, _) = b.prepare_save(SaveOpts { trim_trailing_whitespace: true, final_newline: true }).unwrap();
        assert_eq!(String::from_utf8(bytes).unwrap(), "a\nb\nc\n");
        b.undo();
        assert_eq!(b.text(), "a  \nb\t\nc");
    }

    #[test]
    fn save_trims_and_adds_the_final_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        let mut b = Buffer::from_str("a  \nb");
        b.path = Some(path.clone());
        b.save(SaveOpts { trim_trailing_whitespace: true, final_newline: true }).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "a\nb\n");
        assert!(!b.is_dirty());
    }
}

/// `EDITOR_BIG=path cargo test --release --lib big_file -- --ignored --nocapture`
#[cfg(test)]
mod big {
    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore]
    fn big_file_opens_and_edits_fast() {
        let Ok(path) = std::env::var("EDITOR_BIG") else { return };
        let t = Instant::now();
        let mut b = Buffer::open(Path::new(&path)).unwrap();
        let open = t.elapsed();
        let t = Instant::now();
        let last = b.last_line();
        let tail = b.line(last);
        let mid = Pos::new(last / 2, 4);
        b.begin(mid);
        b.insert(mid, "x");
        b.commit(mid);
        let a = b.idx(Pos::new(last / 2, 0));
        b.remove(a, a + b.line_len(last / 2) + 1);
        b.undo();
        let edit = t.elapsed();
        eprintln!(
            "open {:?} ({} lines, {} MB), jump + edit + undo {:?}, tail {:?}",
            open,
            b.line_count(),
            b.len_bytes() >> 20,
            edit,
            &tail[..20.min(tail.len())]
        );
        assert!(open.as_millis() < 1000, "open took {open:?}");
        assert!(edit.as_millis() < 8, "an edit took {edit:?}");
    }
}
