//! Saving a buffer that has no file, without a window: the menu, the palette, the classic
//! keymap and the unsaved changes dialog ask where to save it; `:w` keeps Vim's E32.

use std::path::PathBuf;

use editor::editor::{Action, Keymap};

use super::App;
use super::fx::{Effect, Tx};
use crate::config::Config;

fn app() -> App {
    let mut a = App::new(Config::default(), PathBuf::from("/p"), false, None);
    // A unit test writes no session file into the user's state folder.
    a.exited = true;
    a
}

/// Types `text` at the end of the active buffer.
fn edit(a: &mut App, text: &str) {
    let buf = &mut a.ed.doc_mut().buf;
    buf.begin(Default::default());
    let at = buf.len_chars();
    buf.replace(at, at, text);
    buf.commit(Default::default());
}

/// An app with one new buffer, edited.
fn untitled() -> App {
    let mut a = app();
    a.new_buffer();
    edit(&mut a, "hello");
    assert!(a.ed.buf().is_dirty());
    assert!(a.ed.buf().path.is_none());
    a
}

/// The save dialog's callback.
type Picked = super::fx::Then<Result<Option<PathBuf>, String>>;

/// Takes the save dialog out of `tx`: the folder it starts in and its callback.
fn save_dialog(tx: &mut Tx) -> Option<(PathBuf, Picked)> {
    let i = tx.effects.iter().position(|e| matches!(e, Effect::ChooseSavePath(..)))?;
    let Effect::ChooseSavePath(start, then) = tx.effects.remove(i) else { unreachable!() };
    Some((start, then))
}

fn message(a: &App) -> String {
    a.ed.msg.as_ref().map(|m| m.text.clone()).unwrap_or_default()
}

#[test]
fn new_file_opens_an_empty_buffer_with_no_name() {
    let mut a = app();
    let mut tx = Tx::new();
    a.run_command("new", &mut tx);
    assert_eq!(a.ed.docs.len(), 1);
    assert!(a.ed.buf().path.is_none());
    a.run_actions(vec![Action::NewBuffer], &mut tx);
    assert_eq!((a.ed.docs.len(), a.ed.active), (2, 1));
}

#[test]
fn save_from_the_palette_asks_where_and_a_cancel_does_nothing() {
    let mut a = untitled();
    let mut tx = Tx::new();
    a.run_command("save", &mut tx);
    let (start, then) = save_dialog(&mut tx).expect("save asks where to save a buffer with no file");
    assert_eq!(start, PathBuf::from("/p"), "the dialog starts in the project");
    assert!(message(&a).is_empty(), "no E32: {}", message(&a));
    then(&mut a, Ok(None), &mut tx);
    assert!(a.ed.buf().path.is_none(), "cancel leaves the buffer as it was");
    assert!(a.ed.buf().is_dirty());
    assert!(save_dialog(&mut tx).is_none());

    a.app_menu_pick("cmd:save", &mut tx);
    let (_, then) = save_dialog(&mut tx).expect("the menu's Save asks too");
    then(&mut a, Ok(Some(PathBuf::from("/p/notes.md"))), &mut tx);
    assert_eq!(a.ed.buf().path.as_deref(), Some(std::path::Path::new("/p/notes.md")));
    assert!(a.meta.get(&PathBuf::from("/p/notes.md")).is_some_and(|m| m.lang.is_some()), "the language follows the name");
}

#[test]
fn a_dialog_that_cannot_show_says_why() {
    let mut a = untitled();
    let mut tx = Tx::new();
    a.run_command("save", &mut tx);
    let (_, then) = save_dialog(&mut tx).unwrap();
    then(&mut a, Err("no file dialogs here".into()), &mut tx);
    assert!(message(&a).contains("no file dialogs here"), "{}", message(&a));
    assert!(a.ed.buf().path.is_none());
}

#[test]
fn vim_w_with_no_name_keeps_e32_and_the_classic_keymap_asks() {
    let mut a = untitled();
    let mut tx = Tx::new();
    a.run_actions(vec![Action::Save(None)], &mut tx);
    assert!(save_dialog(&mut tx).is_none(), ":w shows no dialog");
    assert!(message(&a).contains("E32"), "{}", message(&a));
    a.ed.msg = None;
    a.ed.keymap = Keymap::Classic;
    a.run_actions(vec![Action::Save(None)], &mut tx);
    assert!(save_dialog(&mut tx).is_some(), "Cmd S in the classic keymap asks");
    assert!(message(&a).is_empty(), "{}", message(&a));
}

#[test]
fn save_in_the_quit_dialog_names_the_new_file_then_quits_after_the_write() {
    let mut a = untitled();
    let mut tx = Tx::new();
    a.close_requested(&mut tx);
    let i = tx.effects.iter().position(|e| matches!(e, Effect::Confirm(..))).unwrap();
    let Effect::Confirm(dialog, then) = tx.effects.remove(i) else { unreachable!() };
    assert_eq!(dialog.title, "Save changes to \"[No Name]\"?");
    then(&mut a, Ok(Some(super::unsaved::SAVE.into())), &mut tx);
    assert!(!message(&a).contains("E141"), "{}", message(&a));
    let (_, pick) = save_dialog(&mut tx).expect("Save asks where to save the new file");
    // Cancel in the save dialog keeps the window and the edits.
    pick(&mut a, Ok(None), &mut tx);
    assert!(!tx.effects.iter().any(|e| matches!(e, Effect::Quit)));
    assert_eq!(a.quit_after_saves, None);

    a.close_requested(&mut tx);
    let i = tx.effects.iter().position(|e| matches!(e, Effect::Confirm(..))).unwrap();
    let Effect::Confirm(_, then) = tx.effects.remove(i) else { unreachable!() };
    then(&mut a, Ok(Some(super::unsaved::SAVE.into())), &mut tx);
    let (_, pick) = save_dialog(&mut tx).unwrap();
    pick(&mut a, Ok(Some(PathBuf::from("/p/new.txt"))), &mut tx);
    assert_eq!(a.ed.buf().path.as_deref(), Some(std::path::Path::new("/p/new.txt")));
    assert_eq!(a.quit_after_saves, Some(1), "it quits once the write lands");
}

#[test]
fn save_in_the_tab_dialog_names_each_new_file_in_turn() {
    let mut a = untitled();
    a.new_buffer();
    edit(&mut a, "world");
    let mut tx = Tx::new();
    a.close_dirty(&[0, 1], &mut tx);
    let i = tx.effects.iter().position(|e| matches!(e, Effect::Confirm(..))).unwrap();
    let Effect::Confirm(_, then) = tx.effects.remove(i) else { unreachable!() };
    then(&mut a, Ok(Some(super::unsaved::SAVE.into())), &mut tx);
    let (_, first) = save_dialog(&mut tx).unwrap();
    first(&mut a, Ok(Some(PathBuf::from("/p/one.txt"))), &mut tx);
    let (_, second) = save_dialog(&mut tx).expect("the second new file asks next");
    second(&mut a, Ok(Some(PathBuf::from("/p/two.txt"))), &mut tx);
    let paths: Vec<_> = a.ed.docs.iter().map(|d| d.buf.path.clone()).collect();
    assert_eq!(paths, [Some(PathBuf::from("/p/one.txt")), Some(PathBuf::from("/p/two.txt"))]);
}
