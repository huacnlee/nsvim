//! File operations from the tree and the command line: create, rename or move, delete.
//! Paths are relative to the project root. Open buffers follow a rename; a deleted file's
//! buffer stays open, unsaved, so nothing typed is lost.

use std::path::{Path, PathBuf};

use super::App;
use super::tree::TreeIndex;

impl App {
    /// `path` under the project root. Absolute paths and `..` that leave the root are refused.
    fn project_path(&self, rel: &str) -> Result<PathBuf, String> {
        let rel = rel.trim().trim_start_matches("./");
        let p = Path::new(rel);
        if rel.is_empty() || p.is_absolute() || p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return Err(format!("not a path in the project: {rel}"));
        }
        Ok(self.ed.root.join(rel.trim_end_matches('/')))
    }

    /// Creates a file (opened at once) or, with a trailing `/`, a folder.
    pub fn create_path(&mut self, rel: String) {
        let path = match self.project_path(&rel) {
            Ok(p) => p,
            Err(e) => return self.ed.error(e),
        };
        if path.exists() {
            return self.ed.error(format!("already exists: {}", rel.trim()));
        }
        let dir = rel.trim().ends_with('/');
        let made = if dir {
            std::fs::create_dir_all(&path)
        } else {
            path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&path, ""))
        };
        if let Err(e) = made {
            return self.ed.error(format!("cannot create {}: {e}", rel.trim()));
        }
        self.files_moved(vec![path.clone()]);
        if dir {
            self.reveal_in_tree(&path.join("x"));
            self.tree.sel = Some(self.rel_slash(&path));
            self.ed.message(format!("created {}", rel.trim()));
        } else {
            self.open_path(path, None);
        }
    }

    /// Renames or moves `from` to `to`. A `to` ending in `/` or naming a folder keeps the name.
    pub fn move_path(&mut self, from: String, to: String) {
        let (a, mut b) = match (self.project_path(&from), self.project_path(&to)) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(e), _) | (_, Err(e)) => return self.ed.error(e),
        };
        if !a.exists() {
            return self.ed.error(format!("no such file: {from}"));
        }
        if (to.ends_with('/') || b.is_dir())
            && let Some(name) = a.file_name()
        {
            b = b.join(name);
        }
        if b.exists() {
            return self.ed.error(format!("already exists: {}", self.rel_slash(&b)));
        }
        if b.starts_with(&a) {
            return self.ed.error("cannot move a folder into itself");
        }
        let moved = b.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::rename(&a, &b));
        if let Err(e) = moved {
            return self.ed.error(format!("cannot move {from}: {e}"));
        }
        for d in &mut self.ed.docs {
            if let Some(p) = d.buf.path.clone()
                && let Ok(rest) = p.strip_prefix(&a)
            {
                d.buf.path = Some(if rest.as_os_str().is_empty() { b.clone() } else { b.join(rest) });
            }
        }
        self.files_moved(vec![a, b.clone()]);
        self.reveal_in_tree(&b);
        self.ed.message(format!("moved {from} to {}", self.rel_slash(&b)));
    }

    /// Deletes a file or a folder with everything in it.
    pub fn delete_path(&mut self, rel: String) {
        let path = match self.project_path(&rel) {
            Ok(p) => p,
            Err(e) => return self.ed.error(e),
        };
        let gone = if path.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };
        if let Err(e) = gone {
            return self.ed.error(format!("cannot delete {}: {e}", rel.trim()));
        }
        if self.tree.sel.as_deref().is_some_and(|s| Path::new(s).starts_with(rel.trim().trim_end_matches('/'))) {
            self.tree.sel = rel.trim().trim_end_matches('/').rsplit_once('/').map(|(p, _)| p.to_string());
        }
        self.files_moved(vec![path]);
        self.ed.message(format!("deleted {}", rel.trim()));
    }

    /// The path relative to the root, with `/` on every platform.
    fn rel_slash(&self, p: &Path) -> String {
        p.strip_prefix(&self.ed.root).unwrap_or(p).to_string_lossy().replace('\\', "/")
    }

    /// Updates the file list and the tree now, then lets the index worker rescan. Only the
    /// changed paths are read from disk; the rest of the list (which honours .gitignore) stays.
    fn files_moved(&mut self, paths: Vec<PathBuf>) {
        let root = self.ed.root.clone();
        let mut files: Vec<String> = self.files.iter().filter(|f| root.join(f).is_file()).cloned().collect();
        let mut empty_dirs = Vec::new();
        for p in &paths {
            if p.is_file() {
                files.push(self.rel_slash(p));
            } else if p.is_dir() {
                let before = files.len();
                collect(&root, p, &mut files);
                if files.len() == before {
                    empty_dirs.push(self.rel_slash(p));
                }
            }
        }
        files.sort();
        files.dedup();
        let mut index = TreeIndex::build(&files);
        for d in &empty_dirs {
            index.add_dir(d);
        }
        self.tree.index = index;
        self.files = files;
        if let Some(svc) = &self.svc {
            svc.files_changed(paths);
        }
    }
}

