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

/// The frame the window had when it last closed. Read once, before the window opens.
pub fn load_window() -> Option<WindowFrame> {
    let f: WindowFrame = toml::from_str(&std::fs::read_to_string(window_path()).ok()?).ok()?;
    (f.w >= 200.0 && f.h >= 200.0).then_some(f)
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionFile {
    pub path: PathBuf,
    pub line: usize,
    pub col: usize,
}

fn path_for(root: &Path) -> PathBuf {
    let mut h = DefaultHasher::new();
    root.hash(&mut h);
    let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "root".into());
    crate::config::state_dir().join("sessions").join(format!("{name}-{:016x}.toml", h.finish()))
}

/// The session saved for `root`, without files that no longer exist.
pub fn load(root: &Path) -> Option<Session> {
    let src = std::fs::read_to_string(path_for(root)).ok()?;
    let mut s: Session = toml::from_str(&src).ok()?;
    if s.root != root {
        return None;
    }
    s.files.retain(|f| f.path.is_file());
    Some(s)
}

/// Writes the session. Runs once, on the way out.
pub fn save(app: &App) {
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
