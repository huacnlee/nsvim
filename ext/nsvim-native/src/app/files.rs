//! File commands for the tree and the command line: make a file or a folder, move or rename,
//! delete, and open a folder as the project. The file worker does the disk work
//! (`services::io::FsOp`). This thread only asks for it and follows the result.

use std::path::{Path, PathBuf};

use editor::editor::{Action, KeyCode, KeyInput};

use super::App;
use super::fx::Tx;
use super::tree::Follow;
use crate::services::io::FsOp;

impl App {
    /// What the file commands work on: the tree's row when the tree has the focus, else the
    /// active file.
    pub fn file_target(&self) -> Option<PathBuf> {
        if self.tree.focus && self.tree.show {
            return self.tree.sel.as_ref().map(|s| self.ed.root.join(s));
        }
        if self.ed.has_doc() { self.ed.buf().path.clone() } else { None }
    }

    /// Whether `rel` (relative to the root, with `/`) is a folder the tree knows.
    fn known_folder(&self, rel: &str) -> bool {
        self.tree.index.children.contains_key(rel)
    }

    /// The folder that new files go in, relative to the root and with a `/` at the end: the
    /// tree's folder row, or the folder of its file row. Empty for the root.
    fn new_file_folder(&self) -> String {
        let Some(sel) = self.file_target().and_then(|p| self.rel_slash(&p)) else { return String::new() };
        let dir = if self.known_folder(&sel) { sel.as_str() } else { sel.rsplit_once('/').map_or("", |(d, _)| d) };
        if dir.is_empty() { String::new() } else { format!("{dir}/") }
    }

    /// `path` relative to the root, with `/`. None for a path outside the project.
    fn rel_slash(&self, path: &Path) -> Option<String> {
        path.strip_prefix(&self.ed.root).ok().map(|r| r.to_string_lossy().replace('\\', "/"))
    }

    /// A path the user typed: `~/…`, absolute, or relative to the project.
    pub fn typed_path(&self, arg: &str) -> PathBuf {
        let arg = arg.trim();
        if let Some(rest) = arg.strip_prefix("~/").or(if arg == "~" { Some("") } else { None })
            && let Some(home) = dirs::home_dir()
        {
            return home.join(rest);
        }
        let p = Path::new(arg);
        if p.is_absolute() { p.to_path_buf() } else { normal(&self.ed.root.join(p)) }
    }

    /// Opens the command line with `text` in it, for the user to finish and send with Enter.
    fn ask(&mut self, text: &str) {
        editor::vim::open_cmdline(&mut self.ed, ':', text);
    }

    /// The tree's file keys. False when `k` is not one of them.
    pub fn tree_file_key(&mut self, k: KeyInput, tx: &mut Tx) -> bool {
        // A delete waits for `y`. Any other key keeps the file.
        if let Some(path) = self.tree.confirm.take() {
            if k.code == KeyCode::Char('y') && !k.ctrl && !k.cmd {
                self.delete(path, true);
            } else {
                self.ed.message("nothing deleted");
            }
            return true;
        }
        if k.ctrl || k.cmd {
            return false;
        }
        let sel = self.tree.sel.clone();
        match k.code {
            KeyCode::Char('a') => {
                let dir = self.new_file_folder();
                self.ask(&format!("NewFile {dir}"));
            }
            KeyCode::Char('A') => {
                let dir = self.new_file_folder();
                self.ask(&format!("Mkdir {dir}"));
            }
            KeyCode::Char('r' | 'm') | KeyCode::F(2) => {
                let Some(sel) = sel else { return true };
                self.ask(&format!("Move {sel}"));
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                let Some(sel) = sel else { return true };
                let path = self.ed.root.join(&sel);
                let what = if self.known_folder(&sel) { "the folder" } else { "" };
                let dirty = if self.dirty_under(&path).is_empty() { "" } else { " It has unsaved changes." };
                let what = if what.is_empty() { format!("\"{sel}\"") } else { format!("{what} \"{sel}\" and all in it") };
                self.ed.message(format!("Delete {what}?{dirty} y to delete, any other key to keep it"));
                self.tree.confirm = Some(path);
            }
            KeyCode::Char('x') => {
                let Some(sel) = sel else { return true };
                self.ed.message(format!("\"{sel}\" cut. Press p on a folder to move it there"));
                self.tree.cut = Some(self.ed.root.join(sel));
            }
            KeyCode::Char('p') => {
                let Some(from) = self.tree.cut.take() else {
                    self.ed.message("nothing cut. Press x on a row first");
                    return true;
                };
                let dir = self.new_file_folder();
                let name = from.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let to = self.ed.root.join(format!("{dir}{name}"));
                if to != from {
                    self.fs(FsOp::Move { from, to });
                }
            }
            KeyCode::Char('C') => {
                let Some(sel) = sel.filter(|s| self.known_folder(s)) else {
                    self.ed.message("C opens a folder as the project. Pick a folder row");
                    return true;
                };
                self.open_project(self.ed.root.join(sel), tx);
            }
            _ => return false,
        }
        true
    }

