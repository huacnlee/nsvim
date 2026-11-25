use std::path::PathBuf;

use editor::editor::KeyInput;

use super::parts::{OverlayKind, all_rows, head, hit_runs, kind, leader, list, menu, preview, preview_lines, preview_src};
use crate::app::fx::Tx;
use crate::app::menu::Menu;
use crate::app::{App, Overlay, PickKind};

fn app() -> App {
    let cfg = toml::from_str("[ipc]\nenabled = false\n").unwrap();
    App::new(cfg, PathBuf::from("/project"), false, None)
}

fn type_query(a: &mut App, q: &str) {
    for c in q.chars() {
        a.picker_key(KeyInput::ch(c), &mut Tx::new());
    }
}

fn down(a: &mut App, n: usize) {
    for _ in 0..n {
        a.picker_key(KeyInput::plain(editor::editor::KeyCode::Down), &mut Tx::new());
    }
}

#[test]
fn matched_chars_split_the_name_into_runs() {
    assert_eq!(hit_runs("needle.txt", &[0, 1, 2]), vec![("nee".to_string(), true), ("dle.txt".to_string(), false)]);
    assert_eq!(hit_runs("ab", &[1]), vec![("a".to_string(), false), ("b".to_string(), true)]);
    assert_eq!(hit_runs("é→x", &[1]), vec![("é".to_string(), false), ("→".to_string(), true), ("x".to_string(), false)]);
    assert!(hit_runs("", &[]).is_empty());
}

#[test]
fn a_file_row_shows_its_folder_apart_and_hits_count_in_the_name() {
    let mut a = app();
    a.files = vec!["src/needle.txt".into(), "other.md".into()];
    a.open_picker(PickKind::Files);
    assert_eq!(kind(&a), OverlayKind::Pick);
    assert_eq!(all_rows(&a).len(), 2);
    type_query(&mut a, "nee");
    let r = all_rows(&a);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].dir, "src/");
    assert_eq!(r[0].label, "src/needle.txt");
    let runs: Vec<(&str, bool)> = r[0].name.iter().map(|n| (n.text.as_str(), n.hit)).collect();
    assert_eq!(runs, [("nee", true), ("dle.txt", false)], "hits are indices into the name, not the path");
    assert_eq!(head(&a).count, "1 / 2");
}

#[test]
fn a_key_row_keeps_its_section_apart() {
    let mut a = app();
    a.open_key_list(String::new());
    let r = all_rows(&a);
    assert!(r.len() > 300);
    assert!(r.iter().all(|r| !r.label.contains('\t') && !r.extra.is_empty()));
    assert!(r.iter().any(|r| r.extra == "Leader menu" && r.label.starts_with("SPC f")));
    assert!(head(&a).wide, "the key list has no preview");
}

#[test]
fn the_list_sends_only_the_rows_on_screen_and_follows_the_picked_row() {
    let mut a = app();
    a.open_key_list(String::new());
    let total = all_rows(&a).len();
    if let Overlay::Pick(p) = &mut a.overlay {
        p.fit = 10;
    }
    let l = list(&a);
    assert_eq!(l.total as usize, total);
    assert_eq!(l.rows.len(), 11, "the rows that fit and one more");
    assert_eq!(l.rows[0].index, 0);
    down(&mut a, 15);
    let l = list(&a);
    assert_eq!(l.rows[0].index, 6, "the picked row is the last one that fits");
    // Up again inside the screen: the list stays.
    a.picker_key(KeyInput::plain(editor::editor::KeyCode::Up), &mut Tx::new());
    assert_eq!(list(&a).rows[0].index, 6);
    // Up past the first row on screen: the list follows one row at a time.
    for _ in 0..9 {
        a.picker_key(KeyInput::plain(editor::editor::KeyCode::Up), &mut Tx::new());
    }
    assert_eq!(list(&a).rows[0].index, 5);
}

