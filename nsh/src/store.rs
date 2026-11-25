//! The app's one store: the core `Editor`, kept out of the reactive graph, and a revision
//! signal that every change bumps. Views never read the editor directly. They read
//! `Session::frame`, a memo of what the screen shows, so a key changes only the rows, the
//! cursor and the status parts whose content changed.
//!
//! File reads and writes run on tokio's blocking threads (`blocking`). Their results come
//! back on the engine thread in a `spawn`, which writes the store.

use std::path::{Path, PathBuf};

use editor::buffer::Buffer;
use editor::editor::{Action, Doc, Editor, KeyInput, Keymap};
use nsh_core::prelude::*;

use crate::model::{self, Frame};

/// The size of one cell of the code font, measured, and the text area's size in cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    /// The advance of one char of the code font.
    pub cell_w: f32,
    /// The height of one row: the font size times the line height.
    pub cell_h: f32,
    pub rows: usize,
    pub cols: usize,
}

/// The store. `Copy`: capture it in closures freely.
#[derive(Clone, Copy)]
pub struct Session {
    ed: Stored<Editor>,
    rev: Signal<u64>,
    grid: Signal<Grid>,
    /// What the screen shows. Recomputed when `rev` or `grid` changes. Readers run again only
    /// when their part of it changed.
    pub frame: Memo<Frame>,
}

impl Session {
    pub fn new(root: PathBuf, cell_w: f32, cell_h: f32) -> Session {
        let ed = stored(Editor::new(root));
        let rev = signal(0_u64);
        let grid = signal(Grid { cell_w, cell_h, rows: 30, cols: 100 });
        let frame = computed(move || {
            rev.get();
            let rows = grid.with(|g| g.rows);
            ed.with(|e| model::frame(e, rows))
        });
        Session { ed, rev, grid, frame }
    }

    pub fn grid(&self) -> Grid {
        self.grid.get()
    }

    /// Reads the editor without subscribing. For listeners and tests.
    pub fn read<R>(&self, f: impl FnOnce(&Editor) -> R) -> R {
        self.ed.with(|e| f(e))
    }

    /// Changes the editor, then tells the views.
    pub fn edit<R>(&self, f: impl FnOnce(&mut Editor) -> R) -> R {
        let out = self.ed.with(f);
        self.rev.update(|n| *n += 1);
        out
    }

    /// The text area got a new size: fit the editor's rows and columns to it.
    pub fn resize(&self, width: f32, height: f32) {
        let g = self.grid.get_untracked();
        let gutter = self.read(model::gutter_cols) as f32 * g.cell_w;
        let rows = ((height / g.cell_h).floor() as usize).max(1);
        let cols = (((width - gutter) / g.cell_w).floor() as usize).max(10);
        if rows == g.rows && cols == g.cols {
            return;
        }
        batch(|| {
            self.grid.set(Grid { rows, cols, ..g });
            self.edit(|e| {
                e.rows = rows;
                e.cols = cols;
                if e.has_doc() {
                    e.scroll_to_cursor();
                }
            });
        });
    }

    /// A key for the active keymap. Actions outside the text come back and run here.
    pub fn key(&self, key: KeyInput) {
        let actions = self.edit(|e| {
            e.msg = None;
            if !e.has_doc() {
                return Vec::new();
            }
            match e.keymap {
                Keymap::Vim => editor::vim::handle_key(e, key),
                Keymap::Classic => editor::classic::handle_key(e, key),
            }
        });
        for a in actions {
            self.run(a);
        }
    }

    /// Scrolls by `lines`, negative up.
    pub fn scroll(&self, lines: isize) {
        self.edit(|e| e.scroll_by(lines));
    }

