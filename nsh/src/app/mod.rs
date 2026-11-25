//! The app's state: the editing core plus everything around it (the tree, overlays, Settings,
//! setup, git, the project index). `poll` applies what the background workers send.

pub mod actions;
pub mod fx;
pub mod ipc;
pub mod key_list;
pub mod keys;
pub mod lifecycle;
#[cfg(test)]
mod lifecycle_tests;
pub mod lsp;
pub mod menu;
pub mod mouse;
pub mod picker;
pub mod review;
pub mod session;
pub mod settings;
pub mod setup;
pub mod tree;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::highlight::Lang;
use editor::buffer::{Buffer, Pos};
use editor::editor::{Doc, Editor, Keymap};
use fx::Tx;

use crate::config::{self, Config};
use crate::services::{AfterSave, EditorConfig, Event, GitInfo, Hit, Services, Sign, WriteKind};
use crate::theme::Omarchy;

pub use picker::{PickKind, Picker};
pub use settings::Settings;
pub use setup::Setup;

pub enum Screen {
    /// First launch: keys, mouse, look, terminal, ready.
    Setup(Box<Setup>),
    Editor,
}

/// What the app knows about an open file beyond its text.
#[derive(Clone, Debug, Default)]
pub struct DocMeta {
    pub lang: Option<Lang>,
    pub ec: EditorConfig,
    /// Git gutter signs and the buffer version they were computed for.
    pub signs: Vec<Sign>,
    pub signs_version: u64,
    /// The version last sent to the git worker, so each version is diffed once.
    pub signs_asked: u64,
    /// The version last written to the swap file.
    pub swapped: u64,
    /// The version last sent to the language server.
    pub lsp_sent: u64,
    /// The language server's diagnostics.
    pub diags: Vec<crate::services::lsp::Diag>,
    /// Lines changed on disk from outside the editor, since it last saved this file.
    pub outside: Vec<usize>,
    /// The file changed on disk while the buffer had unsaved edits.
    pub conflict: bool,
}

pub enum Overlay {
    None,
    /// The leader (Space) key menu, or one of its groups (`g` git, `l` lsp).
    Leader(Option<char>),
    Pick(Box<Picker>),
    /// The in-app context menu (Linux; other systems show theirs).
    Menu(menu::Menu),
}

/// The file tree's state.
pub struct Tree {
    pub show: bool,
    /// Folders open, by path relative to the root.
    pub open: HashSet<String>,
    /// The tree has the keyboard (`SPC e` twice, or a click on it).
    pub focus: bool,
    /// The row the keyboard is on, by path.
    pub sel: Option<String>,
    /// The first row on screen, and the part of a row the wheel has turned but not scrolled.
    pub top: usize,
    pub wheel_rest: f32,
    /// Whether the next frame scrolls to bring `sel` on screen.
    pub follow: tree::Follow,
    pub width: f32,
    /// Each folder's children, rebuilt when the project's files change.
    pub index: tree::TreeIndex,
}

/// Sizes the view measured, which key handling needs (rows for Ctrl-d, columns for clicks).
#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    /// The advance of one char of the editor font, in points.
    pub cw: f32,
    pub row_h: f32,
    /// Where the text area starts in the window, and its size.
    pub text: [f32; 4],
    /// The gutter's width (signs and numbers) in points.
    pub gutter: f32,
}

#[derive(Default)]
pub struct Grep {
    pub generation: u64,
    pub hits: Vec<Hit>,
    pub error: Option<String>,
    pub running: bool,
}

