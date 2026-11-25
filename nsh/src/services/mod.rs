//! Work that never runs on the UI thread: reading and writing files, indexing the project,
//! git, ripgrep and watching the disk. Each worker sends [`Event`]s back on one channel and
//! wakes the window; `App::poll` drains them before the next frame.

pub mod editorconfig;
pub mod git;
pub mod grep;
pub mod index;
pub mod io;
pub mod ipc;
pub mod lsp;
pub mod probe;
pub mod watch;

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use editor::buffer::{Buffer, Pos};
use std::sync::Arc;

use crate::theme::Omarchy;
/// Wakes the UI thread after a worker sent an event. Any thread may call it.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

pub use editorconfig::EditorConfig;
pub use git::{GitInfo, Sign};
pub use grep::Hit;

/// What to do once a save lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterSave {
    Nothing,
    /// `:wq` and `ZZ`: close the buffer.
    Close,
}

/// A file the app writes that isn't a buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteKind {
    Config,
}

// Events move once each, from a worker to the UI thread; boxing the big ones buys nothing.
#[allow(clippy::large_enum_variant)]
pub enum Event {
    /// A file opened. `recovered` holds unsaved text a crash left in its swap file.
    /// `background` files open without switching to them (a rename's other files).
    Opened {
        path: PathBuf,
        buf: Result<Box<Buffer>, String>,
        at: Option<Pos>,
        ec: EditorConfig,
        recovered: Option<String>,
        background: bool,
    },
    Saved {
        path: PathBuf,
        version: u64,
        mtime: Option<std::time::SystemTime>,
        result: Result<(usize, usize), String>,
        then: AfterSave,
    },
    /// A file's text on disk, read for a reload after an outside change.
    DiskText {
        path: PathBuf,
        text: String,
        mtime: Option<std::time::SystemTime>,
    },
    Wrote {
        what: WriteKind,
        result: Result<(), String>,
    },
    /// The project's files, relative to the root, sorted, and the tree built from them off
    /// the UI thread.
    Files(Vec<String>, crate::app::tree::TreeIndex),
    Git(GitInfo),
    Signs {
        path: PathBuf,
        version: u64,
        signs: Vec<Sign>,
    },
    Grep {
        generation: u64,
        hits: Vec<Hit>,
        error: Option<String>,
    },
    /// Files changed on disk under the project or an open file.
    FsChanged(Vec<PathBuf>),
    /// Omarchy's theme changed (or went away).
    Omarchy(Option<Omarchy>),
    /// The config file changed on disk, read again: the config, or why it does not parse.
    Config(Result<crate::config::Config, String>),
    /// What the first-launch setup asked about this machine.
    Setup(probe::Answer),
    /// The old folders were copied to the new names: the line to show.
    Migrated(String),
    /// The start of a file, for the finder's preview.
    Preview {
        path: PathBuf,
        text: String,
    },
    /// A language server's diagnostics for a file, replacing the ones before.
    Diagnostics {
        path: PathBuf,
        diags: Vec<lsp::Diag>,
    },
    /// Where a definition is, or every reference.
    Locations {
        kind: &'static str,
        locs: Vec<lsp::Loc>,
    },
    Hover {
        path: PathBuf,
        text: String,
    },
    Completion {
        path: PathBuf,
        at: Pos,
        items: Vec<lsp::Item>,
    },
    /// A language server started or stopped.
    LspInfo(String),
    /// Text edits from a rename or a code action, by file.
    ApplyEdit {
        label: String,
        files: Vec<lsp::FileEdit>,
    },
    /// The code actions at the cursor, by title.
    CodeActions {
        titles: Vec<String>,
    },
    /// A request on the control socket, and where its reply goes.
    Ipc {
        req: crate::ipc::Request,
        reply: Sender<crate::ipc::Response>,
    },
    /// A `:{range}!cmd` filter finished: its output replaces lines `first..=last` of `path`
    /// when the buffer is still at `version`.
    Filtered {
        path: PathBuf,
        version: u64,
        first: usize,
        last: usize,
        result: Result<(i32, String, String), String>,
    },
    /// A `:!cmd` command finished.
    ShellDone {
        cmd: String,
        result: Result<(i32, String, String), String>,
    },
}

/// Handles to the workers. Built in `AppSpec::setup`, once the window's waker exists.
pub struct Services {
    pub events: Receiver<Event>,
    io: Sender<io::Job>,
    git: Sender<git::Job>,
    grep: Sender<grep::Job>,
    index: Sender<Vec<PathBuf>>,
    watch: Sender<watch::Job>,
    lsp: Sender<lsp::Job>,
    probe: Sender<probe::Job>,
    out: Out,
    root: PathBuf,
    /// The control socket this editor serves, when it serves one.
    pub socket: Option<PathBuf>,
}

