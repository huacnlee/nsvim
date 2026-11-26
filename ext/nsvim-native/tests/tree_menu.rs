//! The tree's right-click menu through the real window: each item, its native dialog and the
//! answer, with the real file worker and index.

mod common;

use std::time::Duration;

use nsh_testing::DialogAnswer;

const WAIT: Duration = Duration::from_secs(5);

/// A project with `a.txt` open and a folder `src` with one file. The test answers the dialogs.
fn project() -> (tempfile::TempDir, common::Flow) {
    let dir = common::scratch();
    let a = common::write(dir.path(), "a.txt", "alpha\n");
    common::write(dir.path(), "src/lib.rs", "pub fn x() {}\n");
    let mut f = common::flow(&a, None);
    f.update(|a, _| a.tree.show = true);
    f.ui.wait_for("role:treeitem[src]").unwrap();
    f.ui.answer_dialogs_myself();
    (dir, f)
}

/// Right-clicks the tree row `row`, picks the menu item that starts with `item`, and waits for
/// the dialog with the title `title`.
fn ask(f: &mut common::Flow, row: &str, item: &str, title: &str) {
    f.ui.right_click(&format!("role:treeitem[{row}]")).unwrap();
    let menu = f.ui.menu().expect("the row's menu shows");
    let label = menu.iter().map(|e| e.label.clone()).find(|l| l.starts_with(item)).unwrap_or_else(|| panic!("{item} in {menu:?}"));
    f.ui.choose_menu_item(&label).unwrap();
    let want = title.to_string();
    f.ui.wait_until(title, WAIT, move |s| s.dialogs().iter().any(|d| d.title.as_deref() == Some(want.as_str()))).unwrap();
}

fn type_and_press(f: &mut common::Flow, text: &str, button: &str) {
    f.ui.answer_dialog(DialogAnswer::TypeAndPress { text: text.into(), button: button.into() }).unwrap();
}

#[test]
fn new_file_from_the_menu_makes_the_file_in_the_row_folder_and_opens_it() {
    let (dir, mut f) = project();
    ask(&mut f, "src", "New file", "New file");
    type_and_press(&mut f, "main.rs", "Create");
    let main = dir.path().join("src/main.rs");
    let m = main.clone();
    f.settle("the new file opens", move |a| a.ed.has_doc() && a.ed.buf().path.as_deref() == Some(m.as_path()));
    assert!(main.is_file());
    assert!(f.read(|a| a.ed.cmd.is_none()), "the command line stays closed");
}

#[test]
fn new_folder_from_the_menu_makes_an_empty_folder() {
    let (dir, mut f) = project();
    ask(&mut f, "a.txt", "New folder", "New folder");
    type_and_press(&mut f, "docs", "Create");
    f.ui.wait_for("role:treeitem[docs]").unwrap();
    assert!(dir.path().join("docs").is_dir());
}

#[test]
fn rename_from_the_menu_renames_the_row_and_the_open_buffer_follows() {
    let (dir, mut f) = project();
    ask(&mut f, "a.txt", "Rename", "Rename \"a.txt\"");
    type_and_press(&mut f, "b.txt", "Rename");
    let b = dir.path().join("b.txt");
    let want = b.clone();
    f.settle("the buffer has the new name", move |a| a.ed.buf().path.as_deref() == Some(want.as_path()));
    assert!(b.is_file() && !dir.path().join("a.txt").exists());
    f.ui.wait_for("role:treeitem[b.txt]").unwrap();
}

#[test]
fn delete_from_the_menu_asks_and_only_the_delete_button_deletes() {
    let (dir, mut f) = project();
    ask(&mut f, "src", "Delete", "Delete \"src\"?");
    f.ui.answer_dialog(DialogAnswer::Press("Cancel".into())).unwrap();
    assert!(dir.path().join("src/lib.rs").is_file(), "cancel keeps the folder");
    ask(&mut f, "src", "Delete", "Delete \"src\"?");
    f.ui.answer_dialog(DialogAnswer::Press("Delete".into())).unwrap();
    f.ui.wait_for_gone("role:treeitem[src]", WAIT).unwrap();
    assert!(!dir.path().join("src").exists(), "the folder goes with all in it");
}

#[test]
fn copy_relative_path_from_the_menu_says_what_it_copied() {
    let (_dir, mut f) = project();
    f.ui.right_click("role:treeitem[a.txt]").unwrap();
    f.ui.choose_menu_item("Copy relative path").unwrap();
    f.settle("it says what it copied", |a| a.ed.msg.as_ref().is_some_and(|m| m.text == "copied a.txt"));
}
