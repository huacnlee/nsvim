//! Command-line arguments: `nvim [path[:line]] [+line] [--wait] [--new]`, `PROGRAM ctl` and
//! `PROGRAM mcp`. `nvim` is the shell alias the first-run setup makes, and PROGRAM is the
//! binary's full path. The program does not read its own name, so any alias or path starts it.
//! A folder opens as the project. A file opens in the project around it: its git work tree,
//! else its folder.

use std::path::{Path, PathBuf};

use editor::buffer::Pos;

/// The help text. `usage` puts this binary's full path in place of PROGRAM.
const USAGE: &str = "nsvim: a modal code editor. Read fast. Edit quick.

Usage:
  nvim [PATH[:LINE]]... [+LINE] [--wait] [--new] [--tui]
  PROGRAM ctl COMMAND [ARGS]
  PROGRAM mcp

The first launch runs a short setup. It can make nvim, vim and vi open nsvim with shell
aliases. \\nvim still runs Neovim. Scripts and AI agents do not see aliases, so they run the
program by its full path:
  PROGRAM

Examples:
  nvim .                      open this folder as the project
  nvim src/main.rs:42         open a file at line 42, in the project around it
  nvim --wait NOTES.md        return when its buffer closes, for $EDITOR and git
  nvim --new                  open a new window, also when one runs for the project
  nvim --tui src/main.rs      edit in this terminal, like vim
  PROGRAM ctl --help          drive the running editor
  PROGRAM mcp                 serve the running editor to AI agents, over stdio

nsvim opens a window, also from a terminal. --tui edits in the terminal instead. A path
opens in the window already running for its project, when there is one.

Options:
  --wait          return when the last file's buffer closes
  --new           open a new window
  --tui           edit in this terminal, not in a window
  -h, --help      print this help
  -V, --version   print the version

Docs: https://github.com/huacnlee/nsvim/blob/main/docs/cli.md
";

const MCP_USAGE: &str = "nsvim mcp: a Model Context Protocol server, over stdio, for the editor
running in the current folder. An agent starts it. You do not run it by hand.

  claude mcp add nsvim -- PROGRAM mcp

Tools and setup for Codex and opencode: https://github.com/huacnlee/nsvim/blob/main/docs/agents.md
";

/// This binary's full path, with links resolved, as one shell word.
pub fn program() -> String {
    let exe = std::env::current_exe().ok().map(|e| e.canonicalize().unwrap_or(e));
    exe.map(|p| shell_word(&p.display().to_string())).unwrap_or_else(|| "nsvim".into())
}

/// `word` as one shell word: as it is when it has only safe characters, else in single quotes.
pub fn shell_word(word: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "/._-+=:,@%".contains(c);
    if !word.is_empty() && word.chars().all(safe) { word.to_string() } else { format!("'{}'", word.replace('\'', "'\\''")) }
}

/// The help text, with `program` (a shell word) as the program.
pub fn usage(program: &str) -> String {
    USAGE.replace("PROGRAM", program)
}

/// The help text of `mcp`, with `program` (a shell word) as the program.
pub fn mcp_usage(program: &str) -> String {
    MCP_USAGE.replace("PROGRAM", program)
}

/// What the command line asks for. `argv` includes the program name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `PROGRAM ctl ARGS`: drive a running editor.
    Ctl(Vec<String>),
    /// `PROGRAM mcp`: serve the running editor to an agent over stdio.
    Mcp,
    McpHelp,
    Help,
    Version,
    /// Open the editor. `wait`: return when the last file's buffer closes. `new`: open a
    /// window, also when an editor runs for the project.
    Open {
        wait: bool,
        new: bool,
        /// `--tui`: edit in the terminal it was started in, not in a window.
        tui: bool,
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
        _ => {
            let has = |flag: &str| argv.iter().skip(1).any(|a| a == flag);
            Command::Open { wait: has("--wait"), new: has("--new"), tui: has("--tui") }
        }
    }
}

/// True when this run edits in the terminal, as vim does: `--tui` in a terminal (stdin and
/// stdout are one), or on Linux with no display. Otherwise it opens a window, also when started
/// in a terminal. `-g`, the old flag for a window, is still accepted and changes nothing.
pub fn wants_tui() -> bool {
    use std::io::IsTerminal;
    let argv: Vec<String> = std::env::args().collect();
    let tui = matches!(command(&argv), Command::Open { tui: true, .. });
    let terminal = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let no_display = cfg!(target_os = "linux") && std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none();
    in_terminal(tui, terminal, no_display, std::env::var_os("NSH_DEV_APP_DIR").is_some())
}

/// `wants_tui` with its inputs given. No display means the terminal: there is no window to open.
fn in_terminal(tui: bool, terminal: bool, no_display: bool, dev: bool) -> bool {
    !dev && (no_display || (terminal && tui))
}