impl Services {
    /// Starts every worker. With `ipc`, also serves the project's control socket.
    pub fn start(root: PathBuf, waker: Waker, ipc: bool) -> Services {
        let (tx, events) = channel();
        let out = Out { tx, waker };
        let socket = if ipc { ipc::start(&root, out.clone()) } else { None };
        Services {
            events,
            io: io::start(out.clone()),
            git: git::start(root.clone(), out.clone()),
            grep: grep::start(root.clone(), out.clone()),
            index: index::start(root.clone(), out.clone()),
            watch: watch::start(root.clone(), out.clone()),
            lsp: lsp::start(root.clone(), out.clone()),
            probe: probe::start(out.clone()),
            out,
            root,
            socket,
        }
    }

    /// Filters lines `first..=last` of `path` through `cmd`, off the UI thread.
    pub fn filter(&self, path: PathBuf, version: u64, first: usize, last: usize, cmd: String, input: String) {
        let out = self.out.clone();
        ipc::run_shell(cmd, self.root.clone(), Some(input), move |result| out.send(Event::Filtered { path, version, first, last, result }));
    }

    /// Runs `cmd` in the project folder, off the UI thread.
    pub fn shell(&self, cmd: String) {
        let out = self.out.clone();
        let c = cmd.clone();
        ipc::run_shell(cmd, self.root.clone(), None, move |result| out.send(Event::ShellDone { cmd: c, result }));
    }

    /// Removes the control socket; the process is about to end.
    pub fn close_socket(&self) {
        if let Some(p) = &self.socket {
            let _ = std::fs::remove_file(p);
        }
    }

    pub fn open(&self, path: PathBuf, at: Option<Pos>) {
        let _ = self.io.send(io::Job::Open { path, at });
    }

    pub fn save(&self, path: PathBuf, bytes: Vec<u8>, version: u64, then: AfterSave) {
        let _ = self.io.send(io::Job::Save { path, bytes, version, then });
    }

    pub fn read_disk(&self, path: PathBuf) {
        let _ = self.io.send(io::Job::ReadDisk { path });
    }

    pub fn open_edited(&self, path: PathBuf, edits: Vec<lsp::TextEdit>) {
        let _ = self.io.send(io::Job::OpenEdited { path, edits });
    }

    pub fn lsp(&self, job: lsp::Job) {
        let _ = self.lsp.send(job);
    }

    pub fn keep_history(&self, path: PathBuf, history: editor::buffer::History) {
        let _ = self.io.send(io::Job::History { path, history: Box::new(history) });
    }

    pub fn swap(&self, path: PathBuf, text: String) {
        let _ = self.io.send(io::Job::Swap { path, text });
    }

    pub fn unswap(&self, path: PathBuf) {
        let _ = self.io.send(io::Job::Unswap { path });
    }

    pub fn preview(&self, path: PathBuf) {
        let _ = self.io.send(io::Job::Preview { path });
    }

    pub fn write(&self, what: WriteKind, path: PathBuf, text: String) {
        let _ = self.io.send(io::Job::Write { what, path, text });
    }

    /// Tells the index these paths changed. It walks the project again when one came or went.
    pub fn files_changed(&self, paths: Vec<PathBuf>) {
        let _ = self.index.send(paths);
    }

    pub fn git_refresh(&self) {
        let _ = self.git.send(git::Job::Info);
    }

    pub fn git_signs(&self, path: PathBuf, text: String, version: u64) {
        let _ = self.git.send(git::Job::Signs { path, text, version });
    }

    pub fn grep(&self, pattern: String, generation: u64) {
        let _ = self.grep.send(grep::Job { pattern, generation });
    }

    /// Watches `path` (an open file outside the project) for outside changes.
    /// Copies the folders of the old name to the new one. The file worker runs it before any
    /// later job, so files open after the copy.
    pub fn migrate(&self, m: crate::migrate::Migration) {
        if !m.is_empty() {
            let _ = self.io.send(io::Job::Migrate(m));
        }
    }

    /// Finds what the setup screen needs about this machine. Sends `Event::Setup`.
    pub fn probe_setup(&self) {
        let _ = self.probe.send(probe::Job::Facts);
    }

    /// Looks command names up on `$PATH`. Sends `Event::Setup`.
    pub fn look_up(&self, names: Vec<String>) {
        if !names.is_empty() {
            let _ = self.probe.send(probe::Job::LookUp(names));
        }
    }

    /// Reads `file` again each time it changes and sends `Event::Config`.
    pub fn watch_config(&self, file: PathBuf) {
        let _ = self.watch.send(watch::Job::Config(file));
    }

    pub fn watch(&self, path: PathBuf) {
        let _ = self.watch.send(watch::Job::Add(path));
    }
}

/// A worker's way back to the UI thread: send, then wake the window.
#[derive(Clone)]
pub struct Out {
    tx: Sender<Event>,
    waker: Waker,
}

impl Out {
    /// Hands `e` to the UI thread through the channel `App::poll` drains, then wakes it. The
    /// shell polls even while the window draws nothing (on another workspace, covered), so
    /// agents and reloads never wait for the user to look.
    pub fn send(&self, e: Event) {
        if self.tx.send(e).is_ok() {
            (self.waker)();
        }
    }
}
