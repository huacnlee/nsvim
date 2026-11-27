//! The editing core: open documents, the cursor, the mode, registers and options. It knows
//! nothing about windows or drawing. Keymaps (`vim`, `classic`) change it through `&mut Editor`
//! and return [`Action`]s for what the app must do outside the text.

use std::path::{Path, PathBuf};

use crate::buffer::{Buffer, Pos, SaveOpts};

/// The editor's modes. A classic keymap stays in Insert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual(VisualKind),
    /// Typing on the command line: `:`, `/` or `?`. The line itself is in `Editor::cmd`.
    Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualKind {
    Char,
    Line,
    Block,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Visual(VisualKind::Char) => "VISUAL",
            Mode::Visual(VisualKind::Line) => "V-LINE",
            Mode::Visual(VisualKind::Block) => "V-BLOCK",
            Mode::Command => "COMMAND",
        }
    }
}

/// Which family of bindings the user picked during onboarding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Keymap {
    #[default]
    Vim,
    Classic,
}

/// A key, as the keymaps read it. `ctrl` is the Control key on every platform; `cmd` is the
/// platform's shortcut modifier (Command on macOS, Control elsewhere, so Ctrl+S on Linux has
/// both set). Vim bindings read `ctrl`; classic bindings read `cmd`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyInput {
    pub code: KeyCode,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub cmd: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCode {
    /// A typed character, already shifted (`A`, `:`, `{`).
    Char(char),
    Esc,
    Enter,
    Backspace,
    Delete,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
}

impl KeyInput {
    pub const fn plain(code: KeyCode) -> KeyInput {
        KeyInput { code, ctrl: false, alt: false, shift: false, cmd: false }
    }

    pub const fn ch(c: char) -> KeyInput {
        KeyInput::plain(KeyCode::Char(c))
    }

    pub const fn ctrl(c: char) -> KeyInput {
        KeyInput { code: KeyCode::Char(c), ctrl: true, alt: false, shift: false, cmd: true }
    }

    /// The char a plain key types, or `None` for named keys and chords.
    pub fn typed(&self) -> Option<char> {
        match self.code {
            KeyCode::Char(c) if !self.ctrl && !self.cmd => Some(c),
            _ => None,
        }
    }

    /// Short text for the command line's showcmd area (`d`, `<C-r>`, `<space>`).
    pub fn show(&self) -> String {
        match self.code {
            KeyCode::Char(' ') if !self.ctrl => "<space>".into(),
            KeyCode::Char(c) if self.ctrl => format!("^{}", c.to_ascii_uppercase()),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Esc => "<esc>".into(),
            KeyCode::Enter => "<cr>".into(),
            KeyCode::Tab => "<tab>".into(),
            _ => String::new(),
        }
    }
}

