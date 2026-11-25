//! Git, read with the `git` command: the branch, each file's status for the tree, and the
//! gutter signs of a buffer against `HEAD`. No full git UI: that is out of scope.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{Sender, channel};

use super::{Event, Out};

pub enum Job {
    Info,
    Signs { path: PathBuf, text: String, version: u64 },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GitInfo {
    pub branch: Option<String>,
    /// Status letter by path relative to the root: `M` modified, `U` untracked, `A` added,
    /// `D` deleted.
    pub status: HashMap<String, char>,
    /// Every folder with a changed path inside, for the tree's mark on folders.
    pub dirs: HashSet<String>,
}

/// A gutter sign on one line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sign {
    pub line: usize,
    pub kind: SignKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignKind {
    Added,
    Modified,
    /// Lines were removed after this one.
    Deleted,
}

pub fn start(root: PathBuf, out: Out) -> Sender<Job> {
    let (tx, rx) = channel::<Job>();
    std::thread::Builder::new()
        .name("editor-git".into())
        .spawn(move || {
            if !is_repo(&root) {
                for _ in rx {}
                return;
            }
            out.send(Event::Git(info(&root)));
            while let Ok(job) = rx.recv() {
                // Keep only the newest request of each kind in a burst.
                let mut jobs = vec![job];
                jobs.extend(rx.try_iter());
                let mut info_asked = false;
                let mut signs: HashMap<PathBuf, (String, u64)> = HashMap::new();
                for j in jobs {
                    match j {
                        Job::Info => info_asked = true,
                        Job::Signs { path, text, version } => {
                            signs.insert(path, (text, version));
                        }
                    }
                }
                if info_asked {
                    out.send(Event::Git(info(&root)));
                }
                for (path, (text, version)) in signs {
                    if let Some(signs) = signs_for(&root, &path, &text) {
                        out.send(Event::Signs { path, version, signs });
                    }
                }
            }
        })
        .expect("start the git worker");
    tx
}

fn git(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    // `--no-optional-locks`: `git status` would otherwise take the index lock to refresh it, and
    // a commit the user runs at that moment fails with "index.lock: File exists".
    let out = Command::new("git").arg("--no-optional-locks").arg("-C").arg(root).args(args).output().ok()?;
    out.status.success().then_some(out.stdout)
}

fn is_repo(root: &Path) -> bool {
    git(root, &["rev-parse", "--is-inside-work-tree"]).is_some()
}

pub fn info(root: &Path) -> GitInfo {
    let branch =
        git(root, &["rev-parse", "--abbrev-ref", "HEAD"]).map(|b| String::from_utf8_lossy(&b).trim().to_string()).filter(|b| !b.is_empty());
    let mut status = HashMap::new();
    // Paths in porcelain output are relative to the repository's top, which may sit above root.
    let top = git(root, &["rev-parse", "--show-toplevel"]).map(|t| PathBuf::from(String::from_utf8_lossy(&t).trim()));
    let prefix = top.as_deref().and_then(|t| root.strip_prefix(t).ok()).map(|p| p.to_string_lossy().replace('\\', "/"));
    if let Some(raw) = git(root, &["status", "--porcelain=v1", "-z", "--untracked-files=all"]) {
        let mut parts = raw.split(|b| *b == 0);
        while let Some(rec) = parts.next() {
            if rec.len() < 4 {
                continue;
            }
            let (x, y) = (rec[0] as char, rec[1] as char);
            let path = String::from_utf8_lossy(&rec[3..]).into_owned();
            if x == 'R' || x == 'C' {
                parts.next();
            }
            let letter = match (x, y) {
                ('?', '?') => 'U',
                ('A', _) => 'A',
                (_, 'D') | ('D', _) => 'D',
                _ => 'M',
            };
            let rel = match &prefix {
                Some(p) if !p.is_empty() => match path.strip_prefix(&format!("{p}/")) {
                    Some(r) => r.to_string(),
                    None => continue,
                },
                _ => path,
            };
            status.insert(rel, letter);
        }
    }
    let dirs = changed_dirs(&status);
    GitInfo { branch, status, dirs }
}

/// The folders that hold the paths in `status`, at any depth.
fn changed_dirs(status: &HashMap<String, char>) -> HashSet<String> {
    let mut dirs = HashSet::new();
    for path in status.keys() {
        let mut p = path.as_str();
        // A folder already in the set has its parents in it too.
        while let Some((parent, _)) = p.rsplit_once('/') {
            if !dirs.insert(parent.to_string()) {
                break;
            }
            p = parent;
        }
    }
    dirs
}

/// Signs for `text` (the buffer now) against the file at `HEAD`. `None` when git doesn't track
/// the file.
pub fn signs_for(root: &Path, path: &Path, text: &str) -> Option<Vec<Sign>> {
    let dir = path.parent()?;
    let name = path.file_name()?.to_string_lossy().into_owned();
    let base = git(dir, &["show", &format!("HEAD:./{name}")])?;
    let _ = root;
    let base = String::from_utf8_lossy(&base).replace("\r\n", "\n");
    Some(diff_signs(&base, text))
}

pub fn diff_signs(base: &str, text: &str) -> Vec<Sign> {
    use similar::{DiffOp, TextDiff};
    let diff = TextDiff::configure().timeout(std::time::Duration::from_millis(200)).diff_lines(base, text);
    let mut out = Vec::new();
    for op in diff.ops() {
        match *op {
            DiffOp::Insert { new_index, new_len, .. } => {
                out.extend((new_index..new_index + new_len).map(|line| Sign { line, kind: SignKind::Added }));
            }
            DiffOp::Replace { new_index, new_len, .. } => {
                out.extend((new_index..new_index + new_len).map(|line| Sign { line, kind: SignKind::Modified }));
            }
            DiffOp::Delete { new_index, .. } => {
                out.push(Sign { line: new_index.saturating_sub(1), kind: SignKind::Deleted });
            }
            DiffOp::Equal { .. } => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_with_changes_include_every_parent() {
        let status: HashMap<String, char> =
            [("src/app/mod.rs", 'M'), ("src/ui/text.rs", 'U'), ("README.md", 'M')].iter().map(|(p, l)| (p.to_string(), *l)).collect();
        let mut dirs: Vec<String> = changed_dirs(&status).into_iter().collect();
        dirs.sort();
        assert_eq!(dirs, vec!["src", "src/app", "src/ui"]);
    }

    #[test]
    fn signs_mark_added_changed_and_deleted_lines() {
        let base = "a\nb\nc\nd\n";
        let text = "a\nB\nc\nnew\n";
        let signs = diff_signs(base, text);
        assert!(signs.contains(&Sign { line: 1, kind: SignKind::Modified }));
        assert!(signs.iter().any(|s| s.line == 3));
    }
}
