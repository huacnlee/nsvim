//! The project index: every file under the root that git doesn't ignore, for the finder and
//! the file tree. Walked again when a changed path shows a file came or went.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use ignore::WalkState;

use super::{Event, Out};
use crate::app::tree::TreeIndex;

/// The index stops here, so a home folder opened by mistake stays quick.
const MAX_FILES: usize = 200_000;

pub fn start(root: PathBuf, out: Out) -> Sender<Vec<PathBuf>> {
    let (tx, rx) = channel::<Vec<PathBuf>>();
    std::thread::Builder::new()
        .name("editor-index".into())
        .spawn(move || {
            let (mut files, mut dirs) = walk_all(&root);
            out.send(Event::Files(files.clone(), TreeIndex::build_with(&files, &dirs)));
            while let Ok(first) = rx.recv() {
                // Coalesce a burst (a git checkout touches many files).
                std::thread::sleep(Duration::from_millis(150));
                let mut force = first.is_empty();
                let mut paths = first;
                for more in rx.try_iter() {
                    force |= more.is_empty();
                    paths.extend(more);
                }
                let stale = force
                    || paths.iter().any(|p| {
                        let Ok(rel) = p.strip_prefix(&root) else { return false };
                        let rel = rel.to_string_lossy().replace('\\', "/");
                        files.binary_search(&rel).is_ok() != p.is_file() || dirs.binary_search(&rel).is_ok() != p.is_dir()
                    });
                if stale {
                    // A folder past the cap stays past it: skip the walk on several threads.
                    (files, dirs) = if files.len() >= MAX_FILES { walk_ordered(&root, MAX_FILES) } else { walk_all(&root) };
                    out.send(Event::Files(files.clone(), TreeIndex::build_with(&files, &dirs)));
                }
            }
        })
        .expect("start the index worker");
    tx
}

/// Every file under `root` that git doesn't ignore, relative to it and sorted. The walk runs
/// on several threads, so a large project indexes in a fraction of the time.
pub fn walk(root: &Path) -> Vec<String> {
    walk_all(root).0
}

/// Every file and every folder under `root` that git doesn't ignore, relative to it and
/// sorted. The tree shows the folders too, so an empty folder has a row.
pub fn walk_all(root: &Path) -> (Vec<String>, Vec<String>) {
    walk_capped(root, MAX_FILES)
}

/// `walk_all`, stopped at `max` files. Threads stop at `max` in whatever order they got
/// there, so each walk of a bigger folder would keep a different part of it, and the tree
/// would lose and regain folders on every change (#43). Past `max` the walk runs again on
/// one thread, in name order, and keeps the same files each time.
fn walk_capped(root: &Path, max: usize) -> (Vec<String>, Vec<String>) {
    match walk_parallel(root, max) {
        Some(all) => all,
        None => walk_ordered(root, max),
    }
}

fn builder(root: &Path) -> ignore::WalkBuilder {
    let mut b = ignore::WalkBuilder::new(root);
    b.hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .parents(true)
        .require_git(false)
        .filter_entry(|e| e.file_name() != ".git");
    b
}

/// The walk on several threads, or None when it reached `max` files.
fn walk_parallel(root: &Path, max: usize) -> Option<(Vec<String>, Vec<String>)> {
    let all = Mutex::new(Vec::new());
    let all_dirs = Mutex::new(Vec::new());
    let count = AtomicUsize::new(0);
    builder(root).build_parallel().run(|| {
        let mut batch = Batch { files: Vec::new(), all: &all };
        let mut dirs = Batch { files: Vec::new(), all: &all_dirs };
        let count = &count;
        Box::new(move |entry| {
            let Ok(entry) = entry else { return WalkState::Continue };
            if entry.depth() > 0
                && entry.file_type().is_some_and(|t| t.is_dir())
                && let Ok(rel) = entry.path().strip_prefix(root)
            {
                dirs.files.push(rel.to_string_lossy().replace('\\', "/"));
                return WalkState::Continue;
            }
            if entry.file_type().is_some_and(|t| t.is_file())
                && let Ok(rel) = entry.path().strip_prefix(root)
            {
                batch.files.push(rel.to_string_lossy().replace('\\', "/"));
                if count.fetch_add(1, Ordering::Relaxed) + 1 >= max {
                    return WalkState::Quit;
                }
            }
            WalkState::Continue
        })
    });
    if count.load(Ordering::Relaxed) >= max {
        return None;
    }
    let mut files = all.into_inner().unwrap_or_else(|e| e.into_inner());
    files.sort_unstable();
    let mut dirs = all_dirs.into_inner().unwrap_or_else(|e| e.into_inner());
    dirs.sort_unstable();
    Some((files, dirs))
}

/// The walk on one thread, in name order, stopped at `max` files.
fn walk_ordered(root: &Path, max: usize) -> (Vec<String>, Vec<String>) {
    let (mut files, mut dirs) = (Vec::new(), Vec::new());
    for entry in builder(root).sort_by_file_name(|a, b| a.cmp(b)).build().flatten() {
        let Ok(rel) = entry.path().strip_prefix(root) else { continue };
        let rel = rel.to_string_lossy().replace('\\', "/");
        match entry.file_type() {
            Some(t) if t.is_dir() && entry.depth() > 0 => dirs.push(rel),
            Some(t) if t.is_file() => {
                files.push(rel);
                if files.len() >= max {
                    break;
                }
            }
            _ => {}
        }
    }
    files.sort_unstable();
    dirs.sort_unstable();
    (files, dirs)
}

/// One walker thread's files. They join the shared list once, when the thread ends.
struct Batch<'a> {
    files: Vec<String>,
    all: &'a Mutex<Vec<String>>,
}

impl Drop for Batch<'_> {
    fn drop(&mut self) {
        let mut all = self.all.lock().unwrap_or_else(|e| e.into_inner());
        all.append(&mut self.files);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 30 folders of 40 files.
    fn big_folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for d in 0..30 {
            let sub = dir.path().join(format!("d{d:02}"));
            std::fs::create_dir(&sub).unwrap();
            for f in 0..40 {
                std::fs::write(sub.join(format!("f{f:02}")), "").unwrap();
            }
        }
        dir
    }

    #[test]
    fn a_folder_past_the_cap_keeps_the_same_files_on_each_walk() {
        let dir = big_folder();
        let first = walk_capped(dir.path(), 500);
        assert_eq!(first.0.len(), 500);
        // Name order: the first twelve folders and half of the thirteenth.
        assert_eq!(first.0[0], "d00/f00");
        assert_eq!(first.0[499], "d12/f19");
        assert_eq!(first.1.len(), 13);
        for _ in 0..20 {
            assert_eq!(walk_capped(dir.path(), 500), first);
        }
    }

    #[test]
    fn a_folder_under_the_cap_has_every_file_and_folder() {
        let dir = big_folder();
        let (files, dirs) = walk_capped(dir.path(), 10_000);
        assert_eq!(files.len(), 1200);
        assert_eq!(dirs.len(), 30);
        assert_eq!(walk_ordered(dir.path(), 10_000), (files, dirs));
    }
}