/// What a keymap asks the app to do outside the text.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Write the active buffer (`:w`), optionally to a new path (`:w path`).
    Save(Option<PathBuf>),
    /// Write every modified buffer (`:wa`).
    SaveAll,
    /// Close the active buffer (`:q`, `:q!`), quitting when it is the last one.
    Quit {
        force: bool,
    },
    /// Close every buffer and quit (`:qa`, `:qa!`).
    QuitAll {
        force: bool,
    },
    /// Open a file (`:e path`).
    Open(PathBuf),
    /// Open a file with the cursor at a position (`Ctrl-o` back into another file, a mark).
    OpenAt(PathBuf, Pos),
    /// Rename the symbol under the cursor across the project (`:Rename name`, LSP).
    Rename(String),
    /// List the language server's code actions at the cursor (`:CodeAction`).
    CodeActions,
    /// Format the file with the language server (`:Format`), or lines `first..=last` of a
    /// range (`:'<,'>Format`). Zero-based, inclusive.
    Format(Option<(usize, usize)>),
    /// Replace lines `first..=last` with the output of a shell command run on them
    /// (`:'<,'>!sort`). Zero-based, inclusive.
    Filter {
        first: usize,
        last: usize,
        cmd: String,
    },
    /// Run a shell command and show its output (`:!cargo fmt`).
    Shell(String),
    /// Reload the active buffer from disk (`:e!`).
    Reload,
    FindFile,
    /// Pick a project to open: a recent one, or a folder from the system's dialog.
    OpenProject,
    /// Open this folder as the project (`:Open path`, `:cd path`). Relative to the project.
    OpenFolder(String),
    /// Make an empty file and open it (`:NewFile path`). Relative to the project.
    NewFile(String),
    /// Open an empty buffer with no file name (`:enew`, `Cmd N` in the classic keymap).
    NewBuffer,
    /// Make a folder and its parents (`:Mkdir path`). Relative to the project.
    NewFolder(String),
    /// Move or rename the tree's row when the tree has the focus, else the active file
    /// (`:Move path`). Relative to the project.
    MovePath(String),
    /// Delete the tree's row when the tree has the focus, else the active file (`:Delete`).
    /// `force` deletes a file with unsaved changes too.
    DeletePath {
        force: bool,
    },
    ToggleTree,
    FocusTree,
    GrepProject,
    PickBuffer,
    CloseBuffer,
    NextBuffer,
    PrevBuffer,
    Diagnostics,
    Settings,
    /// Open App Setup, to change its picks (`:Setup`).
    Setup,
    /// Check for a new release and install it (`:Update`).
    Update,
    /// Open the leader (Space) key menu.
    LeaderMenu,
    /// Open the command palette.
    Palette,
    /// Open the list of every key, with this text in its search line (`F1`, `SPC ?`, `:Keys`).
    KeyList(String),
    /// Put this text on the system clipboard.
    Copy(String),
    /// Read the system clipboard and type it in Insert mode (`Ctrl r +`).
    InsertClipboard,
    /// Read the system clipboard and paste it: after the cursor, or before it when `before`.
    Paste {
        before: bool,
        count: usize,
    },
    /// Set an option from `:set name=value` (`keymap`, `numbers`, `wrap`, ...).
    SetOption(String, String),
    GotoDefinition,
    Hover,
    /// Show a line on the command line.
    Message(String),
    Error(String),
}

/// Where the cursor is in a document and what part of it shows.
#[derive(Clone, Copy, Debug, Default)]
pub struct View {
    pub cursor: Pos,
    /// The other end of a Visual selection, or of a classic Shift selection.
    pub anchor: Option<Pos>,
    /// The first line on screen.
    pub top: usize,
    /// The first column on screen when lines don't wrap.
    pub left: usize,
    /// The column `j` and `k` try to keep (Vim's curswant). `usize::MAX` sticks to line ends.
    pub want_col: usize,
}

/// One more cursor (the classic keymap's Alt-click and Cmd-D): where it is, and the other end
/// of its selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cursor {
    pub pos: Pos,
    pub anchor: Option<Pos>,
}

pub struct Doc {
    /// The document's own number, which stays the same while it is open. No two documents
    /// of the process share one. The app keys a tab with it.
    pub id: u64,
    pub buf: Buffer,
    pub view: View,
    /// Extra cursors, in no order. The main cursor is `view.cursor` with `view.anchor`.
    pub extra: Vec<Cursor>,
}

impl Doc {
    pub fn new(buf: Buffer) -> Doc {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Doc { id, buf, view: View::default(), extra: Vec::new() }
    }
}

/// A register's contents. `linewise` text pastes as whole lines.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reg {
    pub text: String,
    pub linewise: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Registers {
    /// The unnamed register, `"`.
    pub unnamed: Reg,
    /// `a`-`z`.
    pub named: std::collections::HashMap<char, Reg>,
    /// The last yank, `0`.
    pub yank: Reg,
    /// Deletes and changes of a line or more, newest first: `1` to `9`.
    pub numbered: std::collections::VecDeque<Reg>,
    /// The last delete inside one line, `-`.
    pub small: Reg,
    /// The text typed in the last Insert session, `.`.
    pub inserted: Reg,
}