pub struct App {
    pub cfg: Config,
    /// The config as config.toml holds it, so Settings can show `[+]`.
    pub disk_cfg: Config,
    pub screen: Screen,
    pub ed: Editor,
    pub meta: HashMap<PathBuf, DocMeta>,
    pub tree: Tree,
    pub overlay: Overlay,
    /// The Settings buffer, while its tab is open.
    pub settings: Option<Settings>,
    /// The Settings tab is the one showing.
    pub on_settings: bool,
    pub files: Vec<String>,
    pub git: GitInfo,
    pub omarchy: Option<Omarchy>,
    pub svc: Option<Services>,
    pub m: Metrics,
    pub grep: Grep,
    /// Files to open once the workers start, with where to put the cursor.
    pub to_open: Vec<(PathBuf, Option<Pos>)>,
    /// The active document after the session's files open.
    pub restore_active: Option<PathBuf>,
    /// Saves still on their way to disk before `:wqa` can quit.
    pub quit_after_saves: Option<usize>,
    /// A text drag in progress started at this position.
    pub drag_from: Option<Pos>,
    /// When the last edit happened, for diffing against git after a pause.
    pub last_edit: f64,
    pub clock: f64,
    /// The editor font's CSS family list.
    pub font_family: &'static str,
    /// Wheel travel smaller than a line, kept for the next wheel event.
    pub wheel_rest: f32,
    /// A language server's hover text, shown by the cursor until the next key.
    pub hover: Option<String>,
    pub completion: Option<lsp::Completion>,
    /// References, definitions or diagnostics the Locations picker lists.
    pub locations: Vec<crate::services::lsp::Loc>,
    pub locations_title: &'static str,
    /// The window title last set, so it changes only when the active file does.
    pub title: String,
    /// The window's size in points and its position in desktop pixels, kept for next launch.
    pub window: session::WindowFrame,
    /// `open --wait` requests held until their buffer closes.
    pub waiters: Vec<ipc::Waiter>,
    /// Agent requests a scene plays at set times, for headless demos (`ipc::play_scripted`).
    pub scripted: Vec<(f64, crate::ipc::Request)>,
    /// Polls so far, counted for startup timing only.
    pub polls: u32,
    /// Lines an agent pointed at, with its note.
    pub note: Option<ipc::Note>,
    /// Changed blocks of files that changed on disk, newest first.
    pub outside: Vec<review::Hunk>,
    /// Changes agents proposed, waiting for y or n.
    pub proposals: Vec<review::Proposal>,
    /// An agent asked to bring the window forward, with its activation token on Wayland.
    pub raise: Option<Option<String>>,
    /// The config file the watcher reads again when it changes. None: no reload.
    pub config_file: Option<PathBuf>,
    /// When the user last asked to close the window over unsaved edits (`close_requested`).
    pub close_asked: Option<f64>,
    /// `exit` ran: the session is written.
    pub exited: bool,
    /// The view that takes every key has the keyboard focus.
    pub key_focus: bool,
    /// The old folders to copy to the new names, once the workers start.
    pub migration: crate::migrate::Migration,
}

impl App {
    /// The first state, before the window's first frame. Reads config.toml and the session:
    /// small files, read once at startup.
    pub fn init() -> App {
        // Only looks: the file worker copies the old folders (`crate::migrate`).
        let migration = crate::migrate::plan();
        let (cfg, first, notice) = match config::load() {
            config::Loaded::FirstLaunch => (Config::default(), true, None),
            config::Loaded::Ok(c) => (c, false, None),
            config::Loaded::Broken(c, e) => (c, false, Some(e)),
        };
        crate::timing::mark("config read");
        let (root, targets) = crate::args::targets();
        crate::timing::mark("project root found");
        let omarchy = crate::theme::read_omarchy();
        crate::timing::mark("omarchy theme read");
        let mut app = App::new(cfg, root.clone(), first, omarchy);
        app.config_file = Some(config::config_path());
        app.migration = migration;
        app.to_open = targets;
        app.restore_session();
        if let Some((p, _)) = app.to_open.first()
            && let Some(dir) = p.parent().and_then(|d| d.strip_prefix(&root).ok())
        {
            let mut acc = String::new();
            for c in dir.components() {
                if !acc.is_empty() {
                    acc.push('/');
                }
                acc.push_str(&c.as_os_str().to_string_lossy());
                app.tree.open.insert(acc.clone());
            }
        }
        if let Some(e) = notice {
            app.ed.error(e);
        }
        app
    }

