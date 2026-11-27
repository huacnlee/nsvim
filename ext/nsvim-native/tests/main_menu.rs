//! The menu button of the title bar, through the real window: on Windows and Linux it shows the
//! menus of the application menu in a row, a menu opens its items, and the picked item runs.
//! macOS shows the menu in its menu bar.

mod common;

use std::time::Duration;

use nsvim_native::exports::main_menu_shown;

const WAIT: Duration = Duration::from_secs(5);

/// The width of the view `sel`.
fn width(f: &common::Flow, sel: &str) -> f64 {
    f.ui.screen().layout(sel).unwrap().map(|l| l.bounds.size.width.0 as f64).unwrap_or(0.0)
}

/// The labels of the menu that shows, each with the path of its submenus.
fn menu_labels(f: &mut common::Flow) -> Vec<String> {
    // The menu opens on the session's task, a moment after the press.
    for _ in 0..50 {
        if f.ui.menu().is_some() {
            break;
        }
        f.ui.wait(Duration::from_millis(100)).unwrap();
    }
    f.ui.menu().expect("a menu shows").iter().map(|e| e.label.clone()).collect()
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

    // Each menu opens only its own items.
    for (name, want, not) in [
        ("NsVim", vec!["Check for Updates…", "Settings…", "App Setup…", "Quit"], "New File"),
        ("File", vec!["New File", "Open Folder…", "Save"], "Undo"),
        ("Edit", vec!["Undo", "Find in Project…"], "Save"),
        ("View", vec!["Command Palette…", "Toggle File Tree"], "Undo"),
        ("Go", vec!["Go to Definition"], "Undo"),
        ("Help", vec!["Keyboard Shortcuts…"], "Undo"),
    ] {
        f.ui.press(&format!("id:menu-{name}")).unwrap();
        let labels = menu_labels(&mut f);
        for w in want {
            assert!(labels.iter().any(|l| l == w), "{w} in {name}: {labels:?}");
        }
        assert!(!labels.iter().any(|l| l == not), "{not} is not in {name}: {labels:?}");
        assert!(!labels.iter().any(|l| l.contains("Hide")), "the Mac's window items stay on the Mac: {labels:?}");
        if name == "File" {
            assert!(labels.iter().any(|l| l.starts_with("Open Recent > ")), "Open Recent in {labels:?}");
        }
        f.ui.close_menu().unwrap();
    }
    // A menu that closes with no pick keeps the row.
    f.ui.wait_for("id:main-menu-bar").unwrap();

    // A picked item runs as the same item of the menu bar does, and the row hides.
    f.ui.press("id:menu-View").unwrap();
    menu_labels(&mut f);
    f.ui.choose_menu_item("Toggle File Tree").unwrap();
    f.ui.wait_for("id:tree").unwrap();
    assert!(f.read(|a| a.tree.show));
    f.ui.wait_for_gone("id:main-menu-bar", WAIT).unwrap();
    // With the tree, the button sits in the folder over the tree, which stays as wide as the tree.
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
    menu_labels(&mut f);
    f.ui.choose_menu_item("Command Palette…").unwrap();
    f.ui.wait_for("id:picker-input").unwrap();
}