impl Registers {
    pub fn get(&self, name: Option<char>) -> Option<&Reg> {
        match name {
            None | Some('"') | Some('+') | Some('*') => Some(&self.unnamed),
            Some('0') => Some(&self.yank),
            Some(c @ '1'..='9') => self.numbered.get(c as usize - '1' as usize),
            Some('-') => Some(&self.small),
            Some('.') => Some(&self.inserted),
            Some(c) => self.named.get(&c.to_ascii_lowercase()),
        }
    }

    /// Stores a yank or delete. An uppercase name appends, as in Vim.
    pub fn set(&mut self, name: Option<char>, reg: Reg, yank: bool) {
        if let Some(c) = name.filter(|c| c.is_ascii_alphabetic()) {
            let slot = self.named.entry(c.to_ascii_lowercase()).or_default();
            if c.is_ascii_uppercase() {
                slot.text.push_str(&reg.text);
                slot.linewise |= reg.linewise;
            } else {
                *slot = reg.clone();
            }
        }
        if let Some(c @ '1'..='9') = name {
            let i = c as usize - '1' as usize;
            while self.numbered.len() <= i {
                self.numbered.push_back(Reg::default());
            }
            self.numbered[i] = reg.clone();
        } else if name == Some('-') {
            self.small = reg.clone();
        }
        if yank {
            self.yank = reg.clone();
        } else if name.is_none() || name == Some('"') {
            // As in Vim: a delete of a line or more shifts `1`-`9`, a smaller one goes to `-`.
            if reg.linewise || reg.text.contains('\n') {
                self.numbered.push_front(reg.clone());
                self.numbered.truncate(9);
            } else {
                self.small = reg.clone();
            }
        }
        self.unnamed = reg;
    }
}

/// The command line while it is typed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CmdLine {
    /// `:`, `/` or `?`.
    pub kind: char,
    pub text: String,
    /// The caret, in chars.
    pub caret: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Search {
    pub pattern: String,
    pub backward: bool,
}

/// Editing options, from config.toml, `.editorconfig` and `:set`.
#[derive(Clone, Debug)]
pub struct Opts {
    pub tab_width: usize,
    pub spaces: bool,
    pub scrolloff: usize,
    pub wrap: bool,
    /// Search ignores case (`:set ignorecase`), unless `smartcase` sees an uppercase letter.
    pub ignorecase: bool,
    pub smartcase: bool,
    pub auto_close: bool,
    pub save: SaveOpts,
}

impl Default for Opts {
    fn default() -> Opts {
        Opts {
            tab_width: 4,
            spaces: true,
            scrolloff: 8,
            wrap: false,
            ignorecase: true,
            smartcase: true,
            auto_close: true,
            save: SaveOpts { trim_trailing_whitespace: true, final_newline: true },
        }
    }
}

/// A line on the command line after a command ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Msg {
    pub text: String,
    pub error: bool,
}

pub struct Editor {
    pub docs: Vec<Doc>,
    pub active: usize,
    pub mode: Mode,
    pub keymap: Keymap,
    pub regs: Registers,
    pub cmd: Option<CmdLine>,
    pub search: Search,
    pub msg: Option<Msg>,
    pub opts: Opts,
    /// Text rows on screen, set by the view each frame it changes.
    pub rows: usize,
    /// Text columns on screen.
    pub cols: usize,
    /// The Vim engine's pending keys, count, dot repeat and macros.
    pub vim: crate::vim::VimState,
    /// The project root: the folder the editor opened, or the file's folder.
    pub root: PathBuf,
}

