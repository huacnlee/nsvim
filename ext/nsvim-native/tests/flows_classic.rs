//! The classic keymap, driven through the real app: typing, its shortcuts, multiple cursors,
//! and the window shortcuts that pass through the tree. Settings keeps them out.

mod common;

use common::{SRC, rust_flow};
use editor::buffer::Pos;
use nsvim_native::app::{Overlay, PickKind};

fn picking(a: &nsvim_native::app::App, kind: PickKind) -> bool {
    matches!(&a.overlay, Overlay::Pick(p) if p.kind == kind)
}

#[test]
fn classic_shortcuts_open_the_palette_and_undo() {
    let (_dir, mut f) = rust_flow("classic", SRC);
    f.keys("ab");
    assert!(f.text().starts_with("abfn"));
    f.press("ctrl+z");
    assert_eq!(f.text(), SRC);
    f.press("ctrl+shift+p");
    assert!(f.check(|a| picking(a, PickKind::Commands)), "Ctrl+Shift+P opens the command palette");
}

#[test]
fn ctrl_d_adds_cursors_and_typing_edits_at_each() {
    let src = "let a = 1;\nlet b = a;\nlet c = a;\n";
    let (_dir, mut f) = rust_flow("classic", src);
    // Put the cursor on the `a` of line 2's `= a`, then select it and the next match.
    for k in ["down", "end", "left", "left"] {
        f.press(k);
    }
    f.press("ctrl+d");
    f.press("ctrl+d");
    assert_eq!(f.read(|a| a.ed.doc().extra.len()), 1, "{:?}", f.read(|a| format!("{:?}", a.ed.doc().extra)));
    f.keys("z");
    let t = f.text();
    assert!(t.contains("let b = z;") && t.contains("let c = z;"), "{t}");
    f.press("ctrl+z");
    assert_eq!(f.text(), src);
}

#[test]
fn the_tree_keeps_its_keys_and_passes_ctrl_shortcuts_on() {
    let dir = common::scratch();
    common::write(dir.path(), "b.rs", "fn b() {}\n");
    let file = common::write(dir.path(), "a.rs", "fn a() {}\n");
    let mut f = common::flow(&file, Some(&common::keymap_config("classic")));
    f.settle("the project index", |a| a.files.len() == 2);
    f.update(|a, _| {
        a.tree.show = true;
        a.tree.focus = true;
    });
    let before = f.text();
    f.keys("x");
    assert_eq!(f.text(), before, "a plain key the tree does not use never edits the file");
    assert!(f.read(|a| a.tree.focus));
    f.press("ctrl+p");
    assert!(f.check(|a| picking(a, PickKind::Files)), "Ctrl P finds a file while the tree has focus");
}

#[test]
fn settings_keeps_the_window_shortcuts_out_and_ctrl_w_closes_it() {
    let dir = common::scratch();
    let two = common::write(dir.path(), "two.rs", "two\n");
    let main = common::write(dir.path(), "main.rs", SRC);
    let mut f = common::flow_files(&[&main, &two], Some(&common::keymap_config("classic")));
    f.settle("both files open", |a| a.ed.docs.len() == 2);
    f.update(|a, _| a.open_settings());
    let active = f.read(|a| a.ed.active);
    // Settings covers the window, so no shortcut opens a panel or changes the tab under it.
    f.press("ctrl+p");
    f.press("ctrl+tab");
    assert!(f.read(move |a| a.on_settings && matches!(a.overlay, Overlay::None) && a.ed.active == active));
    f.press("ctrl+w");
    assert!(f.read(|a| !a.on_settings && a.settings.is_none()), "Ctrl W closes Settings");
    assert_eq!(f.read(|a| a.ed.docs.len()), 2, "Ctrl W closes no file");

    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.update(|a, _| a.open_settings());
    f.press("ctrl+w");
    assert!(f.read(|a| !a.on_settings && a.settings.is_none()), "Ctrl W closes Settings in the Neovim keymap too");
}

#[test]
fn alt_click_adds_a_cursor_in_the_classic_keymap() {
    let (_dir, mut f) = rust_flow("classic", SRC);
    let (cw, rh, gutter) = f.read(|a| (a.m.cw, a.m.row_h, a.m.gutter));
    let (px, py) = (gutter + 4.0 * cw + cw / 4.0, 2.0 * rh + rh / 2.0);
    f.ui.hold_keys("alt", true).unwrap();
    f.ui.press_at("id:text-area", common::point(px, py)).unwrap();
    f.ui.hold_keys("alt", false).unwrap();
    let extra = f.read(|a| a.ed.doc().extra.iter().map(|c| c.pos).collect::<Vec<_>>());
    assert_eq!(extra, vec![Pos::new(2, 4)]);
}
