//! The mouse in the real app: clicks, drags, the context menu, tabs and the tree's wheel.

mod common;

use common::{SRC, point, rust_flow};
use editor::buffer::Pos;
use editor::editor::Mode;
use nsh_testing::DragOptions;

#[test]
fn a_click_places_the_cursor_and_a_drag_selects_in_visual_mode() {
    let (_dir, mut f) = rust_flow("neovim", &(0..60).map(|i| format!("line number {i}\n")).collect::<String>());
    let (cw, rh, gutter) = f.read(|a| (a.m.cw, a.m.row_h, a.m.gutter));
    let at = |line: f32, col: f32| point(gutter + col * cw + cw / 2.0, line * rh + rh / 2.0);
    f.ui.press_at("id:text-area", at(2.0, 8.0)).unwrap();
    assert_eq!(f.read(|a| a.ed.cursor()), Pos::new(2, 8));
    // Headless drags from the middle of the view.
    let [_, _, w, h] = f.read(|a| a.m.text);
    let from = f.read(move |a| a.pos_at(w / 2.0, h / 2.0)).unwrap();
    let by = point(-3.0 * cw, 2.0 * rh);
    let to = f.read(move |a| a.pos_at(w / 2.0 - 3.0 * cw, h / 2.0 + 2.0 * rh)).unwrap();
    // Headless counts a press within 500 ms of the last one on the same view as a double click,
    // and a double click selects a word. A user who clicks and then drags pauses longer.
    f.ui.wait(std::time::Duration::from_millis(600)).unwrap();
    f.ui.drag("id:text-area", by, DragOptions::default()).unwrap();
    assert!(matches!(f.read(|a| a.ed.mode), Mode::Visual(_)), "{:?}", f.read(|a| a.ed.mode));
    assert_eq!(f.read(|a| a.ed.doc().view.anchor), Some(from));
    assert_eq!(f.read(|a| a.ed.cursor()), to);
}

/// A drag near the bottom keeps the text under the pointer still. Scrolloff used to scroll at
/// every move there, so the selection ran away to the end of the file.
#[test]
fn a_drag_near_the_bottom_selects_what_is_under_the_pointer() {
    let src: String = (1..=300).map(|i| format!("line {i}\n")).collect();
    let (_dir, mut f) = rust_flow("neovim", &src);
    let (cw, rh, [_, _, _, h]) = f.read(|a| (a.m.cw, a.m.row_h, a.m.text));
    let row = (h / rh).floor() - 3.0;
    let middle = h / 2.0;
    let top = f.read(|a| a.ed.doc().view.top);
    // From the middle of the text area down to three rows above its bottom, and to the right.
    let by = point(6.0 * cw, row * rh + rh / 2.0 - middle);
    f.ui.drag("id:text-area", by, DragOptions::default()).unwrap();
    assert_eq!(f.read(|a| a.ed.doc().view.top), top, "the text held still under the pointer");
    assert_eq!(f.read(|a| a.ed.cursor().line), top + row as usize, "the cursor is on the row under the pointer");
}

#[test]
fn right_click_opens_the_menu_and_copy_takes_the_selection() {
    let src: String = (0..100).map(|i| format!("row {i}\n")).collect();
    let (_dir, mut f) = rust_flow("neovim", &src);
    f.update(|a, _| a.cfg.mouse.right_click = true);
    // Select every line, then right-click in the middle of the text, inside the selection.
    f.keys("ggVG");
    f.ui.right_click("id:text-area").unwrap();
    assert!(f.ui.menu().is_some(), "the platform's context menu shows");
    f.ui.choose_menu_item("Copy    y").unwrap();
    assert_eq!(f.read(|a| a.ed.regs.unnamed.text.clone()), src);
    assert_eq!(f.read(|a| a.ed.mode), Mode::Normal);
}

#[test]
fn a_middle_click_closes_a_clean_tab() {
    let dir = common::scratch();
    let two = common::write(dir.path(), "two.rs", "two\n");
    let main = common::write(dir.path(), "main.rs", SRC);
    let mut f = common::flow_files(&[&main, &two], None);
    f.settle("both files open", |a| a.ed.docs.len() == 2);
    f.ui.wait_for("role:tab >> nth=1").unwrap();
    assert_eq!(f.ui.screen().count("role:tab").unwrap(), 2);
    f.middle_click("role:tab >> nth=1", point(20.0, 10.0));
    f.settle("the tab closes", |a| a.ed.docs.len() == 1);
}

