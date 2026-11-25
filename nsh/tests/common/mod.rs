//! Helpers every Headless test shares: a scratch project, the app started on it on a Linux
//! desktop, and keys typed one after another.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use nsh_core::platform::Os;
use nsh_engine::{EngineConfig, Points, Size};
use nsh_testing::Headless;

/// Config for tests: no control socket.
pub const CONFIG: &str = "[ipc]\nenabled = false\n";

/// A folder for test files, outside /tmp and outside the git checkout.
pub fn scratch() -> tempfile::TempDir {
    let base = std::env::var_os("TMPDIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    std::fs::create_dir_all(&base).unwrap();
    tempfile::tempdir_in(base).unwrap()
}

/// Keeps the editor's state and config files in a scratch folder, not in the user's home.
pub fn isolate_home() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let base = std::env::var_os("TMPDIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("editor-nsh-home");
        // SAFETY: set once, before any test starts the app.
        unsafe {
            std::env::set_var("XDG_STATE_HOME", base.join("state"));
            std::env::set_var("XDG_CONFIG_HOME", base.join("config"));
        }
    });
}

/// The app on a Linux desktop with `launch`. The editor is a desktop app, and its root is a
/// window, which Headless shows only on a desktop OS.
pub fn start_with(launch: nsh_app_editor::Launch) -> Headless {
    isolate_home();
    let mut config =
        EngineConfig { viewport: Size::new(Points(1200.0), Points(800.0)), ..EngineConfig::new(nsh_app_editor::root_with(launch)) };
    config.environment.os = Os::Linux;
    Headless::start(config).unwrap()
}

/// The app with `files` open, in their folder as the project, with `config` (or the tests'
/// default config).
pub fn start_files(files: &[&Path], config: Option<&str>) -> Headless {
    start_with(nsh_app_editor::Launch {
        project: files.first().and_then(|f| f.parent()).map(Path::to_path_buf),
        config: Some(config.unwrap_or(CONFIG).into()),
        files: files.iter().map(|f| f.to_path_buf()).collect(),
        first_launch: false,
    })
}

/// The app with one file open.
pub fn start(file: &Path) -> Headless {
    start_files(&[file], None)
}

/// Writes `text` to `name` in `dir` and returns its path.
pub fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let p = dir.join(name);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&p, text).unwrap();
    p
}

/// Presses each key in turn (`"j"`, `"shift+a"`, `"escape"`, `"ctrl+r"`).
pub fn typ(app: &mut Headless, keys: &[&str]) {
    for k in keys {
        app.press_key(k).unwrap();
    }
}

/// Types each char of `text` as a key: letters, digits, symbols and spaces.
pub fn type_text(app: &mut Headless, text: &str) {
    for c in text.chars() {
        let key = match c {
            ' ' => "space".to_string(),
            '\n' => "enter".to_string(),
            c if c.is_ascii_uppercase() => format!("shift+{}", c.to_ascii_lowercase()),
            ':' => "shift+;".into(),
            '!' => "shift+1".into(),
            '?' => "shift+/".into(),
            '_' => "shift+-".into(),
            '(' => "shift+9".into(),
            ')' => "shift+0".into(),
            '{' => "shift+[".into(),
            '}' => "shift+]".into(),
            '"' => "shift+'".into(),
            c => c.to_string(),
        };
        app.press_key(&key).unwrap();
    }
}