    /// A state for `cfg` in project `root`, with nothing read from disk: setup first when
    /// `first`, else the editor with no file open.
    pub fn new(cfg: Config, root: PathBuf, first: bool, omarchy: Option<Omarchy>) -> App {
        let mut ed = Editor::new(root);
        ed.set_keymap(cfg.keymap());
        ed.vim.leader = cfg.leader();
        ed.opts = cfg.opts();
        let screen = if first { Screen::Setup(Box::new(Setup::new(omarchy.as_ref()))) } else { Screen::Editor };
        let font_family = crate::geometry::family(&cfg.font.family);
        App {
            disk_cfg: cfg.clone(),
            tree: Tree {
                show: true,
                open: HashSet::new(),
                focus: false,
                sel: None,
                top: 0,
                wheel_rest: 0.0,
                follow: tree::Follow::No,
                width: cfg.ui.tree_width,
                index: tree::TreeIndex::default(),
            },
            cfg,
            screen,
            ed,
            meta: HashMap::new(),
            overlay: Overlay::None,
            settings: None,
            on_settings: false,
            files: Vec::new(),
            git: GitInfo::default(),
            omarchy,
            svc: None,
            m: Metrics { cw: 7.8, row_h: 22.0, text: [0.0, 0.0, 800.0, 600.0], gutter: 60.0 },
            grep: Grep::default(),
            to_open: Vec::new(),
            restore_active: None,
            quit_after_saves: None,
            drag_from: None,
            last_edit: 0.0,
            clock: 0.0,
            font_family,
            wheel_rest: 0.0,
            hover: None,
            completion: None,
            locations: Vec::new(),
            locations_title: "References",
            title: String::new(),
            window: session::WindowFrame::default(),
            waiters: Vec::new(),
            polls: 0,
            scripted: Vec::new(),
            note: None,
            outside: Vec::new(),
            proposals: Vec::new(),
            raise: None,
            config_file: None,
            close_asked: None,
            exited: false,
            key_focus: false,
            migration: Default::default(),
        }
    }

    /// Starts the workers and opens the files asked for. Runs once, with the window's waker.
    pub fn setup(&mut self, waker: crate::services::Waker) {
        crate::timing::mark("window ready, starting workers");
        let svc = Services::start(self.ed.root.clone(), waker, self.cfg.ipc.enabled);
        svc.migrate(std::mem::take(&mut self.migration));
        crate::timing::mark("workers started");
        for (path, at) in self.to_open.drain(..) {
            svc.open(path, at);
        }
        if let Some(f) = &self.config_file {
            svc.watch_config(f.clone());
        }
        if matches!(self.screen, Screen::Setup(_)) {
            svc.probe_setup();
        }
        self.svc = Some(svc);
    }

    /// Applies what the workers sent. True when anything visible changed.
    pub fn poll(&mut self, clock: f64, tx: &mut Tx) -> bool {
        let mut changed = false;
        if self.polls < 3 {
            self.polls += 1;
            crate::timing::mark(&format!("poll {} (frame {} is next)", self.polls, self.polls));
            // The second frame draws the unseen glyph warm-up (`ui::warm_glyphs`).
            changed = self.polls == 2;
        }
        self.clock = clock;
        let events: Vec<Event> = match &self.svc {
            Some(svc) => svc.events.try_iter().collect(),
            None => Vec::new(),
        };
        for e in events {
            changed = true;
            self.event(e, tx);
        }
        changed |= self.play_scripted(clock, tx);
        if self.ask_signs(clock, tx) {
            changed = true;
        }
        self.write_swaps(clock, tx);
        self.lsp_sync(clock, tx);
        self.check_waiters();
        self.refresh_proposals();
        let title = match (&self.screen, self.on_settings, self.ed.has_doc()) {
            (Screen::Setup(_), _, _) => "setup — nsvim".to_string(),
            (_, true, _) => "settings — nsvim".into(),
            (_, _, true) => format!("{}{} — nsvim", self.ed.buf().name(), if self.ed.buf().is_dirty() { " [+]" } else { "" }),
            _ => "nsvim".into(),
        };
        if title != self.title {
            tx.set_title(title.clone());
            self.title = title;
        }
        changed
    }