#[test]
fn the_wheel_scrolls_the_list_and_keeps_the_picked_row() {
    let mut a = app();
    a.open_key_list(String::new());
    let total = all_rows(&a).len();
    if let Overlay::Pick(p) = &mut a.overlay {
        p.fit = 10;
        p.wheel(22.0 * 3.0, total);
    }
    assert_eq!(list(&a).rows[0].index, 3);
    // Small steps add up to a row.
    if let Overlay::Pick(p) = &mut a.overlay {
        p.wheel(11.0, total);
        p.wheel(11.0, total);
        assert_eq!(p.sel, 0, "the wheel leaves the picked row");
    }
    assert_eq!(list(&a).rows[0].index, 4);
    // The wheel stops at the ends.
    if let Overlay::Pick(p) = &mut a.overlay {
        p.wheel(-22.0 * 100.0, total);
    }
    assert_eq!(list(&a).rows[0].index, 0);
    if let Overlay::Pick(p) = &mut a.overlay {
        p.wheel(22.0 * 10_000.0, total);
    }
    assert_eq!(list(&a).rows.last().unwrap().index as usize, total - 1);
    // A key brings the picked row back on screen: row 1 is now the first one.
    down(&mut a, 1);
    assert_eq!(list(&a).rows[0].index, 1);
}

#[test]
fn the_preview_colours_the_picked_file() {
    let mut a = app();
    a.files = vec!["main.rs".into()];
    a.open_picker(PickKind::Files);
    let path = a.ed.root.join("main.rs");
    if let Overlay::Pick(p) = &mut a.overlay {
        p.preview = Some((path, "fn main() {}\nlet x = 1;\n".into()));
    }
    let src = preview_src(&a).expect("a file picker has a preview");
    let lines = preview_lines(&src);
    assert_eq!(lines[0].number, 1);
    assert_eq!(lines[1].number, 2);
    assert!(lines[0].runs.iter().any(|r| r.text == "fn" && r.token == "code-keyword"), "{:?}", lines[0].runs);
    let text: String = lines[0].runs.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(text, "fn main() {}");
    assert!(lines.iter().all(|l| !l.hit), "a file pick marks no line");
    // The part gives the same lines, and the title names the file.
    let part = preview()(&a);
    assert_eq!(part.title, "main.rs");
    assert_eq!(part.lines, lines);
}

#[test]
fn commands_have_no_preview() {
    let mut a = app();
    a.open_picker(PickKind::Commands);
    assert!(preview_src(&a).is_none());
    assert!(preview()(&a).lines.is_empty());
    assert!(all_rows(&a).iter().any(|r| r.label == "Show every key"));
}

#[test]
fn the_leader_menu_shows_the_group_it_is_in() {
    let mut a = app();
    a.overlay = Overlay::Leader(None);
    assert_eq!(kind(&a), OverlayKind::Leader);
    assert_eq!(leader(&a).title, "SPC");
    a.overlay = Overlay::Leader(Some('l'));
    let p = leader(&a);
    assert_eq!(p.title, "SPC l");
    assert!(p.items.iter().any(|i| i.label == "references"));
}

#[test]
fn the_menu_needs_a_selection_to_cut_and_copy() {
    let mut a = app();
    a.overlay = Overlay::Menu(Menu { x: 10.0, y: 20.0, sel: 2 });
    assert_eq!(kind(&a), OverlayKind::Menu);
    let m = menu(&a);
    assert_eq!((m.x, m.y, m.sel), (10.0, 20.0, 2));
    assert_eq!(m.items.iter().filter(|i| i.separator).count(), 2);
    let item = |id: &str| m.items.iter().find(|i| i.id == id).unwrap().clone();
    assert!(!item("cut").enabled && !item("copy").enabled);
    assert!(item("paste").enabled);
    assert_eq!(item("definition").hint, "gd", "the Neovim keymap's key");
}
