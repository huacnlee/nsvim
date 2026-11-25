//! The file tree in the real app (`ui/tree.tsx`): presses on rows after a scroll, the edge that
//! changes the width, and the limits of that width.

mod common;

use std::time::Duration;

use common::point;
use nsh_testing::DragOptions;

const WAIT: Duration = Duration::from_secs(5);

/// A project of 80 files, with the first open, probed.
fn eighty() -> (tempfile::TempDir, common::Flow) {
    let dir = common::scratch();
    let files: Vec<_> = (0..80).map(|i| common::write(dir.path(), &format!("file{i:02}.txt"), &format!("text {i}\n"))).collect();
    let mut f = common::flow(&files[0], None);
    f.settle("the project index", |a| a.files.len() == 80);
    f.ui.wait_for("role:treeitem[file00.txt]").unwrap();
    (dir, f)
}

/// Rows keep their place on screen while a scroll changes their files. A press after a scroll
/// must open the file that the row shows now.
#[test]
fn a_press_after_a_scroll_opens_the_file_under_it() {
    let (_dir, mut f) = eighty();
    let top = |f: &common::Flow| f.read(|a| a.tree_top(&a.tree.index.rows(&a.tree.open)));
    f.ui.wheel("id:tree", point(0.0, 22.0 * 10.0)).unwrap();
    f.settle("ten rows down", move |a| a.tree_top(&a.tree.index.rows(&a.tree.open)) == 10);
    f.ui.wait_for("role:treeitem[file15.txt]").unwrap();
    f.ui.wait_for_gone("role:treeitem[file00.txt]", WAIT).unwrap();
    f.ui.press("role:treeitem[file15.txt]").unwrap();
    f.settle("file15 opens", |a| a.ed.buf().text() == "text 15\n");
    assert_eq!(top(&f), 10, "a press on a row on screen leaves the scroll where it is");
    f.ui.wait_until("the tree marks the open file", WAIT, |s| s.selected("role:treeitem[file15.txt]").unwrap_or(false)).unwrap();
    assert!(!f.ui.screen().selected("role:treeitem[file00.txt]").unwrap_or(false));
    // The press gives the keys back to the editor.
    f.keys("i");
    f.settle("insert mode", |a| a.ed.mode == editor::editor::Mode::Insert);
}

/// A press on a folder opens it and leaves the keyboard off the tree.
#[test]
fn a_press_on_a_folder_takes_the_keyboard_off_the_tree() {
    let dir = common::scratch();
    let a = common::write(dir.path(), "a.txt", "alpha\n");
    common::write(dir.path(), "src/lib.rs", "pub fn x() {}\n");
    let mut f = common::flow(&a, None);
    f.ui.wait_for("role:treeitem[src]").unwrap();
    f.keys(" e");
    f.settle("the tree has the keyboard", |a| a.tree.focus);
    f.ui.press("role:treeitem[src]").unwrap();
    f.ui.wait_for("role:treeitem[lib.rs]").unwrap();
    assert!(!f.read(|a| a.tree.focus), "the press takes the keyboard off the tree");
    assert_eq!(f.ui.screen().accessibility_level("role:treeitem[lib.rs]").unwrap(), Some(2), "a row inside a folder is one level down");
}

/// The edge keeps the width in the config on release, and stops at 160 and 640 points.
#[test]
fn a_drag_on_the_edge_keeps_the_width_between_its_limits() {
    let (_dir, mut f) = eighty();
    let width = |f: &common::Flow| f.read(|a| (a.tree.width, a.cfg.ui.tree_width));
    f.ui.drag("id:tree-edge", point(60.0, 0.0), DragOptions::default()).unwrap();
    f.settle("the tree grows", |a| a.cfg.ui.tree_width == 320.0);
    assert_eq!(width(&f), (320.0, 320.0));
    f.ui.wait_until("the view follows", WAIT, |s| {
        s.layout("id:tree").unwrap().is_some_and(|l| (l.bounds.size.width.0 - 320.0).abs() < 1.0)
    })
    .unwrap();
    f.ui.drag("id:tree-edge", point(-1000.0, 0.0), DragOptions::default()).unwrap();
    f.settle("the narrowest", |a| a.cfg.ui.tree_width == 160.0);
    f.ui.wait_until("the edge follows", WAIT, |s| {
        s.layout("id:tree").unwrap().is_some_and(|l| (l.bounds.size.width.0 - 160.0).abs() < 1.0)
    })
    .unwrap();
    f.ui.drag("id:tree-edge", point(2000.0, 0.0), DragOptions::default()).unwrap();
    f.settle("the widest", |a| a.cfg.ui.tree_width == 640.0);
    // With the mouse off in the config, the edge does nothing.
    f.update(|a, _| a.cfg.mouse.enabled = false);
    f.ui.drag("id:tree-edge", point(-100.0, 0.0), DragOptions::default()).unwrap();
    f.ui.wait(Duration::from_millis(200)).unwrap();
    assert_eq!(width(&f), (640.0, 640.0));
}
