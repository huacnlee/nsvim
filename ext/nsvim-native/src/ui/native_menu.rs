//! The platform's own menus: the context menu and the application menu. They are not views:
//! the platform draws them.

use std::cell::RefCell;
use std::rc::Rc;

use nsh_core::desktop::MenuRole;
use nsh_core::element::desktop::{MenuItem, popup_menu_at};
use nsh_core::prelude::*;

use crate::app::app_menu::Entry;
use crate::app::fx::{PopupMenu, Then};
use crate::store::Session;

/// Shows `menu` as the platform's own context menu at (`x`, `y`) in the window, then runs
/// `then` with the id of the picked item, or `None`.
pub fn native_menu(s: Session, menu: PopupMenu, x: f32, y: f32, then: Then<Option<String>>) {
    let picked: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let items = menu
        .items
        .into_iter()
        .map(|item| match item {
            None => MenuItem::separator(),
            Some(item) => {
                let picked = picked.clone();
                let id = item.id;
                MenuItem::new(item.title).disabled(!item.enabled).on_select(move || *picked.borrow_mut() = Some(id.clone()))
            }
        })
        .collect();
    let answer = popup_menu_at(items, point(px(x), px(y)));
    // The answer comes when the menu closes, after the picked item's `on_select`.
    spawn(async move {
        answer.await;
        let id = picked.borrow_mut().take();
        s.update(|a, tx| then(a, id, tx));
    });
}

/// The application menu's `entries` (`App::app_menu`) as the platform's menu items, each item
/// carried out on `s` (`App::app_menu_pick`).
pub fn app_menu_items(s: Session, entries: Vec<Entry>) -> Vec<MenuItem> {
    entries
        .into_iter()
        .map(|e| match e {
            Entry::Menu(label, items) => MenuItem::submenu(label, app_menu_items(s, items)),
            Entry::Item(id, label) if id == "none" => MenuItem::new(label).disabled(true),
            Entry::Item(id, label) => MenuItem::new(label).on_select(move || s.update(|a, tx| a.app_menu_pick(&id, tx))),
            Entry::Check(id, label, on) => MenuItem::new(label).checked(on).on_select(move || s.update(|a, tx| a.app_menu_pick(&id, tx))),
            Entry::Quit => MenuItem::role(MenuRole::Quit),
            Entry::Hide(label, key) => {
                let item = MenuItem::role(MenuRole::Hide).label(label);
                match key {
                    Some(key) => item.shortcut(&key),
                    None => item,
                }
            }
            Entry::HideOthers => MenuItem::role(MenuRole::HideOthers),
            Entry::ShowAll => MenuItem::role(MenuRole::ShowAll),
            Entry::Separator => MenuItem::separator(),
        })
        .collect()
}
