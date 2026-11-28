//! The desktop editor keys on macOS, through NSH's key capture and the real key routing: `⌘`
//! shortcuts in the Neovim keymap, and the editing keys of the system's text fields.
//! The core's tests cover each key. These prove that the keys reach it on macOS. Headless on
//! macOS gives no plain key to the key view, so Normal mode keys are left to the core's tests.

#![cfg(target_os = "macos")]

mod common;

use common::{SRC, rust_flow, use_macos};
use editor::editor::{KeyInput, Mode};

/// Puts the Neovim keymap in Insert mode at the end of line 2. Headless on macOS does not
/// give a plain key to the key view, so the test types `jA` through the app.
fn insert_at_line_two(f: &mut common::Flow) {
    f.update(|a, tx| {
        for c in "jA".chars() {
            a.key(KeyInput::ch(c), tx);
        }
    });
    f.settle("Insert mode takes the keyboard", |a| a.ed.mode == Mode::Insert && a.key_focus);
    f.ui.wait(std::time::Duration::from_millis(500)).unwrap();
}

#[test]
fn cmd_and_alt_backspace_then_cmd_z_in_neovim_insert_mode() {
    use_macos();
    let (_dir, mut f) = rust_flow("neovim", SRC);
    insert_at_line_two(&mut f);
    f.press("alt+backspace");
    f.settle("the word before the cursor goes", |a| a.ed.buf().text().contains("    let a = 1\n"));
    f.press("cmd+backspace");
    f.settle("the line goes to the indent", |a| a.ed.buf().text().contains("\n    \n    let b"));
    assert_eq!(f.read(|a| a.ed.mode), Mode::Insert);
    f.press("cmd+z");
    f.settle("the undo restores the line", |a| a.ed.buf().text() == SRC);
    assert_eq!(f.read(|a| a.ed.mode), Mode::Insert, "the insert goes on");
}

#[test]
fn cmd_s_saves_from_neovim_insert_mode() {
    use_macos();
    let (dir, mut f) = rust_flow("neovim", SRC);
    insert_at_line_two(&mut f);
    f.press("cmd+backspace");
    f.press("cmd+s");
    f.settle("the file saves", |a| !a.ed.buf().is_dirty());
    assert!(!std::fs::read_to_string(dir.path().join("main.rs")).unwrap().contains("let a"));
}

#[test]
fn cmd_backspace_and_ctrl_e_in_the_classic_keymap() {
    use_macos();
    let (_dir, mut f) = rust_flow("classic", SRC);
    f.press("ctrl+e");
    f.press("cmd+backspace");
    assert!(f.text().starts_with("\n    let a"), "{}", f.text());
}
