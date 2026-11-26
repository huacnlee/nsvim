//! The tab menu without a window: what each close command closes, which tab shows after it,
//! the tabs with unsaved changes that stay open, and the items that cannot run.

use std::path::PathBuf;

use editor::buffer::{Buffer, Pos};
use editor::editor::{Doc, Keymap, Mode};

use super::App;
use super::fx::{Effect, Tx};
use super::tab_menu::{CloseTabs, TAB_ITEMS};

/// An app in `/project` with one tab for each name. Tab `active` shows. The tabs in `dirty`
/// have unsaved changes.
fn app(names: &[&str], active: usize, dirty: &[usize]) -> App {
    let cfg = toml::from_str("[ipc]\nenabled = false\n").unwrap();
    let mut a = App::new(cfg, PathBuf::from("/project"), false, None);
    a.screen = super::Screen::Editor;
    a.ed.keymap = Keymap::Vim;
    a.ed.mode = Mode::Normal;
    for (i, name) in names.iter().enumerate() {
        let mut b = Buffer::from_str("text\n");
        b.path = Some(PathBuf::from(format!("/project/src/{name}")));
        if dirty.contains(&i) {
            b.insert(Pos::new(0, 0), "x");
        }
        a.ed.docs.push(Doc::new(b));
    }
    a.ed.active = active;
    a
}

/// The names of the open tabs, and the name of the tab that shows.
fn tabs(a: &App) -> (Vec<String>, String) {
    (a.ed.docs.iter().map(|d| d.buf.name()).collect(), a.ed.docs[a.ed.active].buf.name())
}

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn close_others_keeps_only_the_tab_and_shows_it() {
    let mut a = app(&["a", "b", "c", "d"], 0, &[]);
    a.close_tabs(2, CloseTabs::Others, &mut Tx::new());
    assert_eq!(tabs(&a), (names(&["c"]), "c".into()));
}

#[test]
fn close_left_and_close_right_keep_the_tab_that_shows() {
    let mut a = app(&["a", "b", "c", "d", "e"], 3, &[]);
    a.close_tabs(2, CloseTabs::Left, &mut Tx::new());
    assert_eq!(tabs(&a), (names(&["c", "d", "e"]), "d".into()), "the tab that showed still shows");
    a.close_tabs(0, CloseTabs::Right, &mut Tx::new());
    assert_eq!(tabs(&a), (names(&["c"]), "c".into()), "the tab that showed closed: the clicked tab shows");
}

#[test]
fn close_clean_and_close_all_keep_the_tabs_with_unsaved_changes() {
    let mut a = app(&["a", "b", "c", "d"], 0, &[1, 3]);
    a.close_tabs(0, CloseTabs::Clean, &mut Tx::new());
    assert_eq!(tabs(&a).0, names(&["b", "d"]));
    assert!(a.ed.msg.is_none(), "close clean does not aim at the modified tabs");
    let mut a = app(&["a", "b", "c", "d"], 2, &[1, 3]);
    let mut tx = Tx::new();
    a.close_tabs(2, CloseTabs::All, &mut tx);
    assert_eq!(tabs(&a).0, names(&["b", "d"]), "the clean tabs close at once");
    let (dialog, then) = take_confirm(&mut tx);
    assert_eq!(dialog.title, "Save changes to 2 files?");
    assert!(dialog.description.contains("\"b\" and \"d\" have unsaved changes"), "{}", dialog.description);
    then(&mut a, Ok(Some(super::unsaved::DISCARD.into())), &mut Tx::new());
    assert!(a.ed.docs.is_empty(), "don't save closes them without a write");
}

/// The confirm dialog the app asked for, and what runs with its answer.
fn take_confirm(tx: &mut Tx) -> (super::fx::Confirm, super::fx::Then<Result<Option<String>, String>>) {
    let i = tx.effects.iter().position(|e| matches!(e, Effect::Confirm(..))).expect("a dialog");
    match tx.effects.remove(i) {
        Effect::Confirm(d, then) => (d, then),
        _ => unreachable!(),
    }
}

#[test]
fn the_close_button_on_a_dirty_tab_asks_and_cancel_keeps_it() {
    let mut a = app(&["a", "b"], 0, &[1]);
    let mut tx = Tx::new();
    a.close_tab(1, &mut tx);
    assert_eq!(tabs(&a), (names(&["a", "b"]), "b".into()), "the dirty tab shows while the dialog asks");
    let (dialog, then) = take_confirm(&mut tx);
    assert_eq!(dialog.title, "Save changes to \"b\"?");
    let ids: Vec<&str> = dialog.buttons.iter().map(|b| b.0.as_str()).collect();
    assert_eq!(ids, ["save", "discard", "cancel"]);
    then(&mut a, Ok(None), &mut Tx::new());
    assert_eq!(tabs(&a).0, names(&["a", "b"]), "escape keeps the tab");
    a.close_tab(1, &mut tx);
    let (_, then) = take_confirm(&mut tx);
    then(&mut a, Ok(Some(super::unsaved::DISCARD.into())), &mut Tx::new());
    assert_eq!(tabs(&a), (names(&["a"]), "a".into()));
}

