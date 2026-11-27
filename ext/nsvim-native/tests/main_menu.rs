//! The menu button of the title bar, through the real window: on Windows and Linux it opens
//! the application menu, and the picked item runs. macOS shows the menu in its menu bar.

mod common;

use std::time::Duration;

use nsvim_native::exports::main_menu_shown;

const WAIT: Duration = Duration::from_secs(5);

/// The width of the view `sel`.
fn width(f: &common::Flow, sel: &str) -> f64 {
    f.ui.screen().layout(sel).unwrap().map(|l| l.bounds.size.width.0 as f64).unwrap_or(0.0)
}

#[test]
fn the_menu_button_opens_every_menu_of_the_application_menu() {
    if !main_menu_shown() {
        return;
    }
    let dir = common::scratch();
    let a = common::write(dir.path(), "a.txt", "alpha\n");
    let mut f = common::flow(&a, None);
    f.update(|a, _| a.tree.show = false);
    f.ui.wait_for_gone("id:tree", WAIT).unwrap();
    f.ui.press("id:main-menu").unwrap();
    let menu = f.ui.menu().expect("the application menu shows");
    let labels: Vec<String> = menu.iter().map(|e| e.label.clone()).collect();
    for want in [
        "NsVim > Check for Updates…",
        "NsVim > Settings…",
        "NsVim > App Setup…",
        "NsVim > Quit",
        "File > New File",
        "File > Open Folder…",
        "File > Save",
        "Edit > Undo",
        "Edit > Find in Project…",
        "View > Command Palette…",
        "View > Toggle File Tree",
        "Go > Go to Definition",
        "Help > Keyboard Shortcuts…",
    ] {
        assert!(labels.iter().any(|l| l == want), "{want} in {labels:?}");
    }
    assert!(labels.iter().any(|l| l.starts_with("File > Open Recent > ")), "Open Recent in {labels:?}");
    assert!(!labels.iter().any(|l| l.contains("Hide")), "the Mac's window items stay on the Mac: {labels:?}");

    // A picked item runs as the same item of the menu bar does.
    f.ui.choose_menu_item("View > Toggle File Tree").unwrap();
    f.ui.wait_for("id:tree").unwrap();
    assert!(f.read(|a| a.tree.show));
    // With the tree, the button sits in the folder over the tree, which stays as wide as the tree.
    f.ui.wait_for("id:project >> id:main-menu").unwrap();
    assert!((width(&f, "id:project") - width(&f, "id:tree")).abs() < 1.0, "the folder is as wide as the tree");
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
    f.ui.choose_menu_item("View > Command Palette…").unwrap();
    f.ui.wait_for("id:picker-input").unwrap();
}
