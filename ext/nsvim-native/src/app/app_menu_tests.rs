use std::path::PathBuf;

use crate::app::App;
use crate::app::app_menu::Entry;
use crate::app::fx::Tx;

fn app() -> App {
    App::new(crate::config::Config::default(), PathBuf::from("/p"), false, None)
}

fn ids(entries: &[Entry], out: &mut Vec<String>) {
    for e in entries {
        match e {
            Entry::Menu(_, items) => ids(items, out),
            Entry::Item(id, _) => out.push(id.clone()),
            Entry::Quit | Entry::Hide(..) | Entry::HideOthers | Entry::ShowAll | Entry::Separator => {}
        }
    }
}

/// Linux and Windows hide the menu bar, so the palette must run every item of the menu.
#[test]
fn every_menu_item_is_a_palette_command() {
    let app = app();
    for mac in [false, true] {
        let mut all = Vec::new();
        ids(&app.app_menu(&[PathBuf::from("/work/a")], mac), &mut all);
        for id in all {
            if id == "cmd:palette" || id == "none" || id.starts_with("recent:") {
                continue;
            }
            let cmd = id.strip_prefix("cmd:").or_else(|| id.strip_prefix("edit:")).unwrap_or(&id);
            assert!(crate::app::palette::COMMANDS.iter().any(|c| c.id == cmd), "{id} is in the menu but not in the palette");
        }
    }
}

#[test]
fn open_recent_lists_the_recent_projects() {
    let app = app();
    let menus = app.app_menu(&[PathBuf::from("/work/a"), PathBuf::from("/work/b")], false);
    let Some(Entry::Menu(_, file)) = menus.iter().find(|m| matches!(m, Entry::Menu(t, _) if t == "File")) else { panic!() };
    let Some(Entry::Menu(_, recent)) = file.iter().find(|m| matches!(m, Entry::Menu(t, _) if t == "Open Recent")) else { panic!() };
    assert_eq!(
        recent,
        &vec![Entry::Item("recent:/work/a".into(), "/work/a".into()), Entry::Item("recent:/work/b".into(), "/work/b".into())]
    );
    let empty = app.app_menu(&[], false);
    let mut all = Vec::new();
    ids(&empty, &mut all);
    assert!(all.contains(&"none".to_string()));
}

#[test]
fn the_palette_item_opens_the_palette() {
    let mut app = app();
    let mut tx = Tx::new();
    app.app_menu_pick("cmd:palette", &mut tx);
    assert!(matches!(&app.overlay, crate::app::Overlay::Pick(p) if p.kind == crate::app::picker::PickKind::Commands));
}

fn close_window(menus: &[Entry]) -> Option<Entry> {
    let Some(Entry::Menu(_, file)) = menus.iter().find(|m| matches!(m, Entry::Menu(t, _) if t == "File")) else { panic!() };
    file.iter().find(|e| matches!(e, Entry::Hide(l, _) if l == "Close Window")).cloned()
}

#[test]
fn on_a_mac_cmd_w_hides_the_app_and_does_not_quit_it() {
    let mut app = app();
    app.ed.keymap = editor::editor::Keymap::Vim;
    assert_eq!(close_window(&app.app_menu(&[], true)), Some(Entry::Hide("Close Window".into(), Some("mod+w".into()))));
    let Some(Entry::Menu(_, first)) = app.app_menu(&[], true).into_iter().next() else { panic!() };
    assert!(first.contains(&Entry::Hide("Hide NsVim".into(), None)) && first.contains(&Entry::Quit));
    // Off a Mac there is no Hide, and Ctrl W stays Vim's window key.
    assert_eq!(close_window(&app.app_menu(&[], false)), None);
}

#[test]
fn close_window_leaves_cmd_w_to_the_classic_keymap_and_settings() {
    let mut app = app();
    app.ed.keymap = editor::editor::Keymap::Classic;
    assert_eq!(app.close_window_key(), "mod+shift+w", "the classic keymap closes the tab with ⌘ W");
    app.ed.keymap = editor::editor::Keymap::Vim;
    assert_eq!(app.close_window_key(), "mod+w");
    app.on_settings = true;
    assert_eq!(app.close_window_key(), "mod+shift+w", "⌘ W closes Settings");
}
