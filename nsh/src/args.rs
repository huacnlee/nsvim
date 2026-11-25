//! Command-line arguments: `editor [path[:line]] [+line] [--wait]`. A folder opens as the
//! project. A file opens in the project around it: its git work tree, else its folder.

use std::path::{Path, PathBuf};

use editor::buffer::Pos;

/// The project root and the files to open, with a cursor position when one was given.
pub fn targets() -> (PathBuf, Vec<(PathBuf, Option<Pos>)>) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut paths: Vec<(PathBuf, Option<Pos>)> = Vec::new();
    let mut line: Option<usize> = None;
    let args: Vec<String> = std::env::args().skip(1).collect();
    for a in &args {
        if let Some(n) = a.strip_prefix('+') {
            line = n.parse::<usize>().ok().map(|n| n.saturating_sub(1));
            continue;
        }
        if a.starts_with("--") {
            continue;
        }
        let (p, at) = split_line(a.as_str());
        let p = if Path::new(&p).is_absolute() { PathBuf::from(p) } else { cwd.join(p) };
        paths.push((clean(&p), at.map(|l| Pos::new(l, 0))));
    }
    if let (Some(l), Some(first)) = (line, paths.first_mut()) {
        first.1 = Some(Pos::new(l, 0));
    }
    let Some((first, _)) = paths.first().cloned() else { return (cwd.clone(), Vec::new()) };
    if first.is_dir() {
        let rest = paths.into_iter().skip(1).filter(|(p, _)| !p.is_dir()).collect();
        return (first, rest);
    }
    let dir = first.parent().map(Path::to_path_buf).unwrap_or(cwd);
    let root = git_top(&dir).or_else(|| project_root(&dir)).unwrap_or(dir);
    (root, paths.into_iter().filter(|(p, _)| !p.is_dir()).collect())
}

/// `src/main.rs:42` → (`src/main.rs`, line 41). A path that exists as written keeps its colon.
fn split_line(a: &str) -> (String, Option<usize>) {
    if let Some((p, n)) = a.rsplit_once(':')
        && let Ok(n) = n.parse::<usize>()
        && !Path::new(a).exists()
    {
        return (p.to_string(), Some(n.saturating_sub(1)));
    }
    (a.to_string(), None)
}

/// Files that mark a project's top folder when there is no git repository.
const MARKERS: &[&str] =
    &["Cargo.toml", "package.json", "go.mod", "pyproject.toml", "setup.py", "CMakeLists.txt", "Makefile", ".editorconfig"];

/// The nearest folder at or above `dir` with a project marker, below the home folder.
fn project_root(dir: &Path) -> Option<PathBuf> {
    let home = dirs::home_dir();
    dir.ancestors().take_while(|d| Some(*d) != home.as_deref()).find(|d| MARKERS.iter().any(|m| d.join(m).exists())).map(Path::to_path_buf)
}

fn git_top(dir: &Path) -> Option<PathBuf> {
    let out = std::process::Command::new("git").arg("-C").arg(dir).args(["rev-parse", "--show-toplevel"]).output().ok()?;
    out.status.success().then(|| PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

/// `a/./b/../c` → `a/c`, without touching the disk.
fn clean(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trailing_line_number_splits_off() {
        assert_eq!(split_line("src/main.rs:42"), ("src/main.rs".into(), Some(41)));
        assert_eq!(split_line("src/main.rs"), ("src/main.rs".into(), None));
    }

    #[test]
    fn a_marker_names_the_project_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        std::fs::create_dir_all(dir.path().join("src/ui")).unwrap();
        assert_eq!(project_root(&dir.path().join("src/ui")).as_deref(), Some(dir.path()));
    }

    #[test]
    fn dots_clean_away() {
        assert_eq!(clean(Path::new("/a/./b/../c")), PathBuf::from("/a/c"));
    }
}