    /// A click at a point in the text area, relative to its top left.
    pub fn click(&self, x: f32, y: f32) {
        let g = self.grid.get_untracked();
        self.edit(|e| {
            if !e.has_doc() {
                return;
            }
            let gutter = model::gutter_cols(e) as f32 * g.cell_w;
            let doc = e.doc();
            let line = (doc.view.top + (y / g.cell_h).max(0.0) as usize).min(doc.buf.last_line());
            let text = doc.buf.line(line);
            let left = model::columns(&text, e.opts.tab_width)[doc.view.left.min(text.chars().count())];
            let col = left + ((x - gutter).max(0.0) / g.cell_w) as usize;
            let ch = model::char_at_column(&text, e.opts.tab_width, col);
            e.set_cursor(editor::buffer::Pos::new(line, ch));
        });
    }

    /// Opens `path` off the engine thread and shows it.
    pub fn open(&self, path: PathBuf) {
        let me = *self;
        spawn(async move {
            let path = absolute(&path);
            let shown = path.clone();
            match blocking(move || Buffer::open(&path)).await {
                Ok(Ok(buf)) => me.edit(|e| {
                    e.docs.push(Doc::new(buf));
                    e.active = e.docs.len() - 1;
                    e.mode = e.rest_mode();
                    e.scroll_to_cursor();
                }),
                Ok(Err(err)) => me.edit(|e| e.error(format!("{}: {err}", shown.display()))),
                Err(err) => me.edit(|e| e.error(format!("open: {err}"))),
            }
        });
    }

    /// Writes the active buffer off the engine thread, then marks that version clean.
    fn save(&self, to: Option<PathBuf>) {
        let prepared = self.edit(|e| {
            if !e.has_doc() {
                return Err("E32: No file name".to_string());
            }
            if let Some(p) = to {
                let p = if p.is_absolute() { p } else { e.root.join(p) };
                e.doc_mut().buf.path = Some(p);
            }
            let opts = e.opts.save;
            let doc = e.doc_mut();
            let at = doc.view.cursor;
            let out = doc.buf.prepare_save_at(opts, at).map_err(|err| err.to_string());
            doc.view.cursor = doc.buf.clamp(at, false);
            out
        });
        let (path, bytes, version) = match prepared {
            Ok(p) => p,
            Err(err) => return self.edit(|e| e.error(err)),
        };
        let me = *self;
        spawn(async move {
            let at = path.clone();
            let len = bytes.len();
            let written = blocking(move || write_atomic(&at, &bytes).and_then(|()| std::fs::metadata(&at)?.modified())).await;
            me.edit(|e| match written {
                Ok(Ok(mtime)) => {
                    if let Some(doc) = e.docs.iter_mut().find(|d| d.buf.path.as_deref() == Some(path.as_path())) {
                        doc.buf.mark_saved(version, Some(mtime));
                    }
                    e.message(format!("\"{}\" {len}B written", path.display()));
                }
                Ok(Err(err)) => e.error(format!("{}: {err}", path.display())),
                Err(err) => e.error(format!("write: {err}")),
            });
        });
    }

    /// Carries out an action of a keymap. The port runs the ones a text area needs. The
    /// others say so on the command line.
    fn run(&self, action: Action) {
        match action {
            Action::Save(to) => self.save(to),
            Action::Open(path) => self.open(path),
            Action::Quit { .. } | Action::QuitAll { .. } => {
                nsh_core::element::desktop::current_window().close();
            }
            Action::Copy(text) => {
                // The write is done when the platform says so. Nothing waits for it.
                drop(set_clipboard(text));
            }
            Action::Message(text) => self.edit(|e| e.message(text)),
            Action::Error(text) => self.edit(|e| e.error(text)),
            other => {
                let name = format!("{other:?}");
                let name = name.split(['(', ' ', '{']).next().unwrap_or("").to_string();
                self.edit(|e| e.message(format!("{name}: not in the NSH port yet")));
            }
        }
    }
}

fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() { path.to_path_buf() } else { std::env::current_dir().map(|d| d.join(path)).unwrap_or_else(|_| path.into()) }
}

/// Writes `bytes` to a temporary file beside `path`, then renames it over `path`, so a crash
/// mid-write never leaves half a file. Keeps the old file's permissions.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = dir.join(format!(".{name}.editor-{}.tmp", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    if let Ok(meta) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(&tmp, meta.permissions());
    }
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}
