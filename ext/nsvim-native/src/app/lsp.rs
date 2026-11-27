//! The app's side of LSP: keeping servers in step with buffers, going to definitions and
//! references, hover, and the completion list in Insert mode.

use std::path::PathBuf;
use std::time::Duration;

use super::fx::Tx;
use editor::buffer::Pos;
use editor::editor::{KeyCode, KeyInput, Keymap, Mode};

use super::{App, Overlay, PickKind};
use crate::services::Event;
use crate::services::lsp::{Item, Job, Loc};

/// The completion list while it shows.
pub struct Completion {
    /// Where the word being completed starts.
    pub start: Pos,
    pub items: Vec<Item>,
    /// Items that match the word typed so far, by index into `items`.
    pub shown: Vec<usize>,
    pub sel: usize,
}

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl App {
    /// Tells the language server a file opened.
    pub fn lsp_open(&self, path: &std::path::Path) {
        let (Some(svc), Some(i)) = (&self.svc, self.doc_index(path)) else { return };
        let Some(lang) = self.meta.get(path).and_then(|m| m.lang) else { return };
        if self.ed.docs[i].buf.len_bytes() > super::BIG {
            return;
        }
        svc.lsp(Job::Open { path: path.to_path_buf(), lang: lang.name(), text: self.ed.docs[i].buf.text() });
    }

    /// Sends changed buffers to their servers once typing pauses for 150 ms, and asks for
    /// completions while a word is typed in Insert mode.
    pub fn lsp_sync(&mut self, clock: f64, tx: &mut Tx) {
        let Some(svc) = &self.svc else { return };
        let idle = clock - self.last_edit;
        for d in &self.ed.docs {
            let Some(path) = &d.buf.path else { continue };
            let Some(m) = self.meta.get_mut(path) else { continue };
            if m.lsp_sent == d.buf.version() + 1 || d.buf.len_bytes() > super::BIG {
                continue;
            }
            if idle < 0.15 {
                tx.wake_after(Duration::from_secs_f64(0.16 - idle));
                continue;
            }
            m.lsp_sent = d.buf.version() + 1;
            svc.lsp(Job::Change { path: path.clone(), text: d.buf.text() });
        }
    }

    pub fn lsp_event(&mut self, e: Event) {
        match e {
            Event::Diagnostics { path, diags } => {
                if let Some(m) = self.meta.get_mut(&path) {
                    m.diags = diags;
                }
            }
            Event::Locations { kind, locs } => {
                if locs.is_empty() {
                    self.ed.message(format!("no {kind} found"));
                } else if locs.len() == 1 && kind == "definition" {
                    let l = &locs[0];
                    self.open_path(l.path.clone(), Some(l.pos));
                } else {
                    self.show_locations(kind, locs);
                }
            }
            Event::Hover { path, text } => {
                let here = self.ed.has_doc() && self.ed.buf().path.as_deref() == Some(path.as_path());
                if here && !text.trim().is_empty() {
                    self.hover = Some(text.trim().to_string());
                } else if here {
                    self.ed.message("no information here");
                }
            }
            Event::Completion { path, at, items } => self.completions(path, at, items),
            Event::LspInfo(m) => self.ed.message(m),
            Event::ApplyEdit { label, files } => self.apply_workspace_edit(label, files),
            Event::CodeActions { titles } => {
                if titles.is_empty() {
                    self.ed.message("no code actions here");
                } else {
                    self.open_actions(titles);
                }
            }
            _ => {}
        }
    }

    /// A language server's answer that an embedding host sends (the host event `editor.lsp`),
    /// for the active file at the cursor. It goes the same way as a server's own answer.
    /// Positions are zero-based.
    ///
    /// - `{"kind": "completion", "items": [{"label": "len", "kind": "Method", "insert": "len"}]}`
    /// - `{"kind": "hover", "text": "fn len(&self) -> usize"}`
    /// - `{"kind": "diagnostics", "diags": [{"line": 0, "col": 3, "end_line": 0, "end_col": 7,
    ///   "severity": 1, "message": "..."}]}`
    pub fn host_lsp(&mut self, v: &serde_json::Value) {
        let Some((path, at)) = self.here() else { return };
        let text = |v: &serde_json::Value, k: &str| v[k].as_str().unwrap_or("").to_string();
        let num = |v: &serde_json::Value, k: &str| v[k].as_u64().unwrap_or(0) as usize;
        let list = |k: &str| v[k].as_array().cloned().unwrap_or_default();
        let e = match v["kind"].as_str() {
            Some("completion") => Event::Completion {
                path,
                at,
                items: list("items")
                    .iter()
                    .map(|i| {
                        let label = text(i, "label");
                        let insert = i["insert"].as_str().map_or_else(|| label.clone(), str::to_string);
                        Item { label, detail: text(i, "detail"), kind: item_kind(i["kind"].as_str().unwrap_or("")), insert }
                    })
                    .collect(),
            },
            Some("hover") => Event::Hover { path, text: text(v, "text") },
            Some("diagnostics") => Event::Diagnostics {
                path,
                diags: list("diags")
                    .iter()
                    .map(|d| crate::services::lsp::Diag {
                        line: num(d, "line"),
                        col: num(d, "col"),
                        end_line: d["end_line"].as_u64().map_or(num(d, "line"), |n| n as usize),
                        end_col: num(d, "end_col"),
                        severity: d["severity"].as_u64().unwrap_or(1) as u8,
                        message: text(d, "message"),
                    })
                    .collect(),
            },
            _ => return,
        };
        self.lsp_event(e);
    }

    /// Picks item `j` of the completion list, as a click on it does.
    pub fn pick_completion(&mut self, j: usize) {
        if let Some(c) = &mut self.completion
            && j < c.shown.len()
        {
            c.sel = j;
            self.accept_completion();
        }
    }

    /// Lists places in the picker: references, several definitions, diagnostics.
    pub fn show_locations(&mut self, kind: &'static str, locs: Vec<Loc>) {
        self.locations = locs;
        self.locations_title = match kind {
            "references" => "References",
            "definition" => "Definitions",
            "changes" => "Outside changes",
            _ => "Diagnostics",
        };
        self.open_picker(PickKind::Locations);
    }

    /// The active buffer's path and cursor, for a request.
    fn here(&self) -> Option<(PathBuf, Pos)> {
        if !self.ed.has_doc() || self.on_settings {
            return None;
        }
        Some((self.ed.buf().path.clone()?, self.ed.cursor()))
    }

    pub fn goto_definition(&mut self) {
        let Some((path, pos)) = self.here() else { return };
        self.flush_lsp(&path);
        if let Some(svc) = &self.svc {
            svc.lsp(Job::Definition { path, pos });
        }
    }

    pub fn references(&mut self) {
        let Some((path, pos)) = self.here() else { return };
        self.flush_lsp(&path);
        if let Some(svc) = &self.svc {
            svc.lsp(Job::References { path, pos });
        }
    }

    pub fn hover_here(&mut self) {
        let Some((path, pos)) = self.here() else { return };
        self.flush_lsp(&path);
        if let Some(svc) = &self.svc {
            svc.lsp(Job::Hover { path, pos });
        }
    }

    /// Asks the server to rename the symbol under the cursor to `name`.
    pub fn rename(&mut self, name: String) {
        let Some((path, pos)) = self.here() else { return };
        if name.trim().is_empty() {
            self.ed.error("E471: Argument required");
            return;
        }
        self.flush_lsp(&path);
        if let Some(svc) = &self.svc {
            svc.lsp(Job::Rename { path, pos, name: name.trim().to_string() });
        }
    }

    /// The word under the cursor, when it can be a symbol's name, after leaving Insert and
    /// Visual mode. None with no file open.
    fn rename_word(&mut self) -> Option<String> {
        if !self.ed.has_doc() {
            return None;
        }
        let cur = self.ed.cursor();
        let (a, b) = editor::vim::word_range_at(self.ed.buf(), cur);
        let word = self.ed.buf().slice(a, b);
        let word = if word.chars().all(|c| c.is_alphanumeric() || c == '_') { word } else { String::new() };
        if self.ed.keymap == Keymap::Vim && matches!(self.ed.mode, Mode::Insert | Mode::Visual(_)) {
            editor::vim::handle_key(&mut self.ed, KeyInput::plain(KeyCode::Esc));
        }
        Some(word)
    }

    /// Opens the command line with `:Rename <word under the cursor>`, ready to edit. The keys
    /// (F2) rename this way.
    pub fn start_rename(&mut self) {
        let Some(word) = self.rename_word() else { return };
        editor::vim::open_cmdline(&mut self.ed, ':', &format!("Rename {word}"));
    }

    /// Asks for the new name in a dialog. A menu, the palette and the right-click menu rename
    /// this way, so the mouse never lands on the command line. Where no dialog can show (a
    /// terminal), the command line asks.
    pub fn ask_rename(&mut self, tx: &mut Tx) {
        let Some(word) = self.rename_word() else { return };
        let title = if word.is_empty() { "Rename symbol".to_string() } else { format!("Rename \"{word}\"") };
        let dialog = super::fx::Confirm::new(title, "The language server renames it everywhere it is used.")
            .button("rename", "Rename", super::fx::Role::Default)
            .button("cancel", "Cancel", super::fx::Role::Cancel);
        tx.prompt(dialog, word.clone(), move |a, typed, _| match typed {
            Ok(Some(name)) if name.trim() != word => a.rename(name),
            Ok(_) => {}
            Err(_) => editor::vim::open_cmdline(&mut a.ed, ':', &format!("Rename {word}")),
        });
    }

    /// Asks for the code actions at the cursor or over the selection.
    pub fn code_actions(&mut self) {
        let Some((path, pos)) = self.here() else { return };
        self.flush_lsp(&path);
        let (start, end) = match self.ed.doc().view.anchor {
            Some(a) if a < pos => (a, pos),
            Some(a) => (pos, a),
            None => (pos, pos),
        };
        if let Some(svc) = &self.svc {
            svc.lsp(Job::CodeActions { path, start, end });
        }
    }

    /// Applies a rename's or code action's edits: open files at once, other files open in the
    /// background with the edits made and left unsaved.
    fn apply_workspace_edit(&mut self, label: String, files: Vec<crate::services::lsp::FileEdit>) {
        let (n, m) = crate::services::lsp::edits_summary(&files);
        for f in files {
            match self.doc_index(&f.path) {
                Some(i) => {
                    let doc = &mut self.ed.docs[i];
                    super::apply_edits(&mut doc.buf, &f.edits);
                    doc.view.cursor = doc.buf.clamp(doc.view.cursor, false);
                }
                None => {
                    if let Some(svc) = &self.svc {
                        svc.open_edited(f.path, f.edits);
                    }
                }
            }
        }
        self.edited();
        let files = if m == 1 { "1 file".to_string() } else { format!("{m} files") };
        let tail = if m > 1 { ". :wa writes them all" } else { "" };
        self.ed.message(format!("{label}: {n} changes in {files}{tail}"));
    }

    /// The active file's diagnostics, in the picker.
    pub fn show_diagnostics(&mut self) {
        let Some((path, _)) = self.here() else { return };
        let Some(m) = self.meta.get(&path).filter(|m| !m.diags.is_empty()) else {
            self.ed.message("no diagnostics");
            return;
        };
        let i = self.ed.active;
        let locs = m
            .diags
            .iter()
            .map(|d| Loc {
                path: path.clone(),
                pos: Pos::new(d.line, d.col),
                text: format!("{} {}", if d.severity <= 1 { "E" } else { "W" }, d.message),
            })
            .collect();
        let _ = i;
        self.show_locations("diagnostics", locs);
    }

    /// Sends the buffer now, so a request sees the latest text.
    fn flush_lsp(&mut self, path: &std::path::Path) {
        let (Some(svc), Some(i)) = (&self.svc, self.doc_index(path)) else { return };
        let buf = &self.ed.docs[i].buf;
        if let Some(m) = self.meta.get_mut(path)
            && m.lsp_sent != buf.version() + 1
        {
            m.lsp_sent = buf.version() + 1;
            svc.lsp(Job::Change { path: path.to_path_buf(), text: buf.text() });
        }
    }

    /// The word before the cursor: its start and text.
    fn word_before(&self) -> Option<(Pos, String)> {
        if !self.ed.has_doc() {
            return None;
        }
        let cur = self.ed.cursor();
        let line: Vec<char> = self.ed.buf().line(cur.line).chars().collect();
        let end = cur.col.min(line.len());
        let mut start = end;
        while start > 0 && word_char(line[start - 1]) {
            start -= 1;
        }
        Some((Pos::new(cur.line, start), line[start..end].iter().collect()))
    }

    fn typing(&self) -> bool {
        self.ed.mode == Mode::Insert && !self.on_settings
    }

    /// After a typed key in Insert mode: ask for completions at a word's second char or after
    /// `.` and `::`, and filter the open list as the word grows.
    pub fn after_typing(&mut self, k: KeyInput) {
        if !self.typing() {
            self.completion = None;
            return;
        }
        let Some((start, word)) = self.word_before() else { return };
        if let Some(c) = &mut self.completion {
            if c.start != start || word.is_empty() {
                self.completion = None;
            } else {
                refilter(c, &word);
                if c.shown.is_empty() {
                    self.completion = None;
                }
                return;
            }
        }
        let trigger = match k.code {
            KeyCode::Char('.') => true,
            KeyCode::Char(':') => self.ed.buf().line(start.line).chars().take(self.ed.cursor().col).collect::<String>().ends_with("::"),
            KeyCode::Char(c) if word_char(c) => word.chars().count() == 2,
            _ => false,
        };
        if trigger {
            let Some((path, pos)) = self.here() else { return };
            self.flush_lsp(&path);
            if let Some(svc) = &self.svc {
                svc.lsp(Job::Completion { path, pos });
            }
        }
    }

    fn completions(&mut self, path: PathBuf, at: Pos, items: Vec<Item>) {
        if !self.typing() || items.is_empty() {
            return;
        }
        let Some((start, word)) = self.word_before() else { return };
        if self.ed.buf().path.as_deref() != Some(path.as_path()) || at.line != start.line {
            return;
        }
        let mut c = Completion { start, items, shown: Vec::new(), sel: 0 };
        refilter(&mut c, &word);
        self.completion = (!c.shown.is_empty()).then_some(c);
    }

    /// Keys while the completion list shows. True when the list took the key.
    pub fn completion_key(&mut self, k: KeyInput) -> bool {
        let Some(c) = &mut self.completion else { return false };
        let n = c.shown.len();
        let vim = self.ed.keymap == Keymap::Vim;
        match k.code {
            KeyCode::Down => c.sel = (c.sel + 1) % n,
            KeyCode::Up => c.sel = (c.sel + n - 1) % n,
            KeyCode::Char('n') if k.ctrl => c.sel = (c.sel + 1) % n,
            KeyCode::Char('p') if k.ctrl => c.sel = (c.sel + n - 1) % n,
            KeyCode::Tab if k.shift => c.sel = (c.sel + n - 1) % n,
            KeyCode::Enter | KeyCode::Tab => self.accept_completion(),
            KeyCode::Char('y') if k.ctrl && vim => self.accept_completion(),
            KeyCode::Char('e') if k.ctrl && vim => self.completion = None,
            KeyCode::Esc => {
                self.completion = None;
                return false;
            }
            _ => return false,
        }
        true
    }

    fn accept_completion(&mut self) {
        let Some(c) = self.completion.take() else { return };
        let Some(&i) = c.shown.get(c.sel) else { return };
        let insert = c.items[i].insert.clone();
        let cur = self.ed.cursor();
        let doc = self.ed.doc_mut();
        let a = doc.buf.idx(c.start);
        let b = doc.buf.idx(cur);
        doc.buf.begin(cur);
        let end = doc.buf.replace(a, b, &insert);
        let p = doc.buf.pos(end);
        doc.view.cursor = p;
        doc.view.want_col = p.col;
        self.ed.scroll_to_cursor();
        self.edited();
    }

    /// Runs a key for an open hover or completion first. True when it was used up.
    pub fn popup_key(&mut self, k: KeyInput, _tx: &mut Tx) -> bool {
        if self.hover.is_some() {
            self.hover = None;
            if k.code == KeyCode::Esc {
                return true;
            }
        }
        if self.completion.is_some() && self.completion_key(k) {
            return true;
        }
        if !matches!(self.overlay, Overlay::None) {
            self.completion = None;
        }
        false
    }
}