    /// Writes unsaved text to swap files once editing has paused for two seconds.
    fn write_swaps(&mut self, clock: f64, tx: &mut Tx) {
        let Some(svc) = &self.svc else { return };
        let idle = clock - self.last_edit;
        for d in &self.ed.docs {
            let Some(path) = &d.buf.path else { continue };
            let Some(m) = self.meta.get_mut(path) else { continue };
            if !d.buf.is_dirty() || m.swapped == d.buf.version() + 1 {
                continue;
            }
            if d.buf.len_bytes() > 4 * BIG {
                continue;
            }
            if idle < 2.0 {
                tx.wake_after(Duration::from_secs_f64(2.05 - idle));
                continue;
            }
            m.swapped = d.buf.version() + 1;
            svc.swap(path.clone(), d.buf.text());
        }
    }

    pub fn event(&mut self, e: Event, tx: &mut Tx) {
        self.handle_event(e, tx);
        // Events arrive as app updates while the window draws nothing, so `open --wait` must
        // see a buffer open and close here, not only in `poll`.
        self.check_waiters();
    }

    fn handle_event(&mut self, e: Event, tx: &mut Tx) {
        match e {
            Event::Opened { path, buf, at, ec, recovered, background } => match buf {
                Ok(buf) => {
                    let keep = (background && self.ed.has_doc()).then_some(self.ed.active);
                    self.opened(path.clone(), *buf, at, ec);
                    if let Some(i) = keep {
                        self.ed.active = i;
                        self.apply_doc_opts();
                    }
                    if let (Some(text), Some(i)) = (recovered, self.doc_index(&path)) {
                        self.ed.docs[i].buf.recover(&text);
                        let name = self.rel(&path);
                        self.ed.error(format!("E325: recovered unsaved edits to \"{name}\" from a crash. :w keeps them, :e! drops them"));
                    }
                }
                Err(e) => {
                    // An `open --wait` on a file that failed to open gets the error, not a hang.
                    let (failed, keep): (Vec<_>, Vec<_>) =
                        std::mem::take(&mut self.waiters).into_iter().partition(|w| w.path == path && !w.seen);
                    self.waiters = keep;
                    for w in failed {
                        let _ = w.reply.send(crate::ipc::Response::err(w.id, e.clone()));
                    }
                    self.ed.error(e)
                }
            },
            Event::Ipc { req, reply } => self.ipc(req, reply, tx),
            Event::Filtered { path, version, first, last, result } => self.filtered(path, version, first, last, result),
            Event::ShellDone { cmd, result } => match result {
                Ok((code, out, err)) => {
                    let text = if out.trim().is_empty() { err } else { out };
                    let last = text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
                    if code == 0 {
                        self.ed.message(format!(":!{cmd}  {last}"));
                    } else {
                        self.ed.error(format!(":!{cmd} exited {code}: {last}"));
                    }
                }
                Err(e) => self.ed.error(e),
            },
            Event::Saved { path, version, mtime, result, then } => self.saved(path, version, mtime, result, then, tx),
            Event::DiskText { path, text, mtime } => self.disk_text(path, text, mtime),
            Event::Wrote { what, result } => match (what, result) {
                (WriteKind::Config, Ok(())) => {
                    let path = config::config_path();
                    self.ed.message(format!("\"{}\" written", tilde(&path)));
                    self.saved_one(tx);
                }
                (_, Err(e)) => {
                    self.ed.error(e);
                    self.quit_after_saves = None;
                }
            },
            Event::Files(files, index) => {
                self.tree.index = index;
                self.files = files;
                if let Overlay::Pick(p) = &mut self.overlay
                    && p.kind == PickKind::Files
                {
                    p.refilter(&self.files);
                }
                self.ask_preview();
            }
            Event::Git(info) => self.git = info,
            Event::Signs { path, version, signs } => {
                if let Some(m) = self.meta.get_mut(&path)
                    && version >= m.signs_version
                {
                    m.signs = signs;
                    m.signs_version = version;
                }
            }
            Event::Grep { generation, hits, error } => {
                if generation == self.grep.generation {
                    self.grep.hits = hits;
                    self.grep.error = error;
                    self.grep.running = false;
                    if let Overlay::Pick(p) = &mut self.overlay
                        && p.kind == PickKind::Grep
                    {
                        p.sel = p.sel.min(self.grep.hits.len().saturating_sub(1));
                    }
                    self.ask_preview();
                }
            }
            Event::FsChanged(paths) => self.fs_changed(paths),
            e @ (Event::Diagnostics { .. }
            | Event::Locations { .. }
            | Event::Hover { .. }
            | Event::Completion { .. }
            | Event::LspInfo(_)
            | Event::ApplyEdit { .. }
            | Event::CodeActions { .. }) => self.lsp_event(e),
            Event::Preview { path, text } => {
                let want = self.picked().map(|(p, _)| p);
                if let Overlay::Pick(p) = &mut self.overlay
                    && want.as_deref() == Some(path.as_path())
                {
                    p.preview = Some((path, text));
                }
            }
            Event::Config(loaded) => self.config_reloaded(loaded),
            Event::Setup(answer) => self.setup_answer(answer),
            Event::Migrated(line) => self.ed.message(line),
            Event::Omarchy(o) => {
                if o.is_some() {
                    self.omarchy = o;
                }
            }
        }
    }