/// The wheel over the tree scrolls it a row per 22 points, adds up a trackpad's small steps
/// and leaves the keyboard row alone. Keys bring the keyboard row back on screen.
#[test]
fn the_wheel_scrolls_the_tree_and_keys_bring_the_row_back() {
    // The open file is outside the project, so `SPC e` reveals no row and
    // the keyboard row starts at the first one.
    let dir = common::scratch();
    let project = dir.path().join("project");
    for i in 0..200 {
        common::write(&project, &format!("f{i:03}.txt"), "x\n");
    }
    let file = common::write(dir.path(), "outside.txt", "x\n");
    let mut f = common::flow_with(nsh_app_editor::Launch {
        project: Some(project.clone()),
        config: Some(common::CONFIG.into()),
        files: vec![file.clone()],
        first_launch: false,
    });
    f.wait_open(&file);
    f.settle("the project index", |a| a.files.len() == 200);
    let top = |f: &common::Flow| f.read(|a| a.tree_top(&a.tree.index.rows(&a.tree.open)));
    let sel = |f: &common::Flow| f.read(|a| a.tree.sel.clone().unwrap_or_default());
    let visible = f.read(|a| a.tree_visible());
    let max = 200 - visible;
    // Positive points scroll down, as NSH reports a wheel.
    let wheel = |f: &mut common::Flow, dy: f32| f.ui.wheel("id:tree", point(0.0, dy)).unwrap();
    wheel(&mut f, 220.0);
    assert_eq!(top(&f), 10, "ten rows down");
    assert_eq!(f.read(|a| a.tree.sel.clone()), None, "the wheel leaves the keyboard row");
    for _ in 0..11 {
        wheel(&mut f, 2.0);
    }
    assert_eq!(top(&f), 11, "eleven 2-point trackpad steps make one row");
    wheel(&mut f, -22.0 * 500.0);
    assert_eq!(top(&f), 0, "it stops at the first row");
    wheel(&mut f, 22.0 * 500.0);
    assert_eq!(top(&f), max, "it stops with the last row at the bottom");

    f.keys(" e");
    assert!(f.read(|a| a.tree.focus));
    f.keys("j");
    assert_eq!(sel(&f), "f001.txt");
    assert_eq!(top(&f), 1, "j scrolls back to the keyboard row");
    f.press("ctrl+d");
    assert_eq!(sel(&f), format!("f{:03}.txt", 1 + visible / 2), "Ctrl d moves half a screen");
    f.press("ctrl+u");
    assert_eq!(sel(&f), "f001.txt", "Ctrl u moves back");
    f.press("pagedown");
    assert_eq!(sel(&f), format!("f{:03}.txt", visible), "PageDown moves a screen");
    f.keys("G");
    assert_eq!((sel(&f), top(&f)), ("f199.txt".to_string(), max), "G goes to the last row and shows it");
    wheel(&mut f, -22.0 * 5.0);
    assert_eq!(top(&f), max - 5, "the wheel scrolls while the tree has the focus");
    f.keys("k");
    assert_eq!((sel(&f), top(&f)), ("f198.txt".to_string(), 198 + 1 - visible), "k shows the row at the bottom edge");

    f.update(|a, _| a.cfg.mouse.scroll = false);
    let before = top(&f);
    wheel(&mut f, 22.0 * 5.0);
    assert_eq!(top(&f), before, "mouse.scroll off leaves the tree still");
}

#[test]
fn the_wheel_over_the_text_scrolls_it() {
    let src: String = (1..=300).map(|i| format!("line {i}\n")).collect();
    let (_dir, mut f) = rust_flow("neovim", &src);
    let rh = f.read(|a| a.m.row_h);
    f.ui.wheel("id:text-area", point(0.0, 10.0 * rh)).unwrap();
    f.settle("the text scrolls down", |a| a.ed.doc().view.top > 0);
}
