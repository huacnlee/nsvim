//! The command palette without a window: how it opens in each keymap, the keys it shows, and
//! the menu items it carries out.

use std::path::PathBuf;

use editor::buffer::Buffer;
use editor::editor::{Doc, KeyCode, KeyInput, Keymap, Mode, VisualKind};

use super::App;
use super::fx::{Effect, Tx};
use super::palette::{COMMANDS, Command, GROUPS, PALETTE_KEY, leader_command, leader_menu};
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
    assert_eq!(folder.keys(Keymap::Vim, ' ', false), "SPC O");
    assert_eq!(folder.keys(Keymap::Classic, ' ', false), "");
    let undo = COMMANDS.iter().find(|c| c.id == "undo").unwrap();
    assert_eq!(undo.keys(Keymap::Vim, ' ', false), "u, SPC y u", "the keymap's own keys, then the leader's");
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

#[test]
fn the_leader_menu_and_the_palette_hold_the_same_commands() {
    // Every palette command has leader keys, and those keys find it.
    for c in COMMANDS {
        let keys: Vec<char> = c.leader.chars().filter(|&k| k != ' ').collect();
        let found = match keys[..] {
            [k] => leader_command(None, k),
            [g, k] => {
                assert!(GROUPS.iter().any(|x| x.0 == g), "{} ({}) is in no group", c.label, c.id);
                leader_command(Some(g), k)
            }
            _ => panic!("{} ({}) has no leader keys", c.label, c.id),
        };
        assert_eq!(found.map(|f| f.id), Some(c.id), "SPC {}", c.leader);
    }
    // Every key the leader menu shows opens a group, opens the palette, or runs a command.
    let mut seen = 0;
    for (k, _, group) in leader_menu(None) {
        if group {
            let items = leader_menu(Some(k));
            assert!(!items.is_empty(), "SPC {k} is an empty group");
            for (key, _, _) in items {
                assert!(leader_command(Some(k), key).is_some(), "SPC {k} {key}");
                seen += 1;
            }
        } else if k != PALETTE_KEY {
            assert!(leader_command(None, k).is_some(), "SPC {k}");
            seen += 1;
        }
    }
    assert_eq!(seen, COMMANDS.len(), "the menu shows every command once");
}

#[test]
fn each_leader_menu_has_one_command_per_key() {
    let mut menus = vec![leader_menu(None)];
    menus.extend(GROUPS.iter().map(|g| leader_menu(Some(g.0))));
    for menu in menus {
        for (i, (k, label, _)) in menu.iter().enumerate() {
            assert!(menu[i + 1..].iter().all(|o| o.0 != *k), "{k} ({label}) shows twice");
        }
    }
    // A group drops its name from the labels.
    let git = leader_menu(Some('g'));
    assert!(git.iter().any(|(k, l, _)| *k == 'n' && l == "Next change"), "{git:?}");
}

#[test]
fn leader_keys_run_the_palette_commands() {
    let mut a = with_doc(Keymap::Vim, "abc\n");
    let doc = a.ed.doc_mut();
    doc.buf.begin(Default::default());
    doc.buf.replace(0, 1, "x");
    doc.buf.commit(Default::default());
    for k in [' ', 'y', 'u'] {
        key(&mut a, KeyInput::ch(k));
    }
    assert_eq!(a.ed.buf().slice(0, 3), "abc", "SPC y u undoes");
    key(&mut a, KeyInput::ch(' '));
    key(&mut a, KeyInput::ch('b'));
    assert!(matches!(&a.overlay, Overlay::Pick(p) if p.kind == PickKind::Buffers), "SPC b");
}