    fn opened(&mut self, path: PathBuf, buf: Buffer, at: Option<Pos>, ec: EditorConfig) {
        let lang = crate::highlight::lang_for(Some(&path));
        let idx = match self.ed.docs.iter().position(|d| d.buf.path.as_deref() == Some(path.as_path())) {
            Some(i) => i,
            None => {
                self.ed.docs.push(Doc::new(buf));
                self.ed.docs.len() - 1
            }
        };
        let restore = self.restore_active.as_deref() == Some(path.as_path());
        if self.restore_active.is_none() || restore {
            self.ed.active = idx;
            self.on_settings = false;
            if restore {
                self.restore_active = None;
            }
        }
        if !path.starts_with(&self.ed.root)
            && let Some(svc) = &self.svc
        {
            svc.watch(path.clone());
        }
        self.meta.insert(path.clone(), DocMeta { lang: Some(lang), ec, ..Default::default() });
        if let Some(at) = at {
            let prev = self.ed.active;
            self.ed.active = idx;
            self.apply_doc_opts();
            self.ed.set_cursor(at);
            let third = self.ed.rows / 3;
            let doc = self.ed.doc_mut();
            doc.view.top = at.line.saturating_sub(third);
            self.ed.active = prev;
        }
        self.reveal_in_tree(&path);
        self.apply_doc_opts();
        self.lsp_open(&path);
    }

    fn saved(
        &mut self,
        path: PathBuf,
        version: u64,
        mtime: Option<std::time::SystemTime>,
        result: Result<(usize, usize), String>,
        then: AfterSave,
        tx: &mut Tx,
    ) {
        match result {
            Ok((lines, bytes)) => {
                if let Some(i) = self.doc_index(&path) {
                    self.ed.docs[i].buf.mark_saved(version, mtime);
                    if let Some(m) = self.meta.get_mut(&path) {
                        m.conflict = false;
                        m.swapped = 0;
                    }
                    if let Some(svc) = &self.svc {
                        svc.lsp(crate::services::lsp::Job::Saved { path: path.clone() });
                        svc.unswap(path.clone());
                        self.outside.retain(|h| h.path != path);
                        if self.ed.docs[i].buf.version() == version {
                            svc.keep_history(path.clone(), self.ed.docs[i].buf.history());
                        }
                    }
                    if then == AfterSave::Close {
                        self.ed.close(i);
                        if self.ed.docs.is_empty() {
                            self.quit(tx);
                        }
                    }
                }
                self.ed.message(format!("\"{}\" {lines}L, {} written", self.rel(&path), human_bytes(bytes)));
                if let Some(svc) = &self.svc {
                    svc.git_refresh();
                }
            }
            Err(e) => {
                self.ed.error(e);
                self.quit_after_saves = None;
            }
        }
        self.saved_one(tx);
    }