/// The project root and the files to open, with a cursor position when one was given.
pub fn targets() -> (PathBuf, Vec<(PathBuf, Option<Pos>)>) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    // Under `nsh run` the program is NSH's development host, and its arguments are the host's
    // own (`--dev-server URL --app-id ID`). The editor then opens the folder it runs in.
    let args: Vec<String> = if std::env::var_os("NSH_DEV_APP_DIR").is_some() { Vec::new() } else { std::env::args().skip(1).collect() };
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
        if a.starts_with("--") || a == "-g" {
            continue;
        }
        let (p, at) = split_line(a.as_str());
        let p = if Path::new(&p).is_absolute() { PathBuf::from(p) } else { cwd.join(p) };
        paths.push((clean(&p), at.map(|l| Pos::new(l, 0))));
    }
    if let (Some(l), Some(first)) = (line, paths.first_mut()) {
        first.1 = Some(Pos::new(l, 0));
    }
    let Some((first, _)) = paths.first().cloned() else { return (launch_dir(&cwd, dirs::home_dir()), Vec::new()) };
    if first.is_dir() {
        let rest = paths.into_iter().skip(1).filter(|(p, _)| !p.is_dir()).collect();
        return (first, rest);
    }
    let dir = first.parent().map(Path::to_path_buf).unwrap_or(cwd);
    let root = git_top(&dir).or_else(|| project_root(&dir)).unwrap_or(dir);
    (root, paths.into_iter().filter(|(p, _)| !p.is_dir()).collect())
}

/// The folder to open when no path was given. A launcher (Finder, the Start menu, a desktop
/// entry) starts the app in `/`, a drive root or the app's own folder; open the home folder then.
fn launch_dir(cwd: &Path, home: Option<PathBuf>) -> PathBuf {
    let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
    let system = std::env::var_os("SystemRoot").map(PathBuf::from);
    launch_dir_with(cwd, home, &[exe_dir, system].into_iter().flatten().collect::<Vec<_>>())
}

/// `launch_dir` with the app's and the system's folders given; `cwd` inside one of them, or at a
/// filesystem root, means a launcher started the app.
fn launch_dir_with(cwd: &Path, home: Option<PathBuf>, app_dirs: &[PathBuf]) -> PathBuf {
    let launched = cwd.parent().is_none() || app_dirs.iter().any(|d| cwd.starts_with(d));
    match home {
        Some(home) if launched && home.is_dir() => home,
        _ => cwd.to_path_buf(),
    }
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
    fn the_help_shows_nvim_and_this_binarys_full_path() {
        let help = usage("/home/dev/.local/share/nsvim/nsvim");
        assert!(help.contains("  nvim .  ") && help.contains("  nvim src/main.rs:42  "), "{help}");
        assert!(help.contains("\\nvim still runs Neovim"), "{help}");
        assert!(help.contains("  /home/dev/.local/share/nsvim/nsvim ctl --help "), "{help}");
        assert!(!help.contains("PROGRAM") && !help.contains("nsvim ."), "{help}");
        assert!(mcp_usage("'/a b/nsvim'").contains("claude mcp add nsvim -- '/a b/nsvim' mcp"));
    }

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
        assert_eq!(command(&argv(&["a.rs", "--wait"])), Command::Open { wait: true, new: false, window: false });
        assert_eq!(command(&argv(&["--new"])), Command::Open { wait: false, new: true, window: false });
        assert_eq!(command(&argv(&[])), Command::Open { wait: false, new: false, tui: false });
        assert_eq!(command(&argv(&["--tui", "a.rs"])), Command::Open { wait: false, new: false, tui: true });
        assert_eq!(command(&argv(&["-g", "a.rs"])), Command::Open { wait: false, new: false, tui: false });
    }

    #[test]
    fn a_terminal_start_opens_a_window_unless_tui() {
        assert!(!in_terminal(false, true, false, false), "a terminal start opens a window");
        assert!(in_terminal(true, true, false, false), "--tui edits in the terminal");
        assert!(!in_terminal(true, false, false, false), "--tui without a terminal opens a window");
        assert!(!in_terminal(false, false, false, false), "a launcher opens a window");
        assert!(in_terminal(false, false, true, false), "no display, no window");
        assert!(!in_terminal(true, true, false, true), "nsh run opens a window");
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
    fn a_launcher_start_opens_the_home_folder() {
        let home = tempfile::tempdir().unwrap();
        let h = Some(home.path().to_path_buf());
        let app = PathBuf::from("/Applications/nsvim.app/Contents/MacOS");
        // Finder and desktop launchers start in `/`, the Start menu in the app's or system folder.
        assert_eq!(launch_dir_with(Path::new("/"), h.clone(), &[]), home.path());
        assert_eq!(launch_dir_with(&app, h.clone(), std::slice::from_ref(&app)), home.path());
        // A terminal start keeps its folder, even above the app's own folder.
        assert_eq!(launch_dir_with(Path::new("/work/proj"), h.clone(), &[app]), Path::new("/work/proj"));
        assert_eq!(
            launch_dir_with(Path::new("/Applications"), h, &[PathBuf::from("/Applications/nsvim.app/Contents/MacOS")]),
            Path::new("/Applications")
        );
        assert_eq!(launch_dir_with(Path::new("/"), None, &[]), Path::new("/"));
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