/// The kind of a completion item by its LSP name, as the server's answers name them.
fn item_kind(name: &str) -> &'static str {
    const KINDS: &[&str] = &[
        "Method",
        "Function",
        "Constructor",
        "Field",
        "Variable",
        "Class",
        "Interface",
        "Module",
        "Property",
        "Enum",
        "Keyword",
        "Snippet",
        "EnumMember",
        "Constant",
        "Struct",
        "TypeParam",
    ];
    KINDS.iter().find(|k| **k == name).copied().unwrap_or("Text")
}

fn refilter(c: &mut Completion, word: &str) {
    let labels: Vec<&str> = c.items.iter().map(|i| i.label.as_str()).collect();
    let (m, _) = crate::fuzzy::rank(word, &labels, 60);
    // Keep the server's order among equally good matches.
    let mut shown: Vec<(i64, usize)> = m.into_iter().map(|m| (m.score, m.index)).collect();
    shown.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    c.shown = shown.into_iter().map(|(_, i)| i).take(12).collect();
    c.sel = c.sel.min(c.shown.len().saturating_sub(1));
}

/// Applies LSP text edits to a buffer as one undo step, last edit first so earlier positions
/// hold.
pub fn apply_edits(buf: &mut editor::buffer::Buffer, edits: &[crate::services::lsp::TextEdit]) {
    let mut order: Vec<&crate::services::lsp::TextEdit> = edits.iter().collect();
    order.sort_by_key(|e| std::cmp::Reverse(e.start));
    let at = order.last().map(|e| e.start).unwrap_or_default();
    buf.begin(at);
    for e in order {
        let (a, b) = (buf.idx(e.start), buf.idx(e.end));
        buf.replace(a, b, &e.text);
    }
    buf.commit(at);
}
