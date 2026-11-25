//! The real app, headless: keys go through NSH's focus and key capture to the core keymap,
//! and file reads and writes run on their threads.

use std::path::{Path, PathBuf};

use nsh_core::platform::Os;
use nsh_engine::{EngineConfig, Points, Size};
use nsh_testing::Headless;

/// A folder for test files, outside /tmp and outside the git checkout.
fn scratch() -> tempfile::TempDir {
    let base = std::env::var_os("TMPDIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    std::fs::create_dir_all(&base).unwrap();
    tempfile::tempdir_in(base).unwrap()
}

/// The app on a Linux desktop: the editor is a desktop app, and its root is a window.
fn start(file: &Path) -> Headless {
    let mut config = EngineConfig {
        viewport: Size::new(Points(1200.0), Points(800.0)),
        ..EngineConfig::new(nsh_app_editor::root(Some(file.to_string_lossy().into_owned())))
    };
    config.environment.os = Os::Linux;
    Headless::start(config).unwrap()
}

fn typ(app: &mut Headless, keys: &[&str]) {
    for k in keys {
        app.press_key(k).unwrap();
    }
}

#[test]
fn a_file_opens_with_its_name_and_language() -> Result<(), String> {
    let dir = scratch();
    let file = dir.path().join("main.rs");
    std::fs::write(&file, "fn main() {\n    println!(\"hi\");\n}\n").unwrap();
    let mut app = start(&file);
    app.wait_for("text:/println/")?;
    assert!(app.shows("NORMAL"), "{:?}", app.screen().texts());
    assert!(app.shows("rust"));
    assert!(app.shows("1:1"));
    Ok(())
}

#[test]
fn keys_move_the_cursor_and_edit_then_w_writes_the_file() -> Result<(), String> {
    let dir = scratch();
    let file = dir.path().join("notes.txt");
    std::fs::write(&file, "one\ntwo\n").unwrap();
    let mut app = start(&file);
    app.wait_for("text:/two/")?;
    typ(&mut app, &["j", "shift+a"]);
    app.wait_for("INSERT")?;
    typ(&mut app, &["shift+1", "escape"]);
    app.wait_for("text:/two!/")?;
    app.wait_for("NORMAL")?;
    assert!(app.shows("2:4"), "{:?}", app.screen().texts());
    typ(&mut app, &["shift+;", "w", "enter"]);
    app.wait_for("text:/written/")?;
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "one\ntwo!\n");
    Ok(())
}

#[test]
fn visual_mode_shows_in_the_status_line() -> Result<(), String> {
    let dir = scratch();
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "hello world\n").unwrap();
    let mut app = start(&file);
    app.wait_for("text:/hello/")?;
    typ(&mut app, &["v", "e"]);
    app.wait_for("VISUAL")?;
    typ(&mut app, &["escape"]);
    app.wait_for("NORMAL")?;
    Ok(())
}
