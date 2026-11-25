//! Command-line arguments: `nsvim [path[:line]] [+line] [--wait] [--new]`, `nsvim ctl` and
//! `nsvim mcp`. A folder opens as the project. A file opens in the project around it: its git
//! work tree, else its folder.

use std::path::{Path, PathBuf};

use editor::buffer::Pos;

pub const USAGE: &str = "nsvim: a modal code editor. Read fast. Edit quick.

Usage:
  nsvim [PATH[:LINE]]... [+LINE] [--wait] [--new]
  nsvim ctl COMMAND [ARGS]    drive the running editor (nsvim ctl --help)
  nsvim mcp                   serve the running editor to AI agents, over stdio

Examples:
  nsvim .                     open this folder as the project
  nsvim src/main.rs:42        open a file at line 42, in the project around it
  nsvim --wait NOTES.md       return when its buffer closes, for $EDITOR and git
  nsvim --new                 open a new window, also when one runs for the project

A path opens in the editor already running for its project, when there is one.

Options:
  --wait          return when the last file's buffer closes
  --new           open a new window
  -h, --help      print this help
  -V, --version   print the version

Docs: https://github.com/huacnlee/nsvim/blob/main/docs/cli.md
";

pub const MCP_USAGE: &str = "nsvim mcp: a Model Context Protocol server, over stdio, for the editor
running in the current folder. An agent starts it. You do not run it by hand.

  claude mcp add nsvim -- nsvim mcp

Tools and setup for Codex and opencode: https://github.com/huacnlee/nsvim/blob/main/docs/agents.md
";

/// What the command line asks for. `argv` includes the program name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `nsvim ctl ARGS`: drive a running editor.
    Ctl(Vec<String>),
    /// `nsvim mcp`: serve the running editor to an agent over stdio.
    Mcp,
    McpHelp,
    Help,
    Version,
    /// Open the editor. `wait`: return when the last file's buffer closes. `new`: open a
    /// window, also when an editor runs for the project.
    Open {
        wait: bool,
        new: bool,
    },
}

pub fn command(argv: &[String]) -> Command {
    let rest = argv.get(2..).unwrap_or_default();
    match argv.get(1).map(String::as_str) {
        Some("ctl") => Command::Ctl(rest.to_vec()),
        Some("mcp") if rest.iter().any(|a| a == "-h" || a == "--help") => Command::McpHelp,
        Some("mcp") => Command::Mcp,
        Some("-h" | "--help") => Command::Help,
        Some("-V" | "--version") => Command::Version,
        _ => Command::Open { wait: argv.iter().any(|a| a == "--wait"), new: argv.iter().any(|a| a == "--new") },
    }
}

/// The project root and the files to open, with a cursor position when one was given.
pub fn targets() -> (PathBuf, Vec<(PathBuf, Option<Pos>)>) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let args: Vec<String> = std::env::args().skip(1).collect();
    targets_in(&cwd, &args)
}

/// `targets` for `args` (without the program name), with relative paths from `cwd`.
pub fn targets_in(cwd: &Path, args: &[String]) -> (PathBuf, Vec<(PathBuf, Option<Pos>)>) {
    let cwd = cwd.to_path_buf();
    let mut paths: Vec<(PathBuf, Option<Pos>)> = Vec::new();
    let mut line: Option<usize> = None;
    for a in args {
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

    fn argv(args: &[&str]) -> Vec<String> {
        std::iter::once("editor").chain(args.iter().copied()).map(String::from).collect()
    }

    #[test]
    fn subcommands_and_flags_parse() {
        assert_eq!(command(&argv(&["ctl", "state", "--json"])), Command::Ctl(vec!["state".into(), "--json".into()]));
        assert_eq!(command(&argv(&["mcp"])), Command::Mcp);
        assert_eq!(command(&argv(&["mcp", "--help"])), Command::McpHelp);
        assert_eq!(command(&argv(&["-h"])), Command::Help);
        assert_eq!(command(&argv(&["--version"])), Command::Version);
        assert_eq!(command(&argv(&["a.rs", "--wait"])), Command::Open { wait: true, new: false });
        assert_eq!(command(&argv(&["--new"])), Command::Open { wait: false, new: true });
        assert_eq!(command(&argv(&[])), Command::Open { wait: false, new: false });
    }

    #[test]
    fn a_file_opens_at_its_line_in_the_project_around_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        let (root, files) = targets_in(&dir.path().join("src"), &["main.rs:42".to_string(), "--wait".to_string()]);
        assert_eq!(root, dir.path());
        assert_eq!(files, vec![(dir.path().join("src/main.rs"), Some(Pos::new(41, 0)))]);
        let (_, files) = targets_in(dir.path(), &["+7".to_string(), "a.rs".to_string(), "b.rs".to_string()]);
        assert_eq!(files[0], (dir.path().join("a.rs"), Some(Pos::new(6, 0))));
        assert_eq!(files[1], (dir.path().join("b.rs"), None));
    }

    #[test]
    fn a_folder_opens_as_the_project() {
        let dir = tempfile::tempdir().unwrap();
        let (root, files) = targets_in(Path::new("/"), &[dir.path().to_string_lossy().into_owned()]);
        assert_eq!(root, dir.path());
        assert!(files.is_empty());
    }

    #[test]
    fn dots_clean_away() {
        assert_eq!(clean(Path::new("/a/./b/../c")), PathBuf::from("/a/c"));
    }
}
