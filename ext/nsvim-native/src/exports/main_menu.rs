//! `MainMenu`: the application menu (`app::app_menu`) from the menu button of the title bar
//! (`src/main-menu.tsx`). On macOS the menu bar at the top of the screen holds it. On Windows
//! and Linux the platform shows the window's menu bar only with the system's title bar, and
//! nsvim draws its own title bar. So there a press on the button shows the menus in a row in
//! the title bar, and the row draws them as a menu bar does (`nsh/base`'s `Menu` in a
//! `Toolbar`): with one open, the pointer over another name opens that one, and a submenu opens
//! beside its item.

use nsh_core::prelude::*;

use crate::app::app_menu::Entry;

/// Whether the title bar shows the menu button: a window with no menu bar of its own. The
/// terminal and the web have no application menu.
pub fn main_menu_shown() -> bool {
    !cfg!(target_os = "macos") && !cfg!(target_arch = "wasm32") && !crate::terminal()
}

/// An item of a submenu.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct MainMenuSubItem {
    /// What `MainMenu.pick` takes.
    pub id: String,
    pub label: String,
    /// A separator, with no id or label.
    pub separator: bool,
    /// It can be picked.
    pub enabled: bool,
}

/// An item of a menu: an item, an item with a check mark, a separator or a submenu.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct MainMenuItem {
    /// What `MainMenu.pick` takes.
    pub id: String,
    pub label: String,
    /// A separator, with no id or label.
    pub separator: bool,
    /// It can be picked.
    pub enabled: bool,
    /// An item that turns something on or off shows whether it is on; others have no mark.
    pub checkable: bool,
    pub checked: bool,
    /// A submenu's items; empty for the other kinds.
    pub items: Vec<MainMenuSubItem>,
}

/// A menu of the row.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct MainMenuList {
    /// NsVim, File, Edit, View, Go or Help.
    pub name: String,
    pub items: Vec<MainMenuItem>,
}

/// `entries` as a submenu's items.
fn sub_items(entries: Vec<Entry>) -> Vec<MainMenuSubItem> {
    entries
        .into_iter()
        .filter_map(item)
        .map(|i| MainMenuSubItem { id: i.id, label: i.label, separator: i.separator, enabled: i.enabled })
        .collect()
}

/// `e` as an item of the row; `None` for the Mac's window items.
fn item(e: Entry) -> Option<MainMenuItem> {
    let plain = |id: String, label: String| MainMenuItem {
        enabled: id != "none",
        id,
        label,
        separator: false,
        checkable: false,
        checked: false,
        items: Vec::new(),
    };
    Some(match e {
        Entry::Item(id, label) => plain(id, label),
        Entry::Check(id, label, on) => MainMenuItem { checkable: true, checked: on, ..plain(id, label) },
        Entry::Menu(label, items) => MainMenuItem { items: sub_items(items), ..plain(String::new(), label) },
        Entry::Quit => plain(QUIT.into(), "Quit".into()),
        Entry::Separator => MainMenuItem { separator: true, ..plain(String::new(), String::new()) },
        Entry::Hide(..) | Entry::HideOthers | Entry::ShowAll => return None,
    })
}

/// The id of Quit, which asks first as the close button does.
const QUIT: &str = "quit";

/// The menus of the row, with the recent projects in File › Open Recent and the check marks
/// as the app has them now.
pub fn main_menus(app: &crate::app::App, recent: &[std::path::PathBuf]) -> Vec<MainMenuList> {
    app.app_menu(recent, false)
        .into_iter()
        .filter_map(|e| match e {
            Entry::Menu(name, items) => Some(MainMenuList { name, items: items.into_iter().filter_map(item).collect() }),
            _ => None,
        })
        .collect()
}

/// The application menu in the title bar.
#[module]
pub struct MainMenu;

#[module]
impl MainMenu {
    /// Whether the title bar shows the menu button.
    pub fn shown() -> bool {
        main_menu_shown()
    }

    /// The menus, in the order of the menu bar: NsVim, File, Edit, View, Go, Help. The row asks
    /// again each time a menu opens, so the check marks and the recent projects are fresh.
    pub async fn menus() -> Vec<MainMenuList> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (done, menus) = tokio::sync::oneshot::channel();
            crate::bridge::send(move |s| {
                let _ = done.send(s.read(|a| main_menus(a, &crate::app::session::recent(&a.ed.root))));
            });
            menus.await.unwrap_or_default()
        }
        #[cfg(target_arch = "wasm32")]
        {
            Vec::new()
        }
    }

    /// Carries out the item `id` of a menu, as the same item of the menu bar does.
    pub fn pick(id: String) {
        crate::bridge::update(move |a, tx| if id == QUIT { a.close_requested(tx) } else { a.app_menu_pick(&id, tx) });
    }
}
