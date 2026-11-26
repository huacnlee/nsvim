//! The command palette without a window: how it opens in each keymap, the keys it shows, and
//! the menu items it carries out.

use std::path::PathBuf;

use editor::buffer::Buffer;
use editor::editor::{Doc, KeyCode, KeyInput, Keymap, Mode, VisualKind};

use super::App;
use super::fx::{Effect, Tx};
use super::palette::{COMMANDS, Command};
use super::picker::PickKind;
use super::{Overlay, Screen};

fn app(keymap: Keymap) -> App {
    let cfg = toml::from_str("[ipc]\nenabled = false\n").unwrap();
    let mut a = App::new(cfg, PathBuf::from("/project"), false, None);
    a.screen = Screen::Editor;
    a.ed.keymap = keymap;
    a.ed.mode = Mode::Normal;
    a
}

fn with_doc(keymap: Keymap, text: &str) -> App {
    let mut a = app(keymap);
    a.ed.docs.push(Doc::new(Buffer::from_str(text)));
    a.ed.active = 0;
    a
}

fn palette_open(a: &App) -> bool {
    matches!(&a.overlay, Overlay::Pick(p) if p.kind == PickKind::Commands)
}

/// `Ctrl ⇧ P`, as NSH sends it off a Mac: Control sets `cmd` too.
fn ctrl_shift_p() -> KeyInput {
    KeyInput { code: KeyCode::Char('P'), ctrl: true, alt: false, shift: true, cmd: true }
}

fn key(a: &mut App, k: KeyInput) {
    a.key(k, &mut Tx::new());
}

#[test]
fn ctrl_shift_p_toggles_the_palette_in_every_keymap() {
    for keymap in [Keymap::Vim, Keymap::Classic] {
        // With a file open and with none.
        for mut a in [with_doc(keymap, "x\n"), app(keymap)] {
            key(&mut a, ctrl_shift_p());
            assert!(palette_open(&a), "{keymap:?} opens it");
            key(&mut a, ctrl_shift_p());
            assert!(matches!(a.overlay, Overlay::None), "{keymap:?} closes it again");
        }
    }
}

#[test]
fn the_neovim_keymap_opens_the_palette_with_the_leader_and_p() {
    let mut a = with_doc(Keymap::Vim, "x\n");
    key(&mut a, KeyInput::ch(' '));
    assert!(matches!(a.overlay, Overlay::Leader(None)));
    key(&mut a, KeyInput::ch('p'));
    assert!(palette_open(&a), "SPC p");
    a.overlay = Overlay::None;
    // With no file open too.
    let mut a = app(Keymap::Vim);
    key(&mut a, KeyInput::ch(' '));
    key(&mut a, KeyInput::ch('p'));
    assert!(palette_open(&a), "SPC p with no file open");
}

#[test]
fn each_row_shows_the_keys_of_the_active_keymap() {
    let find = COMMANDS.iter().find(|c| c.id == "find").unwrap();
    assert_eq!(find.keys(Keymap::Vim, ' ', false), "SPC f");
    assert_eq!(find.keys(Keymap::Vim, '\\', false), "\\ f");
    assert_eq!(find.keys(Keymap::Classic, ' ', false), "Ctrl P");
    assert_eq!(find.keys(Keymap::Classic, ' ', true), "⌘ P");
    let next = COMMANDS.iter().find(|c| c.id == "next").unwrap();
    assert_eq!(next.keys(Keymap::Classic, ' ', true), "Ctrl Tab", "a real Control key stays Ctrl on a Mac");
    let folder = COMMANDS.iter().find(|c| c.id == "folder").unwrap();
    assert_eq!(folder.keys(Keymap::Vim, ' ', false), "");
}

#[test]
fn ids_and_labels_are_unique() {
    for (i, c) in COMMANDS.iter().enumerate() {
        let Command { label, id, .. } = c;
        assert!(COMMANDS[i + 1..].iter().all(|o| o.id != *id && o.label != *label), "{label} ({id}) shows twice");
    }
}

#[test]
fn undo_and_redo_run_from_the_palette() {
    let mut a = with_doc(Keymap::Vim, "abc\n");
    let doc = a.ed.doc_mut();
    doc.buf.begin(Default::default());
    doc.buf.replace(0, 1, "x");
    doc.buf.commit(Default::default());
    let mut tx = Tx::new();
    a.run_command("undo", &mut tx);
    assert_eq!(a.ed.buf().slice(0, 3), "abc");
    a.run_command("redo", &mut tx);
    assert_eq!(a.ed.buf().slice(0, 3), "xbc");
}

#[test]
fn copy_and_select_all_run_from_the_palette_over_the_selection() {
    let mut a = with_doc(Keymap::Vim, "one\ntwo\n");
    let mut tx = Tx::new();
    a.run_command("all", &mut tx);
    assert_eq!(a.ed.mode, Mode::Visual(VisualKind::Line));
    a.run_command("copy", &mut tx);
    assert!(tx.effects.iter().any(|e| matches!(e, Effect::Copy(t) if t.starts_with("one\ntwo"))), "copy puts the lines on the clipboard");
    assert_eq!(a.ed.mode, Mode::Normal);
}

#[test]
fn file_commands_with_no_file_open_say_so() {
    let mut a = app(Keymap::Vim);
    for id in ["undo", "copy", "definition"] {
        a.ed.msg = None;
        a.run_command(id, &mut Tx::new());
        assert!(a.ed.msg.as_ref().is_some_and(|m| m.text == "no file is open"), "{id}");
    }
}

#[test]
fn open_folder_asks_the_system_for_a_folder() {
    let mut a = app(Keymap::Vim);
    let mut tx = Tx::new();
    a.run_command("folder", &mut tx);
    assert!(tx.effects.iter().any(|e| matches!(e, Effect::ChooseFolder(..))));
}