/// Every file under `dir`, relative to `root`, skipping `.git`.
fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.file_name().is_some_and(|n| n == ".git") {
            continue;
        }
        if p.is_dir() {
            collect(root, &p, out);
        } else if let Ok(r) = p.strip_prefix(root) {
            out.push(r.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::fx::Tx;
    use super::*;
    use editor::editor::{KeyCode, KeyInput};

    fn app(root: &Path) -> App {
        let cfg = toml::from_str("[ipc]\nenabled = false\n").unwrap();
        let mut a = App::new(cfg, root.to_path_buf(), false, None);
        a.files_moved(vec![root.join("src")]);
        a
    }

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        dir
    }

    /// Types `line` on the command line and runs it, as Enter does.
    fn run(a: &mut App, tx: &mut Tx) {
        a.key(KeyInput::plain(KeyCode::Enter), tx);
    }

    #[test]
    fn create_makes_files_and_folders_and_shows_them() {
        let dir = project();
        let mut a = app(dir.path());
        a.create_path("src/ui/".into());
        assert!(dir.path().join("src/ui").is_dir());
        a.tree.open.insert("src".into());
        assert!(a.tree.index.rows(&a.tree.open).iter().any(|r| r.path == "src/ui" && r.dir));
        a.create_path("src/ui/view.rs".into());
        assert!(dir.path().join("src/ui/view.rs").is_file());
        assert!(a.files.contains(&"src/ui/view.rs".to_string()));
        a.create_path("src/main.rs".into());
        assert!(a.ed.msg.as_ref().is_some_and(|m| m.text.contains("already exists")));
    }

    #[test]
    fn move_renames_files_and_folders_and_keeps_open_buffers() {
        let dir = project();
        let mut a = app(dir.path());
        // Opening goes through the IO worker; put the buffer in directly.
        a.ed.docs.push(editor::editor::Doc::new(editor::buffer::Buffer::open(&dir.path().join("src/main.rs")).unwrap()));
        a.ed.active = 0;
        a.move_path("src/main.rs".into(), "src/lib.rs".into());
        assert!(dir.path().join("src/lib.rs").is_file());
        assert!(!dir.path().join("src/main.rs").exists());
        assert_eq!(a.ed.buf().path.as_deref(), Some(dir.path().join("src/lib.rs").as_path()));
        // Into a folder keeps the name; a folder moves with what is in it.
        a.create_path("lib/".into());
        a.move_path("src".into(), "lib/".into());
        assert!(dir.path().join("lib/src/lib.rs").is_file());
        assert_eq!(a.ed.buf().path.as_deref(), Some(dir.path().join("lib/src/lib.rs").as_path()));
        assert!(a.files.contains(&"lib/src/lib.rs".to_string()));
        a.move_path("lib".into(), "lib/src/x".into());
        assert!(dir.path().join("lib").is_dir());
    }

    #[test]
    fn delete_removes_files_and_folders_but_not_outside_the_project() {
        let dir = project();
        let mut a = app(dir.path());
        a.delete_path("src/main.rs".into());
        assert!(!dir.path().join("src/main.rs").exists());
        assert!(!a.files.contains(&"src/main.rs".to_string()));
        a.delete_path("src".into());
        assert!(!dir.path().join("src").exists());
        a.delete_path("../x".into());
        assert!(a.ed.msg.as_ref().is_some_and(|m| m.text.contains("not a path in the project")));
    }

    #[test]
    fn tree_keys_fill_in_the_command_and_enter_runs_it() {
        let dir = project();
        let mut a = app(dir.path());
        let mut tx = Tx::new();
        a.focus_tree();
        a.tree.open.insert("src".into());
        a.tree.sel = Some("src/main.rs".into());
        a.key(KeyInput::plain(KeyCode::Char('a')), &mut tx);
        assert_eq!(a.ed.cmd.as_ref().map(|c| c.text.as_str()), Some("Create src/"));
        for c in "new.rs".chars() {
            a.key(KeyInput::plain(KeyCode::Char(c)), &mut tx);
        }
        run(&mut a, &mut tx);
        assert!(dir.path().join("src/new.rs").is_file());

        a.tree.focus = true;
        a.tree.sel = Some("src/main.rs".into());
        a.key(KeyInput::plain(KeyCode::Char('r')), &mut tx);
        assert_eq!(a.ed.cmd.as_ref().map(|c| c.text.as_str()), Some("Move src/main.rs src/main.rs"));
        for _ in 0.."main.rs".len() {
            a.key(KeyInput::plain(KeyCode::Backspace), &mut tx);
        }
        for c in "app.rs".chars() {
            a.key(KeyInput::plain(KeyCode::Char(c)), &mut tx);
        }
        run(&mut a, &mut tx);
        assert!(dir.path().join("src/app.rs").is_file());

        a.tree.focus = true;
        a.tree.sel = Some("src/app.rs".into());
        a.key(KeyInput::plain(KeyCode::Char('d')), &mut tx);
        assert_eq!(a.ed.cmd.as_ref().map(|c| c.text.as_str()), Some("Delete src/app.rs"));
        a.key(KeyInput::plain(KeyCode::Esc), &mut tx);
        assert!(dir.path().join("src/app.rs").is_file(), "Esc cancels");
    }
}
