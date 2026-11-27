//! Git, read with the `git` command: the branch, each file's status for the tree, the
//! gutter signs of a buffer against `HEAD`, and who last changed each line (`git blame`). No
//! full git UI: that is out of scope.

use std::collections::{HashMap, HashSet};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process::{Command, Stdio};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Sender, channel};

#[cfg(not(target_arch = "wasm32"))]
use super::{Event, Out};

pub enum Job {
    Info,
    Signs { path: PathBuf, text: String, version: u64 },
    Blame { path: PathBuf, text: String, version: u64 },
}

/// Who last changed one line, from `git blame`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlameLine {
    /// The author and when, as the line shows it: `Ada Lovelace, 3 days ago`.
    pub who: String,
    /// The first line of the commit's message. Empty for a line that is not committed yet.
    pub summary: String,
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

#[cfg(not(target_arch = "wasm32"))]
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
                let mut blames: HashMap<PathBuf, (String, u64)> = HashMap::new();
                for j in jobs {
                    match j {
                        Job::Info => info_asked = true,
                        Job::Signs { path, text, version } => {
                            signs.insert(path, (text, version));
                        }
                        Job::Blame { path, text, version } => {
                            blames.insert(path, (text, version));
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
                for (path, (text, version)) in blames {
                    let lines = blame_for(&path, &text).unwrap_or_default();
                    out.send(Event::Blame { path, version, lines });
                }
            }
        })
        .expect("start the git worker");
    tx
}

#[cfg(not(target_arch = "wasm32"))]
fn git(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    // `--no-optional-locks`: `git status` would otherwise take the index lock to refresh it, and
    // a commit the user runs at that moment fails with "index.lock: File exists".
    let out = Command::new("git").arg("--no-optional-locks").arg("-C").arg(root).args(args).output().ok()?;
    out.status.success().then_some(out.stdout)
}

