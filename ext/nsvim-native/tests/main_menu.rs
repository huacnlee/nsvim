//! The menu button of the title bar, through the real window: on Windows and Linux it shows the
//! menus of the application menu in a row, drawn as a menu bar: a menu opens its items, the
//! pointer over another name opens that menu, a submenu opens beside its item, and the picked
//! item runs. macOS shows the menu in its menu bar.

mod common;

use std::time::Duration;

use nsvim_native::exports::main_menu_shown;

const WAIT: Duration = Duration::from_secs(5);

/// The width of the view `sel`.
fn width(f: &common::Flow, sel: &str) -> f64 {
    f.ui.screen().layout(sel).unwrap().map(|l| l.bounds.size.width.0 as f64).unwrap_or(0.0)
}

/// The left and right edges of the view `sel`, in the window.
fn edges(f: &common::Flow, sel: &str) -> (f64, f64) {
    let b = f.ui.screen().layout(sel).unwrap().expect(sel).bounds;
    (b.origin.x.0 as f64, (b.origin.x.0 + b.size.width.0) as f64)
}

/// An item of the open menu: `id:menu-item-<label>`.
fn item(label: &str) -> String {
    format!("id:menu-item-{label}")
}

#[test]
fn the_menu_button_shows_the_menus_in_a_row() {
    if !main_menu_shown() {
        return;
    }
    let dir = common::scratch();
    let a = common::write(dir.path(), "a.txt", "alpha\n");
    let mut f = common::flow(&a, None);
    f.update(|a, _| a.tree.show = false);
    f.ui.wait_for_gone("id:tree", WAIT).unwrap();
    assert_eq!(f.ui.screen().count("id:main-menu-bar").unwrap(), 0, "the row shows only after a press");
    f.ui.press("id:main-menu").unwrap();
    f.ui.wait_for("id:main-menu-bar").unwrap();
    for name in ["NsVim", "File", "Edit", "View", "Go", "Help"] {
        f.ui.wait_for(&format!("id:menu-{name}")).unwrap();
    }
    // The names sit side by side, from left to right.
    let x = |sel: &str| f.ui.screen().layout(sel).unwrap().map(|l| l.bounds.origin.x.0 as f64).unwrap();
    assert!(x("id:menu-NsVim") < x("id:menu-File") && x("id:menu-File") < x("id:menu-Help"), "the menus are in a row");

    // A press opens one menu. While it is open, the pointer over another name opens that one in
    // its place, as a menu bar does. Each menu holds only its own items.
    f.ui.press("id:menu-NsVim").unwrap();
    for (name, want, not) in [
        ("NsVim", vec!["Check for Updates…", "Settings…", "App Setup…", "Quit"], "New File"),
        ("File", vec!["New File", "Open Folder…", "Save", "Open Recent"], "Undo"),
        ("Edit", vec!["Undo", "Find in Project…"], "Save"),
        ("View", vec!["Command Palette…", "File Tree"], "Undo"),
        ("Go", vec!["Go to Definition"], "Undo"),
        ("Help", vec!["Keyboard Shortcuts…"], "Undo"),
    ] {
        f.ui.hover(&format!("id:menu-{name}")).unwrap();
        for w in want {
            f.ui.wait_for(&item(w)).unwrap_or_else(|e| panic!("{w} in {name}: {e}"));
        }
        assert_eq!(f.ui.screen().count(&item(not)).unwrap(), 0, "{not} is not in {name}");
        assert_eq!(f.ui.screen().count("id:menu-item-Hide NsVim").unwrap(), 0, "the Mac's window items stay on the Mac");
    }

    // Open Recent opens beside File's items, clear of them.
    f.ui.hover("id:menu-File").unwrap();
    f.ui.hover(&item("Open Recent")).unwrap();
    f.ui.wait_for("id:menu-sub-item-0").unwrap();
    let (_, parent_right) = edges(&f, &item("Open Recent"));
    let (sub_left, _) = edges(&f, "id:menu-sub-item-0");
    assert!(sub_left >= parent_right, "the submenu ({sub_left}) does not cover its menu (ends at {parent_right})");

    // A press on the open menu's name closes it, and the row stays.
    f.ui.press("id:menu-File").unwrap();
    f.ui.wait_for_gone(&item("New File"), WAIT).unwrap();
    f.ui.wait_for("id:main-menu-bar").unwrap();

    // A picked item runs as the same item of the menu bar does. An item with a check mark keeps
    // its menu open, and the row stays after any pick: the menu button hides it.
    f.ui.press("id:menu-View").unwrap();
    f.ui.press(&item("File Tree")).unwrap();
    f.ui.wait_for("id:tree").unwrap();
    assert!(f.read(|a| a.tree.show));
    f.ui.wait_for(&item("File Tree")).unwrap();
    f.ui.press(&item("File Tree")).unwrap();
    f.ui.wait_for_gone("id:tree", WAIT).unwrap();
    f.ui.press("id:menu-View").unwrap();
    f.ui.wait_for_gone(&item("File Tree"), WAIT).unwrap();
    f.ui.wait_for("id:main-menu-bar").unwrap();
    f.ui.press("id:main-menu").unwrap();
    f.ui.wait_for_gone("id:main-menu-bar", WAIT).unwrap();

    // With the tree, the button sits in the folder over the tree, which stays as wide as the tree.
    f.update(|a, _| a.tree.show = true);
    f.ui.wait_for("id:project >> id:main-menu").unwrap();
    assert!((width(&f, "id:project") - width(&f, "id:tree")).abs() < 1.0, "the folder is as wide as the tree");

    // There the row takes the place of the path, and a second press on the button hides it.
    f.ui.press("id:main-menu").unwrap();
    f.ui.wait_for("id:project >> id:menu-Help").unwrap();
    f.ui.press("id:main-menu").unwrap();
    f.ui.wait_for_gone("id:main-menu-bar", WAIT).unwrap();
}

#[test]
fn the_main_menu_opens_the_command_palette() {
    if !main_menu_shown() {
        return;
    }
    let dir = common::scratch();
    let a = common::write(dir.path(), "a.txt", "alpha\n");
    let mut f = common::flow(&a, None);
    f.ui.press("id:main-menu").unwrap();
    f.ui.press("id:menu-View").unwrap();
    f.ui.press(&item("Command Palette…")).unwrap();
    f.ui.wait_for("id:picker-input").unwrap();
    // The menu closed with the pick; the row stays.
    f.ui.wait_for_gone(&item("Command Palette…"), WAIT).unwrap();
    f.ui.wait_for("id:main-menu-bar").unwrap();
}
