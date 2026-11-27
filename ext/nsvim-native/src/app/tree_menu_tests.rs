//! The tree's menu without a window: the items, the dialogs each one asks with, and what a
//! cancel or a missing dialog does. `tests/tree_files.rs` makes the changes on a real disk.

use std::path::PathBuf;

use super::super::App;
use super::super::fx::{Confirm, Effect, Role, Then, Tx};
use super::{OK, TREE_ITEMS};

/// An app in `/project` whose tree has `src/lib.rs` and `a.txt`.
fn app() -> App {
    let cfg = toml::from_str("[ipc]\nenabled = false\n").unwrap();
    let mut a = App::new(cfg, PathBuf::from("/project"), false, None);
    a.screen = super::super::Screen::Editor;
    a.tree.index = super::super::tree::TreeIndex::build(&["a.txt".into(), "src/lib.rs".into()]);
    a.tree.show = true;
    a
}

type Answer = Then<Result<Option<String>, String>>;

/// The one dialog the app asked for: the alert, the text its field starts with (None for an
/// alert without a field), and what runs with its answer.
fn take_dialog(tx: &mut Tx) -> (Confirm, Option<String>, Answer) {
    let i = tx.effects.iter().position(|e| matches!(e, Effect::Confirm(..) | Effect::Prompt(..))).expect("a dialog");
    match tx.effects.remove(i) {
        Effect::Confirm(d, then) => (d, None, then),
        Effect::Prompt(d, text, then) => (d, Some(text), then),
        _ => unreachable!(),
    }
}

fn pick(a: &mut App, path: &str, id: &str) -> (Confirm, Option<String>, Answer) {
    let mut tx = Tx::new();
    a.tree_menu_pick(path, id, &mut tx);
    take_dialog(&mut tx)
}

#[test]
fn the_menu_shows_every_item_with_its_tree_key_and_takes_the_row() {
    let mut a = app();
    let mut tx = Tx::new();
    a.tree_menu("src/lib.rs".into(), (10.0, 20.0), &mut tx);
    let menu = tx
        .effects
        .into_iter()
        .find_map(|e| match e {
            Effect::PopupMenu { menu, .. } => Some(menu),
            _ => None,
        })
        .expect("a right-click on a row opens the system's menu");
    assert_eq!(menu.items.len(), TREE_ITEMS.len());
    let titles: Vec<String> = menu.items.iter().flatten().map(|i| i.title.clone()).collect();
    assert_eq!(
        titles,
        [
            "New file…    a",
            "New folder…    A",
            "Rename…    r",
            "Delete…    d",
            "Copy path",
            "Copy relative path",
            crate::services::reveal::TITLE
        ]
    );
    assert!(menu.items.iter().flatten().all(|i| i.enabled));
    assert_eq!(a.tree.sel.as_deref(), Some("src/lib.rs"), "the row the menu works on shows");
    assert!(a.tree.focus, "the tree keys work on the row next");
}

#[test]
fn new_file_and_new_folder_ask_for_a_name_in_the_row_folder() {
    let mut a = app();
    let (d, text, _) = pick(&mut a, "src/lib.rs", "new-file");
    assert_eq!(d.title, "New file");
    assert_eq!(d.description, "In \"src/\".", "a file row's folder");
    assert_eq!(text.as_deref(), Some(""), "a text field, empty");
    assert_eq!(d.buttons[0], (OK.to_string(), "Create".to_string(), Role::Default));
    let (d, _, _) = pick(&mut a, "src", "new-folder");
    assert_eq!(d.title, "New folder");
    assert_eq!(d.description, "In \"src/\".", "a folder row is the folder");
    let (d, _, _) = pick(&mut a, "a.txt", "new-file");
    assert_eq!(d.description, "In the project folder.");
}

#[test]
fn a_cancel_or_an_empty_name_changes_nothing() {
    let mut a = app();
    for id in ["new-file", "new-folder", "rename"] {
        let (_, _, then) = pick(&mut a, "a.txt", id);
        then(&mut a, Ok(None), &mut Tx::new());
        assert!(a.ed.msg.is_none(), "{id}: a cancel is quiet");
    }
    let (_, _, then) = pick(&mut a, "a.txt", "new-file");
    then(&mut a, Ok(Some("  ".into())), &mut Tx::new());
    assert!(a.ed.msg.is_none(), "no name makes nothing");
    let (_, _, then) = pick(&mut a, "a.txt", "rename");
    then(&mut a, Ok(Some("a.txt".into())), &mut Tx::new());
    assert!(a.ed.msg.is_none(), "the same name moves nothing");
    let (_, _, then) = pick(&mut a, "a.txt", "delete");
    then(&mut a, Ok(Some("cancel".into())), &mut Tx::new());
    assert!(a.ed.msg.is_none() && a.tree.confirm.is_none(), "cancel deletes nothing");
}

