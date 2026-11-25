//! The services of the web build. A browser page has no threads, no files and no processes. So
//! the project is a demo that lives in memory (`Disk`), and each request answers at once: its
//! event goes on the same channel as a worker's, and the waker makes the app poll it.
//!
//! `Disk` compiles on every target, so its tests run with the others. `Services` here replaces
//! the native one only in the web build.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::git::{GitInfo, Sign};
use super::grep::Hit;

/// Where the demo project is. Paths in the editor start here.
pub const ROOT: &str = "/demo";

/// The file the demo opens first.
pub const FIRST: &str = "README.md";

/// The demo project's files, by path from `ROOT`.
const FILES: &[(&str, &str)] = &[
    ("Cargo.toml", include_str!("../../web/demo/Cargo.toml")),
    ("README.md", include_str!("../../web/demo/README.md")),
    ("src/main.rs", include_str!("../../web/demo/src/main.rs")),
    ("src/note.rs", include_str!("../../web/demo/src/note.rs")),
];

/// Hits a project search finds at most, as the native search.
const MAX_HITS: usize = 2000;

/// The demo project in memory: the text of each file as the editor last wrote it.
pub struct Disk {
    files: BTreeMap<String, String>,
}

impl Default for Disk {
    fn default() -> Self {
        Disk::demo()
    }
}

impl Disk {
    pub fn demo() -> Disk {
        Disk { files: FILES.iter().map(|(p, t)| (p.to_string(), t.to_string())).collect() }
    }

    /// `path` from the root, with `/` between its parts.
    pub fn rel(path: &Path) -> Option<String> {
        let rel = path.strip_prefix(ROOT).ok()?.to_string_lossy().replace('\\', "/");
        (!rel.is_empty()).then_some(rel)
    }

    pub fn read(&self, path: &Path) -> Option<&str> {
        self.files.get(&Disk::rel(path)?).map(String::as_str)
    }

    /// Keeps `text` as the file at `path`. False for a path outside the project.
    pub fn write(&mut self, path: &Path, text: String) -> bool {
        let Some(rel) = Disk::rel(path) else { return false };
        self.files.insert(rel, text);
        true
    }

    /// Every file, from the root, sorted.
    pub fn list(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }

    /// The lines that hold `pattern`: its case counts only when it has an uppercase letter.
    pub fn grep(&self, pattern: &str) -> Vec<Hit> {
        if pattern.is_empty() {
            return Vec::new();
        }
        let smart = pattern.chars().any(char::is_uppercase);
        let needle = if smart { pattern.to_string() } else { pattern.to_lowercase() };
        let mut hits = Vec::new();
        for (rel, text) in &self.files {
            for (i, line) in text.lines().enumerate() {
                let hay = if smart { line.to_string() } else { line.to_lowercase() };
                if let Some(b) = hay.find(&needle) {
                    hits.push(Hit { path: rel.clone(), line: i, col: hay[..b].chars().count(), text: line.to_string() });
                    if hits.len() >= MAX_HITS {
                        return hits;
                    }
                }
            }
        }
        hits
    }

    /// The git signs of `text` as the file at `path`, against the demo's first text.
    pub fn signs(path: &Path, text: &str) -> Vec<Sign> {
        let Some(rel) = Disk::rel(path) else { return Vec::new() };
        let base = FILES.iter().find(|(p, _)| *p == rel).map_or("", |(_, t)| t);
        super::git::diff_signs(base, text)
    }

    /// What git would say: the branch, and the files changed since the demo started.
    pub fn git(&self) -> GitInfo {
        let mut status = HashMap::new();
        for (rel, text) in &self.files {
            match FILES.iter().find(|(p, _)| p == rel) {
                Some((_, t)) if t == text => {}
                Some(_) => {
                    status.insert(rel.clone(), 'M');
                }
                None => {
                    status.insert(rel.clone(), 'U');
                }
            }
        }
        let mut dirs = HashSet::new();
        for rel in status.keys() {
            let mut at = rel.as_str();
            while let Some((dir, _)) = at.rsplit_once('/') {
                dirs.insert(dir.to_string());
                at = dir;
            }
        }
        GitInfo { branch: Some("main".into()), status, dirs }
    }
}