    /// Files changed on disk. Open ones are read back by the file worker (a reload compares
    /// text, so the editor's own saves change nothing). The index worker decides whether files
    /// came or went. No file is touched on this thread.
    fn fs_changed(&mut self, paths: Vec<PathBuf>) {
        let Some(svc) = &self.svc else { return };
        let mut project = Vec::new();
        for p in paths {
            if self.doc_index(&p).is_some() {
                svc.read_disk(p.clone());
            }
            if !p.components().any(|c| c.as_os_str() == ".git") {
                project.push(p);
            }
        }
        if !project.is_empty() {
            svc.files_changed(project);
        }
        svc.git_refresh();
    }

    /// Asks git for the active buffer's signs once it has rested for 300 ms after an edit.
    fn ask_signs(&mut self, clock: f64, tx: &mut Tx) -> bool {
        if !self.ed.has_doc() || self.on_settings {
            return false;
        }
        let doc = self.ed.doc();
        let Some(path) = doc.buf.path.clone() else { return false };
        let version = doc.buf.version();
        let Some(m) = self.meta.get_mut(&path) else { return false };
        if m.signs_asked == version + 1 {
            return false;
        }
        if version != m.signs_version && clock - self.last_edit < 0.3 && version > 0 {
            tx.wake_after(Duration::from_secs_f64(0.32 - (clock - self.last_edit)));
            return false;
        }
        m.signs_asked = version + 1;
        if doc.buf.len_bytes() > BIG {
            return false;
        }
        if let Some(svc) = &self.svc {
            svc.git_signs(path, doc.buf.text(), version);
        }
        false
    }

    pub fn doc_index(&self, path: &Path) -> Option<usize> {
        self.ed.docs.iter().position(|d| d.buf.path.as_deref() == Some(path))
    }

    /// `path` relative to the project root, or with `~` for the home folder.
    pub fn rel(&self, path: &Path) -> String {
        match path.strip_prefix(&self.ed.root) {
            Ok(r) => r.to_string_lossy().into_owned(),
            Err(_) => tilde(path),
        }
    }

    /// Applies config.toml's options, then the active file's `.editorconfig` on top.
    pub fn apply_doc_opts(&mut self) {
        let mut opts = self.cfg.opts();
        (opts.ignorecase, opts.smartcase) = (self.ed.opts.ignorecase, self.ed.opts.smartcase);
        if self.ed.has_doc()
            && let Some(path) = &self.ed.doc().buf.path
            && let Some(m) = self.meta.get(path)
        {
            if let Some(s) = m.ec.indent_spaces {
                opts.spaces = s;
            }
            if let Some(n) = m.ec.indent_size {
                opts.tab_width = n;
            }
            if let Some(t) = m.ec.trim_trailing_whitespace {
                opts.save.trim_trailing_whitespace = t;
            }
            if let Some(f) = m.ec.insert_final_newline {
                opts.save.final_newline = f;
            }
        }
        self.ed.opts = opts;
    }

    /// Re-applies the config to the editor after Settings or `:set` changed it.
    pub fn config_changed(&mut self) {
        let map = self.cfg.keymap();
        if map != self.ed.keymap {
            self.ed.set_keymap(map);
        }
        self.ed.vim.leader = self.cfg.leader();
        self.font_family = crate::geometry::family(&self.cfg.font.family);
        self.tree.width = self.cfg.ui.tree_width;
        self.apply_doc_opts();
    }

    /// Opens the folders above `path` in the tree.
    pub fn reveal_in_tree(&mut self, path: &Path) {
        let Ok(rel) = path.strip_prefix(&self.ed.root) else { return };
        self.settle_tree_top();
        self.tree.follow = tree::Follow::Center;
        let mut acc = String::new();
        if let Some(parent) = rel.parent() {
            for c in parent.components() {
                if !acc.is_empty() {
                    acc.push('/');
                }
                acc.push_str(&c.as_os_str().to_string_lossy());
                self.tree.open.insert(acc.clone());
            }
        }
        self.tree.sel = Some(rel.to_string_lossy().replace('\\', "/"));
    }