#[test]
fn an_answer_goes_to_the_file_worker() {
    // No file worker runs here, so the error says that the change went to it.
    let mut a = app();
    let (_, _, then) = pick(&mut a, "src/lib.rs", "new-file");
    then(&mut a, Ok(Some("main.rs".into())), &mut Tx::new());
    assert!(a.ed.msg.as_ref().is_some_and(|m| m.error && m.text.contains("file worker")));
    a.ed.msg = None;
    let (_, _, then) = pick(&mut a, "a.txt", "delete");
    then(&mut a, Ok(Some(OK.into())), &mut Tx::new());
    assert!(a.ed.msg.as_ref().is_some_and(|m| m.error && m.text.contains("file worker")));
}

#[test]
fn rename_starts_with_the_name_and_delete_names_what_goes() {
    let mut a = app();
    let (d, text, _) = pick(&mut a, "src/lib.rs", "rename");
    assert_eq!(d.title, "Rename \"lib.rs\"");
    assert_eq!(text.as_deref(), Some("lib.rs"), "the field starts with the name, not the path");
    let (d, text, _) = pick(&mut a, "src", "delete");
    assert_eq!(text, None, "an alert without a field");
    assert_eq!(d.title, "Delete \"src\"?");
    assert!(d.description.starts_with("The folder \"src\" and all in it"), "{}", d.description);
    assert_eq!(d.buttons[0], (OK.to_string(), "Delete".to_string(), Role::Destructive));
    assert_eq!(d.buttons[1].2, Role::Cancel);
    let (d, _, _) = pick(&mut a, "a.txt", "delete");
    assert_eq!(d.description, "\"a.txt\" goes from the disk. You cannot undo this.");
}

#[test]
fn with_no_dialog_the_command_line_and_the_y_key_ask() {
    let mut a = app();
    let cmd = |a: &App| a.ed.cmd.as_ref().map(|c| c.text.clone());
    let (_, _, then) = pick(&mut a, "src/lib.rs", "new-file");
    then(&mut a, Err("no dialogs here".into()), &mut Tx::new());
    assert_eq!(cmd(&a).as_deref(), Some("NewFile src/"));
    let (_, _, then) = pick(&mut a, "src", "new-folder");
    then(&mut a, Err("no dialogs here".into()), &mut Tx::new());
    assert_eq!(cmd(&a).as_deref(), Some("Mkdir src/"));
    let (_, _, then) = pick(&mut a, "src/lib.rs", "rename");
    then(&mut a, Err("no dialogs here".into()), &mut Tx::new());
    assert_eq!(cmd(&a).as_deref(), Some("Move src/lib.rs"));
    let (_, _, then) = pick(&mut a, "a.txt", "delete");
    then(&mut a, Err("no dialogs here".into()), &mut Tx::new());
    assert_eq!(a.tree.confirm, Some(PathBuf::from("/project/a.txt")), "y deletes next");
}

#[test]
fn copy_path_and_copy_relative_path_copy_the_row_path() {
    let mut a = app();
    let copied = |a: &mut App, id: &str| {
        let mut tx = Tx::new();
        a.tree_menu_pick("src/lib.rs", id, &mut tx);
        tx.effects.into_iter().find_map(|e| match e {
            Effect::Copy(t) => Some(t),
            _ => None,
        })
    };
    assert_eq!(copied(&mut a, "copy-path").as_deref(), Some("/project/src/lib.rs"));
    assert_eq!(copied(&mut a, "copy-relative-path").as_deref(), Some("src/lib.rs"));
}

#[test]
fn reveal_shows_the_row_in_the_file_manager() {
    let mut a = app();
    let mut tx = Tx::new();
    a.tree_menu_pick("src/lib.rs", "reveal-in-file-manager", &mut tx);
    assert!(matches!(tx.effects.as_slice(), [Effect::Reveal(p)] if p == &PathBuf::from("/project/src/lib.rs")));
}
