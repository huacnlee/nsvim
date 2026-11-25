use std::path::PathBuf;

use editor::editor::KeyInput;
use editor::syntax::highlight::Kind;

use super::picker::{hit_runs, preview_lines, preview_src, rows};
use super::{leader_part, menu_height};
use crate::app::fx::Tx;
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
    assert_eq!(rows(&a).len(), 2);
    type_query(&mut a, "nee");
    let r = rows(&a);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].dir, "src/");
    assert_eq!(r[0].name, "needle.txt");
    assert_eq!(r[0].hits, vec![0, 1, 2], "hits are indices into the name, not the path");
}

#[test]
fn a_key_row_keeps_its_section_apart() {
    let mut a = app();
    a.open_key_list(String::new());
    let r = rows(&a);
    assert!(r.len() > 300);
    assert!(r.iter().all(|r| !r.name.contains('\t') && !r.extra.is_empty()));
    assert!(r.iter().any(|r| r.extra == "Leader menu" && r.name.starts_with("SPC f")));
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
    assert!(lines[0].runs.contains(&("fn".to_string(), Some(Kind::Keyword))), "{:?}", lines[0].runs);
    let text: String = lines[0].runs.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(text, "fn main() {}");
    assert!(lines.iter().all(|l| !l.hit), "a file pick marks no line");
}

#[test]
fn commands_have_no_preview() {
    let mut a = app();
    a.open_picker(PickKind::Commands);
    assert!(preview_src(&a).is_none());
    assert!(rows(&a).iter().any(|r| r.name == "Show every key"));
}

#[test]
fn the_leader_menu_shows_the_group_it_is_in() {
    let mut a = app();
    a.overlay = Overlay::Leader(None);
    assert_eq!(leader_part(&a).title, "SPC");
    a.overlay = Overlay::Leader(Some('l'));
    let p = leader_part(&a);
    assert_eq!(p.title, "SPC l");
    assert!(p.items.iter().any(|i| i.1 == "references"));
}

#[test]
fn the_menu_height_counts_rows_and_separators() {
    assert_eq!(menu_height(), 8.0 + 9.0 * 24.0 + 2.0 * 9.0);
}