#[cfg(not(target_arch = "wasm32"))]
fn is_repo(root: &Path) -> bool {
    git(root, &["rev-parse", "--is-inside-work-tree"]).is_some()
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
/// Who last changed each line of `text` (the buffer now, saved or not). `None` when git doesn't
/// track the file.
pub fn blame_for(path: &Path, text: &str) -> Option<Vec<BlameLine>> {
    use std::io::Write;
    let dir = path.parent()?;
    let name = path.file_name()?;
    let mut child = Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(dir)
        .args(["blame", "--porcelain", "--contents", "-", "--"])
        .arg(name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    // Write on another thread: git can fill its output pipe before it reads all of the input.
    let mut stdin = child.stdin.take()?;
    let input = text.as_bytes().to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    let out = child.wait_with_output().ok()?;
    let _ = writer.join();
    if !out.status.success() {
        return None;
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    Some(parse_blame(&String::from_utf8_lossy(&out.stdout), now))
}

/// The lines of `git blame --porcelain` output, in order. `now` is the time in seconds since
/// 1970, for the age of each change.
pub fn parse_blame(raw: &str, now: i64) -> Vec<BlameLine> {
    #[derive(Default)]
    struct Commit {
        author: String,
        time: i64,
        summary: String,
    }
    let mut commits: HashMap<String, Commit> = HashMap::new();
    // The final line number (from 1) and the commit of each line.
    let mut lines: Vec<(usize, String)> = Vec::new();
    // The commit and line of the header that is open, until its tab line.
    let mut open: Option<(String, usize)> = None;
    for l in raw.lines() {
        if l.starts_with('\t') {
            lines.extend(open.take().map(|(sha, n)| (n, sha)));
            continue;
        }
        if let Some((sha, _)) = &open {
            let c = commits.entry(sha.clone()).or_default();
            if let Some(v) = l.strip_prefix("author ") {
                c.author = v.to_string();
            } else if let Some(v) = l.strip_prefix("author-time ") {
                c.time = v.trim().parse().unwrap_or(0);
            } else if let Some(v) = l.strip_prefix("summary ") {
                c.summary = v.to_string();
            }
            continue;
        }
        let mut parts = l.split(' ');
        if let (Some(sha), Some(_), Some(n)) = (parts.next(), parts.next(), parts.next())
            && sha.len() >= 40
            && let Ok(n) = n.parse::<usize>()
        {
            commits.entry(sha.to_string()).or_default();
            open = Some((sha.to_string(), n));
        }
    }
    let mut out = vec![BlameLine::default(); lines.iter().map(|(n, _)| *n).max().unwrap_or(0)];
    for (n, sha) in lines {
        let Some(c) = commits.get(&sha) else { continue };
        out[n - 1] = if sha.bytes().all(|b| b == b'0') {
            BlameLine { who: "Not committed yet".into(), summary: String::new() }
        } else {
            BlameLine { who: format!("{}, {}", c.author, ago(now - c.time)), summary: c.summary.clone() }
        };
    }
    out
}

/// How long ago, for `secs` seconds: `just now`, `5 minutes ago`, `1 year ago`.
pub fn ago(secs: i64) -> String {
    let units = [(365 * 86_400, "year"), (30 * 86_400, "month"), (86_400, "day"), (3_600, "hour"), (60, "minute")];
    for (size, name) in units {
        let n = secs / size;
        if n >= 1 {
            return format!("{n} {name}{} ago", if n == 1 { "" } else { "s" });
        }
    }
    "just now".into()
}

pub fn diff_signs(base: &str, text: &str) -> Vec<Sign> {
    use similar::DiffOp;
    let diff = crate::app::review::configure(200).diff_lines(base, text);
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

    #[test]
    fn blame_reads_the_author_age_and_summary_of_each_line() {
        let sha = "1".repeat(40);
        let zero = "0".repeat(40);
        let raw = format!(
            "{sha} 1 1 2\nauthor Ada Lovelace\nauthor-mail <ada@example.com>\nauthor-time 1000\nauthor-tz +0000\nsummary Add the notes\nfilename a.txt\n\tone\n\
             {sha} 2 2\n\ttwo\n\
             {zero} 3 3 1\nauthor Not Committed Yet\nauthor-time 4000\nsummary Version of a.txt from a.txt\nfilename a.txt\n\tthree\n"
        );
        let lines = parse_blame(&raw, 1000 + 3 * 86_400);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], BlameLine { who: "Ada Lovelace, 3 days ago".into(), summary: "Add the notes".into() });
        assert_eq!(lines[1], lines[0]);
        assert_eq!(lines[2], BlameLine { who: "Not committed yet".into(), summary: String::new() });
    }

    #[test]
    fn ages_read_in_the_largest_whole_unit() {
        assert_eq!(ago(5), "just now");
        assert_eq!(ago(60), "1 minute ago");
        assert_eq!(ago(2 * 3_600 + 10), "2 hours ago");
        assert_eq!(ago(400 * 86_400), "1 year ago");
    }

    #[test]
    fn blame_runs_git_on_the_text_of_the_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let run = |args: &[&str]| {
            let ok = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.com", "-c", "commit.gpgsign=false"])
                .args(args)
                .output()
                .unwrap()
                .status
                .success();
            assert!(ok, "git {args:?}");
        };
        run(&["init", "-q"]);
        std::fs::write(root.join("a.txt"), "one\ntwo\n").unwrap();
        run(&["add", "a.txt"]);
        run(&["commit", "-q", "-m", "Add the notes"]);
        let lines = blame_for(&root.join("a.txt"), "one\nTWO\n").expect("git tracks a.txt");
        assert!(lines[0].who.starts_with("Ada Lovelace, "), "{:?}", lines[0]);
        assert_eq!(lines[0].summary, "Add the notes");
        assert_eq!(lines[1].who, "Not committed yet");
        assert_eq!(blame_for(&root.join("new.txt"), "x\n"), None);
    }
}