impl Editor {
    pub fn new(root: PathBuf) -> Editor {
        Editor {
            docs: Vec::new(),
            active: 0,
            mode: Mode::Normal,
            keymap: Keymap::Vim,
            regs: Registers::default(),
            cmd: None,
            search: Search::default(),
            msg: None,
            opts: Opts::default(),
            rows: 30,
            cols: 100,
            vim: Default::default(),
            root,
        }
    }

    pub fn has_doc(&self) -> bool {
        self.active < self.docs.len()
    }

    /// The active document. Panics when none is open; check [`Self::has_doc`] first.
    pub fn doc(&self) -> &Doc {
        &self.docs[self.active]
    }

    pub fn doc_mut(&mut self) -> &mut Doc {
        &mut self.docs[self.active]
    }

    pub fn buf(&self) -> &Buffer {
        &self.doc().buf
    }

    pub fn cursor(&self) -> Pos {
        self.doc().view.cursor
    }

    /// The mode a keymap rests in: Normal for Vim, Insert for classic.
    pub fn rest_mode(&self) -> Mode {
        match self.keymap {
            Keymap::Vim => Mode::Normal,
            Keymap::Classic => Mode::Insert,
        }
    }

    pub fn set_keymap(&mut self, map: Keymap) {
        self.keymap = map;
        self.mode = self.rest_mode();
        self.cmd = None;
        self.vim = Default::default();
        if self.has_doc() {
            self.doc_mut().view.anchor = None;
        }
    }

    /// Opens `path`, or switches to it when it is already open. Returns its index.
    pub fn open(&mut self, path: &Path) -> std::io::Result<usize> {
        let path = absolute(path);
        if let Some(i) = self.docs.iter().position(|d| d.buf.path.as_deref() == Some(path.as_path())) {
            self.active = i;
            return Ok(i);
        }
        let buf = Buffer::open(&path)?;
        self.docs.push(Doc::new(buf));
        self.active = self.docs.len() - 1;
        Ok(self.active)
    }

    /// Closes document `i` and picks the one before it.
    pub fn close(&mut self, i: usize) {
        if i < self.docs.len() {
            self.docs.remove(i);
            if self.active >= self.docs.len() || self.active > i {
                self.active = self.active.saturating_sub(1);
            }
        }
    }

    /// Moves document `from` to place `to` among the documents, as a tab dragged along the buffer
    /// line does. `to` counts the places without `from`. The active document stays active.
    /// An index past the end does nothing.
    pub fn move_doc(&mut self, from: usize, to: usize) {
        if from >= self.docs.len() || to >= self.docs.len() || from == to {
            return;
        }
        let active = self.active;
        let doc = self.docs.remove(from);
        self.docs.insert(to, doc);
        self.active = if active == from {
            to
        } else if from < active && active <= to {
            active - 1
        } else if to <= active && active < from {
            active + 1
        } else {
            active
        };
    }

    /// Puts the documents in the order of `ids`, their [`Doc::id`]s: the order of the tabs
    /// across the panes. A document `ids` leaves out keeps its place after the others. The
    /// active document stays active.
    pub fn order_docs(&mut self, ids: &[u64]) {
        let active = self.docs.get(self.active).map(|d| d.id);
        let rank = |d: &Doc| ids.iter().position(|&id| id == d.id).unwrap_or(usize::MAX);
        self.docs.sort_by_key(rank);
        if let Some(id) = active {
            self.active = self.docs.iter().position(|d| d.id == id).unwrap_or(self.active);
        }
    }

    /// Moves the cursor to `p`, clamped for the mode, and forgets the wanted column.
    pub fn set_cursor(&mut self, p: Pos) {
        let past_end = self.mode == Mode::Insert || matches!(self.mode, Mode::Visual(_));
        let doc = self.doc_mut();
        doc.view.cursor = doc.buf.clamp(p, past_end);
        doc.view.want_col = doc.view.cursor.col;
        self.scroll_to_cursor();
    }