    /// Opens `path` (relative to the root, or absolute), or switches to it when it is open.
    pub fn open_path(&mut self, path: PathBuf, at: Option<Pos>) {
        let path = if path.is_absolute() { path } else { self.ed.root.join(path) };
        self.on_settings = false;
        if let Some(i) = self.doc_index(&path) {
            self.ed.active = i;
            self.apply_doc_opts();
            if let Some(at) = at {
                self.ed.set_cursor(at);
            }
            self.reveal_in_tree(&path);
            return;
        }
        if let Some(svc) = &self.svc {
            svc.open(path, at);
        }
    }

    /// Writes the session and quits.
    pub fn quit(&mut self, tx: &mut Tx) {
        self.exit();
        tx.quit();
    }

    /// A `:{range}!cmd` filter finished: replace the lines with its output, as one undo step.
    fn filtered(&mut self, path: PathBuf, version: u64, first: usize, last: usize, result: Result<(i32, String, String), String>) {
        let (code, out, err) = match result {
            Ok(r) => r,
            Err(e) => return self.ed.error(e),
        };
        if code != 0 {
            let why = err.lines().next().unwrap_or("").to_string();
            return self.ed.error(format!("the filter exited {code}, nothing changed: {why}"));
        }
        let Some(i) = self.doc_index(&path) else { return };
        let doc = &mut self.ed.docs[i];
        if doc.buf.version() != version {
            return self.ed.error("the buffer changed while the filter ran, nothing changed");
        }
        let last = last.min(doc.buf.last_line());
        let a = doc.buf.idx(editor::buffer::Pos::new(first, 0));
        let b = if last < doc.buf.last_line() { doc.buf.idx(editor::buffer::Pos::new(last + 1, 0)) } else { doc.buf.len_chars() };
        let ends_open = b == doc.buf.len_chars() && !doc.buf.slice(a, b).ends_with('\n');
        let text = if ends_open { out.strip_suffix('\n').unwrap_or(&out).to_string() } else { out };
        let at = doc.view.cursor;
        doc.buf.begin(at);
        doc.buf.replace(a, b, &text);
        doc.buf.commit(editor::buffer::Pos::new(first, 0));
        doc.view.cursor = doc.buf.clamp(editor::buffer::Pos::new(first, 0), false);
        let n = text.lines().count();
        self.edited();
        self.ed.message(format!("{} lines filtered into {n}", last - first + 1));
    }

    /// Diffs proposals made for files that were not open yet, once they are.
    fn refresh_proposals(&mut self) {
        for i in 0..self.proposals.len() {
            if self.proposals[i].base != u64::MAX {
                continue;
            }
            if let Some(d) = self.doc_index(&self.proposals[i].path.clone()) {
                let (old, v) = (self.ed.docs[d].buf.text(), self.ed.docs[d].buf.version());
                self.proposals[i].rediff(&old, v);
            }
        }
    }

    pub fn is_vim(&self) -> bool {
        self.ed.keymap == Keymap::Vim
    }

    /// Marks that an edit happened now (for the git gutter's debounce).
    pub fn edited(&mut self) {
        self.last_edit = self.clock;
    }

    /// Whether `/` matches are highlighted.
    pub fn search_shown(&self) -> bool {
        !self.ed.search.pattern.is_empty() && self.ed.vim.hlsearch
    }
}

/// Applies LSP text edits to a buffer as one undo step, last edit first so earlier positions
/// hold.
pub fn apply_edits(buf: &mut Buffer, edits: &[crate::services::lsp::TextEdit]) {
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

/// Buffers above this many bytes skip git signs and LSP: copying their text on every pause
/// would stall the UI thread. Swap files stop at four times this.
pub const BIG: usize = 4 * 1024 * 1024;

/// `1.4KB`, `52B`, `3.1MB`.
pub fn human_bytes(n: usize) -> String {
    if n < 1024 {
        format!("{n}B")
    } else if n < 1024 * 1024 {
        format!("{:.1}KB", n as f64 / 1024.0)
    } else {
        format!("{:.1}MB", n as f64 / (1024.0 * 1024.0))
    }
}

/// `path` with the home folder written as `~`.
pub fn tilde(path: &Path) -> String {
    match dirs::home_dir().and_then(|h| path.strip_prefix(&h).ok().map(Path::to_path_buf)) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}