#[test]
fn the_dialog_finds_its_tab_after_the_tabs_move() {
    let mut a = app(&["a", "b", "c"], 0, &[2]);
    let mut tx = Tx::new();
    a.close_tab(2, &mut tx);
    let (_, then) = take_confirm(&mut tx);
    a.close_tab(0, &mut Tx::new());
    then(&mut a, Ok(Some(super::unsaved::DISCARD.into())), &mut Tx::new());
    assert_eq!(tabs(&a).0, names(&["b"]), "\"c\" closes, wherever it is now");
}

#[test]
fn close_others_on_a_dirty_tab_closes_only_the_clean_ones() {
    let mut a = app(&["a", "b", "c"], 0, &[0]);
    a.close_tabs(0, CloseTabs::Others, &mut Tx::new());
    assert_eq!(tabs(&a), (names(&["a"]), "a".into()));
    assert!(a.ed.msg.is_none(), "every tab it closes was clean");
}

#[test]
fn items_with_nothing_to_close_cannot_run() {
    let a = app(&["a", "b", "c"], 0, &[]);
    assert!(!a.tab_item_enabled(0, "close-left"), "no tab on the left of the first");
    assert!(a.tab_item_enabled(0, "close-right"));
    assert!(!a.tab_item_enabled(2, "close-right"), "no tab on the right of the last");
    let one = app(&["a"], 0, &[0]);
    assert!(!one.tab_item_enabled(0, "close-others"), "one tab has no others");
    assert!(!one.tab_item_enabled(0, "close-clean"), "no tab is clean");
    assert!(one.tab_item_enabled(0, "close-all"));
}

#[test]
fn a_disabled_item_does_nothing() {
    let mut a = app(&["a", "b"], 1, &[]);
    a.tab_menu_pick(0, "close-left", &mut Tx::new());
    assert_eq!(tabs(&a).0, names(&["a", "b"]));
}

#[test]
fn the_menu_shows_every_item_with_the_palette_keys() {
    let mut a = app(&["a", "b", "c"], 1, &[]);
    let mut tx = Tx::new();
    a.tab_menu(0, (10.0, 20.0), &mut tx);
    let menu = tx
        .effects
        .into_iter()
        .find_map(|e| match e {
            Effect::PopupMenu { menu, .. } => Some(menu),
            _ => None,
        })
        .expect("a right-click on a tab opens the system's menu");
    assert_eq!(menu.items.len(), TAB_ITEMS.len());
    let item = |id: &str| menu.items.iter().flatten().find(|i| i.id == id).unwrap().clone();
    assert_eq!(item("close-others").title, "Close others    SPC t o");
    assert_eq!(item("close-all").title, "Close all    SPC t a");
    assert!(!item("close-left").enabled, "tab 0 has no tab on its left");
    assert!(item("close-right").enabled);
}

#[test]
fn copy_path_and_copy_relative_path_copy_the_tab_path() {
    let mut a = app(&["a.rs", "b.rs"], 0, &[]);
    let copied = |a: &mut App, id: &str| {
        let mut tx = Tx::new();
        a.tab_menu_pick(1, id, &mut tx);
        tx.effects.into_iter().find_map(|e| match e {
            Effect::Copy(t) => Some(t),
            _ => None,
        })
    };
    assert_eq!(copied(&mut a, "copy-path").as_deref(), Some("/project/src/b.rs"));
    assert_eq!(copied(&mut a, "copy-relative-path").as_deref(), Some("src/b.rs"));
}

#[test]
fn reveal_shows_the_tab_file_in_the_tree() {
    let mut a = app(&["a.rs", "b.rs"], 0, &[]);
    a.tree.show = false;
    a.tab_menu_pick(1, "reveal", &mut Tx::new());
    assert!(a.tree.show);
    assert_eq!(a.tree.sel.as_deref(), Some("src/b.rs"));
}

#[test]
fn the_palette_closes_tabs_from_the_tab_that_shows() {
    let mut a = app(&["a", "b", "c", "d"], 2, &[]);
    a.run_command("close-right", &mut Tx::new());
    assert_eq!(tabs(&a), (names(&["a", "b", "c"]), "c".into()));
    a.run_command("close-others", &mut Tx::new());
    assert_eq!(tabs(&a), (names(&["c"]), "c".into()));
}
