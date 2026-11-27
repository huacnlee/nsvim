//! The welcome page of a window with no project.

use std::path::PathBuf;

use crate::app::App;
use crate::app::fx::{Effect, Tx};
use crate::config::Config;

fn app() -> App {
    App::new(Config::default(), PathBuf::from("/home/dev"), false, None)
}

#[test]
fn a_window_with_a_project_shows_no_welcome_page() {
    let a = app();
    assert!(!a.shows_welcome());
    assert!(a.tree.show);
}

#[test]
fn a_window_with_no_project_shows_the_welcome_page_until_a_file_opens() {
    let mut a = app();
    a.start_welcome_with(vec![PathBuf::from("/work/nsh")]);
    assert!(a.shows_welcome());
    assert!(!a.tree.show, "the home folder shows no tree");
    let mut tx = Tx::new();
    a.app_menu_pick("cmd:new", &mut tx);
    assert!(a.ed.has_doc(), "New file opens a buffer");
    assert!(!a.shows_welcome());
    a.on_settings = true;
    assert!(!a.shows_welcome(), "Settings covers it");
}

#[test]
fn the_welcome_page_lists_the_first_things_to_do_with_their_keys() {
    let mut a = app();
    a.start_welcome_with(Vec::new());
    let rows = a.welcome_actions();
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["cmd:new", "cmd:folder", "cmd:project", "cmd:settings", "cmd:keys"]);
    assert_eq!(rows[0].label, "New file");
    assert!(!rows[0].keys.is_empty(), "{rows:?}");
    assert!(a.welcome_recent().is_empty());
}

#[test]
fn a_recent_project_opens_in_its_own_window() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = app();
    a.start_welcome_with((0..12).map(|i| dir.path().join(format!("p{i}"))).collect());
    let recent = a.welcome_recent();
    assert_eq!(recent.len(), 8, "the page lists eight at most");
    assert_eq!(recent[0].id, format!("recent:{}", dir.path().join("p0").display()));
    let mut tx = Tx::new();
    a.app_menu_pick(&format!("recent:{}", dir.path().display()), &mut tx);
    assert!(matches!(tx.effects.first(), Some(Effect::OpenProject(p)) if p == dir.path()));
    assert!(tx.effects.iter().any(|e| matches!(e, Effect::Quit)), "the empty welcome window gives its place");
}
