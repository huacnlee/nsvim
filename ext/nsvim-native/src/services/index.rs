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
                    (files, dirs) = walk_all(&root);
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
    let all = Mutex::new(Vec::new());
    let all_dirs = Mutex::new(Vec::new());
    let count = AtomicUsize::new(0);
    ignore::WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .parents(true)
        .require_git(false)
        .filter_entry(|e| e.file_name() != ".git")
        .build_parallel()
        .run(|| {
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
                    if count.fetch_add(1, Ordering::Relaxed) + 1 >= MAX_FILES {
                        return WalkState::Quit;
                    }
                }
                WalkState::Continue
            })
        });
    let mut files = all.into_inner().unwrap_or_else(|e| e.into_inner());
    files.sort_unstable();
    files.truncate(MAX_FILES);
    let mut dirs = all_dirs.into_inner().unwrap_or_else(|e| e.into_inner());
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