    /// Scrolls so the cursor is on screen with `scrolloff` lines around it.
    pub fn scroll_to_cursor(&mut self) {
        let rows = self.rows.max(1);
        let off = self.opts.scrolloff.min(rows.saturating_sub(1) / 2);
        let cols = self.cols.max(1);
        let wrap = self.opts.wrap;
        let doc = self.doc_mut();
        let line = doc.view.cursor.line;
        let last = doc.buf.last_line();
        if line < doc.view.top + off {
            doc.view.top = line.saturating_sub(off);
        } else if line + off >= doc.view.top + rows {
            doc.view.top = (line + off + 1).saturating_sub(rows);
        }
        doc.view.top = doc.view.top.min(last);
        if wrap {
            doc.view.left = 0;
        } else {
            let col = doc.view.cursor.col;
            if col < doc.view.left {
                doc.view.left = col;
            } else if col >= doc.view.left + cols {
                doc.view.left = col + 1 - cols;
            }
        }
    }

    /// Scrolls the view by `lines` (negative up). The cursor stays where it is, also off screen.
    /// The next cursor move scrolls back to it.
    pub fn scroll_view(&mut self, lines: isize) {
        if !self.has_doc() {
            return;
        }
        let doc = self.doc_mut();
        let last = doc.buf.last_line();
        doc.view.top = (doc.view.top as isize + lines).clamp(0, last as isize) as usize;
    }

    /// Scrolls the view by `lines` (negative up) and keeps the cursor on screen.
    pub fn scroll_by(&mut self, lines: isize) {
        if !self.has_doc() {
            return;
        }
        let rows = self.rows.max(1);
        let off = self.opts.scrolloff.min(rows.saturating_sub(1) / 2);
        let past_end = self.mode == Mode::Insert;
        let doc = self.doc_mut();
        let last = doc.buf.last_line();
        let top = (doc.view.top as isize + lines).clamp(0, last as isize) as usize;
        doc.view.top = top;
        let lo = if top == 0 { 0 } else { top + off };
        let hi = (top + rows).saturating_sub(off + 1).max(lo);
        let line = doc.view.cursor.line.clamp(lo.min(last), hi.min(last));
        if line != doc.view.cursor.line {
            let want = doc.view.want_col;
            doc.view.cursor = doc.buf.clamp(Pos::new(line, want), past_end);
        }
    }

    /// Register `name`'s contents, or `None` when it is empty. `/` is the last search and `:`
    /// the last Ex command.
    pub fn reg(&self, name: Option<char>) -> Option<Reg> {
        let text = |t: &str| (!t.is_empty()).then(|| Reg { text: t.to_string(), linewise: false });
        match name {
            Some('/') => text(&self.search.pattern),
            Some(':') => text(self.vim.last_ex.as_deref().unwrap_or("")),
            _ => self.regs.get(name).filter(|r| !r.text.is_empty()).cloned(),
        }
    }

    /// How a search treats case: `Some(true)` ignores it, `Some(false)` matches it, `None` lets
    /// smartcase decide from the pattern.
    pub fn case(&self) -> Option<bool> {
        if !self.opts.ignorecase {
            Some(false)
        } else if !self.opts.smartcase {
            Some(true)
        } else {
            None
        }
    }

    pub fn message(&mut self, text: impl Into<String>) {
        self.msg = Some(Msg { text: text.into(), error: false });
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.msg = Some(Msg { text: text.into(), error: true });
    }

    /// The indent one level adds: a tab, or `tab_width` spaces.
    pub fn indent_unit(&self) -> String {
        if self.opts.spaces { " ".repeat(self.opts.tab_width) } else { "\t".into() }
    }
}

/// `p` made absolute against the current directory, without touching the disk.
pub fn absolute(p: &Path) -> PathBuf {
    if p.is_absolute() { p.to_path_buf() } else { std::env::current_dir().map(|d| d.join(p)).unwrap_or_else(|_| p.to_path_buf()) }
}

#[cfg(test)]
#[path = "editor_tests.rs"]
mod tests;
