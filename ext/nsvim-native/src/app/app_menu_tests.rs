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
            Entry::Quit | Entry::Separator => {}
        }
    }
}

#[test]
fn every_command_item_names_a_palette_command() {
    let app = app();
    let mut all = Vec::new();
    ids(&app.app_menu(&[]), &mut all);
    for id in all {
        if let Some(cmd) = id.strip_prefix("cmd:") {
            assert!(cmd == "palette" || crate::app::picker::COMMANDS.iter().any(|(_, c)| *c == cmd), "{id}");
        }
        if let Some(e) = id.strip_prefix("edit:") {
            assert!(crate::app::menu::ITEMS.iter().any(|i| i.0 == e), "{id}");
        }
    }
}

#[test]
fn open_recent_lists_the_recent_projects() {
    let app = app();
    let menus = app.app_menu(&[PathBuf::from("/work/a"), PathBuf::from("/work/b")]);
    let Some(Entry::Menu(_, file)) = menus.iter().find(|m| matches!(m, Entry::Menu(t, _) if t == "File")) else { panic!() };
    let Some(Entry::Menu(_, recent)) = file.iter().find(|m| matches!(m, Entry::Menu(t, _) if t == "Open Recent")) else { panic!() };
    assert_eq!(
        recent,
        &vec![Entry::Item("recent:/work/a".into(), "/work/a".into()), Entry::Item("recent:/work/b".into(), "/work/b".into())]
    );
    let empty = app.app_menu(&[]);
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