/// The app of the web build: the editor on the demo project, with the default config and the
/// first file open. No setup, no control socket.
pub fn demo_app() -> crate::app::App {
    let mut cfg = crate::config::Config::default();
    cfg.ipc.enabled = false;
    let root = PathBuf::from(ROOT);
    let mut app = crate::app::App::new(cfg, root.clone(), false, None);
    app.to_open = vec![(root.join(FIRST), None)];
    app.tree.open.insert("src".into());
    app
}

#[cfg(target_arch = "wasm32")]
pub use page::Services;

#[cfg(target_arch = "wasm32")]
mod page {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::mpsc::{Receiver, channel};

    use editor::buffer::{Buffer, Pos};

    use super::Disk;
    use crate::services::{AfterSave, EditorConfig, Event, Out, WriteKind, lsp};

    /// A preview shows at most this much of a file.
    const PREVIEW_BYTES: usize = 64 * 1024;

    /// The web build's services: the same calls as the native workers take, answered at once
    /// from the demo project in memory.
    pub struct Services {
        pub events: Receiver<Event>,
        /// The web serves no control socket.
        pub socket: Option<PathBuf>,
        out: Out,
        disk: RefCell<Disk>,
    }

    impl Services {
        pub fn start(_root: PathBuf, waker: crate::services::Waker, _ipc: bool) -> Services {
            let (tx, events) = channel();
            let s = Services { events, socket: None, out: Out { tx, waker }, disk: RefCell::new(Disk::demo()) };
            s.send_files();
            s.git_refresh();
            s
        }

        fn send_files(&self) {
            let files = self.disk.borrow().list();
            let index = crate::app::tree::TreeIndex::build(&files);
            self.out.send(Event::Files(files, index));
        }

        fn no_shell(&self) -> Result<(i32, String, String), String> {
            Err("the web has no shell: commands run only in the desktop app".into())
        }

        pub fn filter(&self, path: PathBuf, version: u64, first: usize, last: usize, _cmd: String, _input: String) {
            self.out.send(Event::Filtered { path, version, first, last, result: self.no_shell() });
        }

        pub fn shell(&self, cmd: String) {
            self.out.send(Event::ShellDone { cmd, result: self.no_shell() });
        }

        pub fn close_socket(&self) {}

        pub fn open(&self, path: PathBuf, at: Option<Pos>) {
            let mut buf = match self.disk.borrow().read(&path) {
                Some(text) => Buffer::from_str(text),
                None => Buffer::empty(),
            };
            buf.path = Some(path.clone());
            let buf = Ok(Box::new(buf));
            self.out.send(Event::Opened { path, buf, at, ec: EditorConfig::default(), recovered: None, background: false });
        }

        pub fn save(&self, path: PathBuf, bytes: Vec<u8>, version: u64, then: AfterSave) {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let lines = text.lines().count();
            let new = self.disk.borrow().read(&path).is_none();
            let result = if self.disk.borrow_mut().write(&path, text) {
                Ok((lines, bytes.len()))
            } else {
                Err(format!("\"{}\" E212: Can't open file for writing: the web demo keeps only its own files", path.display()))
            };
            self.out.send(Event::Saved { path, version, mtime: None, result, then });
            if new {
                self.send_files();
            }
            self.git_refresh();
        }

        pub fn read_disk(&self, path: PathBuf) {
            if let Some(text) = self.disk.borrow().read(&path) {
                self.out.send(Event::DiskText { path: path.clone(), text: text.to_string(), mtime: None });
            }
        }

        /// Edits from a language server: the web runs none.
        pub fn open_edited(&self, _path: PathBuf, _edits: Vec<lsp::TextEdit>) {}

        /// The web runs no language server.
        pub fn lsp(&self, _job: lsp::Job) {}

        /// Undo history and swap files need a disk. The page keeps the buffers while it lives.
        pub fn keep_history(&self, _path: PathBuf, _history: editor::buffer::History) {}

