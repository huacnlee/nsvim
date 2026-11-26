//! The last session per project: open files, cursor positions and open folders, so the
//! editor reopens where it was. Written on quit, read at startup.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::App;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub root: PathBuf,
    pub files: Vec<SessionFile>,
    pub active: Option<PathBuf>,
    pub tree_open: Vec<String>,
}

/// The window's size in points and position in desktop pixels, for the next launch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowFrame {
    pub w: f32,
    pub h: f32,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

fn window_path() -> PathBuf {
    crate::config::state_dir().join("window.toml")
}

/// The state folder to read at startup: the old one until the copy to the new one ran.
fn read_dir() -> PathBuf {
    crate::config::readable(crate::config::state_dir(), crate::config::old_state_dir())
}

/// The frame the window had when it last closed. Read once, before the window opens.
pub fn load_window() -> Option<WindowFrame> {
    let f: WindowFrame = toml::from_str(&std::fs::read_to_string(read_dir().join("window.toml")).ok()?).ok()?;
    (f.w >= 200.0 && f.h >= 200.0).then_some(f)
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionFile {
    pub path: PathBuf,
    pub line: usize,
    pub col: usize,
}

/// The session file of `root` in the state folder `state`.
fn path_in(state: &Path, root: &Path) -> PathBuf {
    let mut h = DefaultHasher::new();
    root.hash(&mut h);
    let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "root".into());
    state.join("sessions").join(format!("{name}-{:016x}.toml", h.finish()))
}

fn path_for(root: &Path) -> PathBuf {
    path_in(&crate::config::state_dir(), root)
}

/// The session saved for `root`, without files that no longer exist.
pub fn load(root: &Path) -> Option<Session> {
    let src = std::fs::read_to_string(path_in(&read_dir(), root)).ok()?;
    let mut s: Session = toml::from_str(&src).ok()?;
    if s.root != root {
        return None;
    }
    s.files.retain(|f| f.path.is_file());
    Some(s)
}

/// The projects that have a session, newest first, without `current`, the home folder and
/// folders that no longer exist: the recent projects that `Open project` lists.
pub fn recent(current: &Path) -> Vec<PathBuf> {
    recent_in(&read_dir(), current, dirs::home_dir().as_deref())
}

/// The project of the newest session that still exists: the project to open when the
/// command line names none.
pub fn last() -> Option<PathBuf> {
    last_in(&read_dir())
}

fn last_in(state: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(state.join("sessions")).ok()?;
    let mut found: Vec<(std::time::SystemTime, PathBuf)> =
        entries.flatten().filter_map(|e| Some((e.metadata().and_then(|m| m.modified()).ok()?, e.path()))).collect();
    found.sort_by_key(|e| std::cmp::Reverse(e.0));
    found.into_iter().find_map(|(_, file)| {
        let s: Session = toml::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
        s.root.is_dir().then_some(s.root)
    })
}

/// How many recent projects `recent` gives at most.
const RECENT: usize = 30;

fn recent_in(state: &Path, current: &Path, home: Option<&Path>) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(state.join("sessions")) else { return Vec::new() };
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let when = e.metadata().and_then(|m| m.modified()).ok()?;
            let s: Session = toml::from_str(&std::fs::read_to_string(e.path()).ok()?).ok()?;
            Some((when, s.root))
        })
        .filter(|(_, root)| root != current && Some(root.as_path()) != home && root.is_dir())
        .collect();
    found.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out: Vec<PathBuf> = Vec::new();
    for (_, root) in found {
        if !out.contains(&root) {
            out.push(root);
        }
    }
    out.truncate(RECENT);
    out
}

/// Writes the session. Runs once, on the way out.
pub fn save(app: &App) {
    // A unit test that quits must not write into the user's state folder.
    if cfg!(test) && std::env::var_os("XDG_STATE_HOME").is_none() {
        return;
    }
    let files = app
        .ed
        .docs
        .iter()
        .filter_map(|d| Some(SessionFile { path: d.buf.path.clone()?, line: d.view.cursor.line, col: d.view.cursor.col }))
        .collect();
    let active = app.ed.has_doc().then(|| app.ed.doc().buf.path.clone()).flatten();
    let mut tree_open: Vec<String> = app.tree.open.iter().cloned().collect();
    tree_open.sort();
    let s = Session { root: app.ed.root.clone(), files, active, tree_open };
    let path = path_for(&app.ed.root);
    if let (Some(dir), Ok(text)) = (path.parent(), toml::to_string(&s)) {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(&path, text);
    }
    if app.window.w >= 200.0
        && let Ok(text) = toml::to_string(&app.window)
    {
        let _ = std::fs::write(window_path(), text);
    }
}

#[cfg(test)]
mod recent_tests {
    use super::*;

    #[test]
    fn recent_projects_skip_the_current_one_home_and_missing_folders() {
        let state = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let (a, b, gone) = (dir.path().join("a"), dir.path().join("b"), dir.path().join("gone"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::create_dir_all(state.path().join("sessions")).unwrap();
        for root in [&a, &b, &gone, &dir.path().to_path_buf()] {
            let s = Session { root: root.clone(), ..Default::default() };
            std::fs::write(path_in(state.path(), root), toml::to_string(&s).unwrap()).unwrap();
        }
        let got = recent_in(state.path(), &a, Some(dir.path()));
        assert_eq!(got, vec![b]);
    }

    #[test]
    fn the_last_project_is_the_newest_session_that_still_exists() {
        let state = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(last_in(state.path()), None, "no session yet");
        let (old, new, gone) = (dir.path().join("old"), dir.path().join("new"), dir.path().join("gone"));
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::create_dir_all(state.path().join("sessions")).unwrap();
        let now = std::time::SystemTime::now();
        for (root, age) in [(&old, 30), (&new, 20), (&gone, 10)] {
            let path = path_in(state.path(), root);
            std::fs::write(&path, toml::to_string(&Session { root: root.clone(), ..Default::default() }).unwrap()).unwrap();
            let f = std::fs::File::options().write(true).open(&path).unwrap();
            f.set_modified(now - std::time::Duration::from_secs(age)).unwrap();
        }
        assert_eq!(last_in(state.path()), Some(new));
    }
}