    /// Carries out a file command from the command line.
    pub fn file_action(&mut self, action: Action, tx: &mut Tx) {
        match action {
            Action::OpenFolder(dir) => {
                let root = self.typed_path(&dir);
                self.open_project(root, tx);
            }
            Action::NewFile(p) if p.ends_with('/') || p.ends_with('\\') => self.fs(FsOp::NewFolder(self.typed_path(&p))),
            Action::NewFile(p) => self.fs(FsOp::NewFile(self.typed_path(&p))),
            Action::NewFolder(p) => self.fs(FsOp::NewFolder(self.typed_path(&p))),
            Action::MovePath(p) => {
                let Some(from) = self.file_target() else { return self.ed.error("E32: No file name") };
                let mut to = self.typed_path(&p);
                // A move onto a folder puts the file inside it, as `mv` does.
                let into = p.ends_with('/') || self.rel_slash(&to).is_some_and(|r| r.is_empty() || self.known_folder(&r));
                if into && let Some(name) = from.file_name() {
                    to = to.join(name);
                }
                if to != from {
                    self.fs(FsOp::Move { from, to });
                }
            }
            Action::DeletePath { force } => {
                let Some(path) = self.file_target() else { return self.ed.error("E32: No file name") };
                self.delete(path, force);
            }
            _ => {}
        }
    }

    /// Open files at `path` or inside it with unsaved changes, by buffer index.
    fn dirty_under(&self, path: &Path) -> Vec<usize> {
        (0..self.ed.docs.len())
            .filter(|&i| self.ed.docs[i].buf.is_dirty() && self.ed.docs[i].buf.path.as_ref().is_some_and(|p| p.starts_with(path)))
            .collect()
    }

    fn delete(&mut self, path: PathBuf, force: bool) {
        if path == self.ed.root || !path.starts_with(&self.ed.root) {
            return self.ed.error(format!("only files inside the project can be deleted: {}", path.display()));
        }
        if !force && !self.dirty_under(&path).is_empty() {
            return self.ed.error("E37: No write since last change (add ! to override)");
        }
        self.fs(FsOp::Delete(path));
    }

    fn fs(&mut self, op: FsOp) {
        match &self.svc {
            Some(svc) => svc.fs(op),
            None => self.ed.error("the file worker is not running"),
        }
    }

