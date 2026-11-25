//! File commands in the tree: make, rename, move and delete files and folders, with the real
//! file worker and index. Keys go through the real window, as a user types them.

mod common;

use std::time::Duration;

const WAIT: Duration = Duration::from_secs(5);

/// A project with `a.txt` open and a folder `src` with one file, the tree focused.
fn project(config: Option<&str>) -> (tempfile::TempDir, common::Flow) {
    let dir = common::scratch();
    let a = common::write(dir.path(), "a.txt", "alpha\n");
    common::write(dir.path(), "src/lib.rs", "pub fn x() {}\n");
    let mut f = common::flow(&a, config);
    f.ui.wait_for("role:treeitem[src]").unwrap();
    f.update(|a, _| a.focus_tree());
    f.settle("the tree has the keyboard on a.txt", |a| a.tree.focus && a.tree.sel.as_deref() == Some("a.txt"));
    (dir, f)
}

fn backspace(f: &mut common::Flow, n: usize) {
    for _ in 0..n {
        f.press("backspace");
    }
}

#[test]
fn a_and_shift_a_make_a_file_and_an_empty_folder() {
    let (dir, mut f) = project(None);
    f.keys("a");
    assert_eq!(f.read(|a| a.ed.cmd.as_ref().map(|c| c.text.clone())), Some("NewFile ".to_string()), "a asks for the name");
    f.keys("notes/todo.md\n");
    let todo = dir.path().join("notes/todo.md");
    let t = todo.clone();
    f.settle("the new file opens", move |a| a.ed.has_doc() && a.ed.buf().path.as_deref() == Some(t.as_path()));
    assert!(todo.is_file(), "the file and its folder are on disk");
    assert!(!f.read(|a| a.tree.focus), "the keyboard goes to the new file");
    f.ui.wait_for("role:treeitem[todo.md]").unwrap();

    f.update(|a, _| a.focus_tree());
    f.keys("A");
    assert_eq!(
        f.read(|a| a.ed.cmd.as_ref().map(|c| c.text.clone())),
        Some("Mkdir notes/".to_string()),
        "new things go in the row's folder"
    );
    f.keys("empty\n");
    f.ui.wait_for("role:treeitem[empty]").unwrap();
    assert!(dir.path().join("notes/empty").is_dir());
    assert_eq!(f.read(|a| a.tree.sel.clone()), Some("notes/empty".to_string()), "the tree's row goes to the new folder");
}

#[test]
fn r_renames_the_row_and_the_open_buffer_follows() {
    let (dir, mut f) = project(None);
    f.keys("r");
    assert_eq!(f.read(|a| a.ed.cmd.as_ref().map(|c| c.text.clone())), Some("Move a.txt".to_string()));
    backspace(&mut f, "a.txt".len());
    f.keys("b.txt\n");
    let b = dir.path().join("b.txt");
    let want = b.clone();
    f.settle("the buffer has the new name", move |a| a.ed.buf().path.as_deref() == Some(want.as_path()));
    assert!(b.is_file() && !dir.path().join("a.txt").exists());
    f.ui.wait_for("role:treeitem[b.txt]").unwrap();
    f.ui.wait_for_gone("role:treeitem[a.txt]", WAIT).unwrap();
    // A save goes to the new name.
    f.update(|a, _| a.tree.focus = false);
    f.keys("ixx\x1b:w\n");
    f.settle("the save", |a| !a.ed.buf().is_dirty());
    assert_eq!(std::fs::read_to_string(&b).unwrap(), "xxalpha\n");
    assert!(!dir.path().join("a.txt").exists(), "the save does not make the old file again");
}

#[test]
fn x_then_p_moves_a_file_into_a_folder() {
    let (dir, mut f) = project(None);
    f.keys("x");
    assert!(f.said("cut"));
    f.update(|a, _| a.tree.sel = Some("src".into()));
    f.keys("p");
    let moved = dir.path().join("src/a.txt");
    let want = moved.clone();
    f.settle("the open buffer follows the move", move |a| a.ed.buf().path.as_deref() == Some(want.as_path()));
    assert!(moved.is_file() && !dir.path().join("a.txt").exists());
    assert_eq!(f.read(|a| a.tree.sel.clone()), Some("src/a.txt".to_string()));
}

#[test]
fn move_onto_a_folder_puts_the_row_inside_it() {
    let (dir, mut f) = project(None);
    f.keys(":Move src\n");
    let moved = dir.path().join("src/a.txt");
    let want = moved.clone();
    f.settle("moved into src", move |a| a.ed.buf().path.as_deref() == Some(want.as_path()));
    assert!(moved.is_file());
}

#[test]
fn d_asks_and_only_y_deletes() {
    let (dir, mut f) = project(None);
    f.update(|a, _| a.tree.sel = Some("src".into()));
    f.keys("d");
    assert!(f.said("Delete the folder \"src\""), "{:?}", f.msg());
    f.keys("n");
    assert!(f.said("nothing deleted"));
    assert!(dir.path().join("src/lib.rs").is_file());
    f.keys("dy");
    f.ui.wait_for_gone("role:treeitem[src]", WAIT).unwrap();
    assert!(!dir.path().join("src").exists());

    // The open file: its buffer closes.
    f.update(|a, _| a.tree.sel = Some("a.txt".into()));
    f.keys("dy");
    f.settle("the buffer closes", |a| !a.ed.has_doc());
    assert!(!dir.path().join("a.txt").exists());
}

#[test]
fn delete_keeps_a_file_with_unsaved_changes_without_a_bang() {
    let dir = common::scratch();
    let a = common::write(dir.path(), "a.txt", "alpha\n");
    let mut f = common::flow(&a, None);
    f.keys("ix\x1b:Delete\n");
    assert!(f.said("E37"), "{:?}", f.msg());
    assert!(a.is_file());
    f.keys(":Delete!\n");
    f.settle("the buffer closes", |a| !a.ed.has_doc());
    assert!(!a.exists());
}

#[test]
fn file_keys_work_in_the_classic_keymap() {
    let (dir, mut f) = project(Some(&common::keymap_config("classic")));
    f.press("f2");
    assert_eq!(f.read(|a| a.ed.cmd.as_ref().map(|c| c.text.clone())), Some("Move a.txt".to_string()), "F2 renames");
    backspace(&mut f, "a.txt".len());
    f.keys("c.txt\n");
    let c = dir.path().join("c.txt");
    f.settle("renamed", move |a| a.ed.buf().path.as_deref() == Some(c.as_path()));
    f.update(|a, _| a.focus_tree());
    f.press("delete");
    f.keys("y");
    f.settle("the buffer closes", |a| !a.ed.has_doc());
    assert!(!dir.path().join("c.txt").exists());
}

#[test]
fn new_file_never_writes_over_a_file() {
    let (dir, mut f) = project(None);
    f.keys(":NewFile src/lib.rs\n");
    f.settle("the error", |a| a.ed.msg.as_ref().is_some_and(|m| m.text.contains("already exists")));
    assert_eq!(std::fs::read_to_string(dir.path().join("src/lib.rs")).unwrap(), "pub fn x() {}\n");
}