        pub fn swap(&self, _path: PathBuf, _text: String) {}

        pub fn unswap(&self, _path: PathBuf) {}

        pub fn preview(&self, path: PathBuf) {
            let Some(text) = self.disk.borrow().read(&path).map(|t| t.chars().take(PREVIEW_BYTES).collect()) else { return };
            self.out.send(Event::Preview { path, text });
        }

        /// The config file: the web keeps the config in memory only.
        pub fn write(&self, what: WriteKind, _path: PathBuf, _text: String) {
            self.out.send(Event::Wrote { what, result: Ok(()) });
        }

        pub fn files_changed(&self, _paths: Vec<PathBuf>) {
            self.send_files();
        }

        pub fn git_refresh(&self) {
            self.out.send(Event::Git(self.disk.borrow().git()));
        }

        pub fn git_signs(&self, path: PathBuf, text: String, version: u64) {
            let signs = Disk::signs(&path, &text);
            self.out.send(Event::Signs { path, version, signs });
        }

        pub fn grep(&self, pattern: String, generation: u64) {
            let hits = self.disk.borrow().grep(&pattern);
            self.out.send(Event::Grep { generation, hits, error: None });
        }

        /// The web has no old folders to copy.
        pub fn migrate(&self, _m: crate::migrate::Migration) {}

        /// The web has no setup screen.
        pub fn probe_setup(&self) {}
        pub fn check_update(&self, _manual: bool) {}

        /// The web has no files that change from outside.
        pub fn watch_config(&self, _file: PathBuf) {}

        pub fn watch(&self, _path: PathBuf) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(rel: &str) -> PathBuf {
        Path::new(ROOT).join(rel)
    }

    #[test]
    fn the_demo_has_a_rust_file_a_readme_and_a_manifest() {
        let disk = Disk::demo();
        assert_eq!(disk.list(), ["Cargo.toml", "README.md", "src/main.rs", "src/note.rs"]);
        assert!(disk.read(&at(FIRST)).is_some_and(|t| t.starts_with("# notes")));
        assert!(disk.read(&at("src/main.rs")).is_some_and(|t| t.contains("fn main")));
        assert_eq!(disk.read(Path::new("/elsewhere/a.rs")), None);
    }

    #[test]
    fn the_demo_starts_in_the_editor_with_vim_keys_and_the_first_file() {
        let app = demo_app();
        assert!(matches!(app.screen, crate::app::Screen::Editor), "the web has no setup screen");
        assert!(app.is_vim());
        assert!(!app.cfg.ipc.enabled);
        assert_eq!(app.to_open, [(at(FIRST), None)]);
    }

    #[test]
    fn a_write_changes_the_file_and_git_marks_it() {
        let mut disk = Disk::demo();
        assert!(disk.git().status.is_empty());
        assert!(disk.write(&at("src/note.rs"), "changed\n".into()));
        assert!(disk.write(&at("src/new.rs"), "new\n".into()));
        assert!(!disk.write(Path::new("/elsewhere/a.rs"), String::new()));
        let git = disk.git();
        assert_eq!(git.status.get("src/note.rs"), Some(&'M'));
        assert_eq!(git.status.get("src/new.rs"), Some(&'U'));
        assert!(git.dirs.contains("src"));
        assert_eq!(disk.read(&at("src/note.rs")), Some("changed\n"));
    }

    #[test]
    fn grep_finds_lines_with_smart_case() {
        let disk = Disk::demo();
        let hits = disk.grep("note::parse");
        assert!(hits.iter().any(|h| h.path == "src/main.rs" && h.text.contains("Note::parse")), "lowercase ignores case: {hits:?}");
        assert!(disk.grep("NOTE::parse").is_empty(), "an uppercase letter makes case count");
    }

    #[test]
    fn signs_compare_with_the_first_text() {
        let text = format!("// new\n{}", FILES[2].1);
        let signs = Disk::signs(&at("src/main.rs"), &text);
        assert_eq!(signs.len(), 1);
        assert_eq!(signs[0].line, 0);
    }
}