    /// The file worker made, moved or deleted a file or folder.
    pub fn fs_done(&mut self, op: FsOp, result: Result<(), String>) {
        if let Err(e) = result {
            return self.ed.error(e);
        }
        match op {
            FsOp::NewFile(p) => {
                self.tree.focus = false;
                self.ed.message(format!("\"{}\" created", self.rel(&p)));
                self.open_path(p, None);
            }
            FsOp::NewFolder(p) => {
                self.show_in_tree(&p);
                self.ed.message(format!("\"{}\" created", self.rel(&p)));
            }
            FsOp::Move { from, to } => {
                for i in 0..self.ed.docs.len() {
                    let Some(old) = self.ed.docs[i].buf.path.clone() else { continue };
                    let Ok(rest) = old.strip_prefix(&from) else { continue };
                    let new = if rest.as_os_str().is_empty() { to.clone() } else { to.join(rest) };
                    if let Some(svc) = &self.svc {
                        svc.lsp(crate::services::lsp::Job::Close { path: old.clone() });
                    }
                    if let Some(m) = self.meta.remove(&old) {
                        self.meta.insert(new.clone(), m);
                    }
                    self.ed.docs[i].buf.path = Some(new);
                }
                if self.tree.cut.as_ref() == Some(&from) {
                    self.tree.cut = None;
                }
                self.show_in_tree(&to);
                self.ed.message(format!("\"{}\" moved to \"{}\"", self.rel(&from), self.rel(&to)));
            }
            FsOp::Delete(p) => {
                for i in (0..self.ed.docs.len()).rev() {
                    if self.ed.docs[i].buf.path.as_ref().is_some_and(|d| d.starts_with(&p)) {
                        if let Some(svc) = &self.svc {
                            svc.lsp(crate::services::lsp::Job::Close { path: self.ed.docs[i].buf.path.clone().unwrap_or_default() });
                            svc.unswap(self.ed.docs[i].buf.path.clone().unwrap_or_default());
                        }
                        self.ed.close(i);
                    }
                }
                self.ed.mode = self.ed.rest_mode();
                self.apply_doc_opts();
                if let Some(parent) = p.parent().and_then(|d| self.rel_slash(d)) {
                    self.tree.sel = (!parent.is_empty()).then_some(parent);
                    self.tree.follow = Follow::Edge;
                }
                self.ed.message(format!("\"{}\" deleted", self.rel(&p)));
            }
        }
        if let Some(svc) = &self.svc {
            svc.files_changed(Vec::new());
            svc.git_refresh();
        }
    }

    /// Opens the folders above `path` and puts the tree's row on it. A folder opens too.
    fn show_in_tree(&mut self, path: &Path) {
        self.reveal_in_tree(path);
        if let Some(rel) = self.rel_slash(path)
            && self.tree.focus
        {
            self.tree.open.insert(rel);
        }
    }
}

/// `path` without `.` and with each `..` taking off the folder before it. The disk is not read.
fn normal(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_takes_out_dots() {
        assert_eq!(normal(Path::new("/p/src/../docs/./a.md")), PathBuf::from("/p/docs/a.md"));
        assert_eq!(normal(Path::new("/p/..")), PathBuf::from("/"));
    }

    #[test]
    fn fs_ops_never_write_over_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("src/a.rs");
        FsOp::NewFile(a.clone()).apply().unwrap();
        assert!(a.is_file(), "the parent folder is made too");
        assert!(FsOp::NewFile(a.clone()).apply().unwrap_err().contains("already exists"));
        let b = dir.path().join("b.rs");
        std::fs::write(&b, "keep").unwrap();
        assert!(FsOp::Move { from: a.clone(), to: b.clone() }.apply().unwrap_err().contains("already exists"));
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "keep");
        let src = dir.path().join("src");
        assert!(FsOp::Move { from: src.clone(), to: src.join("deeper") }.apply().unwrap_err().contains("into itself"));
        let moved = dir.path().join("lib/x/a.rs");
        FsOp::Move { from: a.clone(), to: moved.clone() }.apply().unwrap();
        assert!(moved.is_file() && !a.exists());
        FsOp::NewFolder(dir.path().join("docs/notes")).apply().unwrap();
        assert!(dir.path().join("docs/notes").is_dir());
        FsOp::Delete(dir.path().join("lib")).apply().unwrap();
        assert!(!dir.path().join("lib").exists(), "a folder goes with all in it");
        assert!(FsOp::Delete(dir.path().join("lib")).apply().is_err());
    }
}
