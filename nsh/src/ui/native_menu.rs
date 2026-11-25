//! The platform's own context menu. It is not a view: the platform draws it.

use std::cell::RefCell;
use std::rc::Rc;

use nsh_core::element::desktop::{MenuItem, popup_menu_at};
use nsh_core::prelude::*;

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
