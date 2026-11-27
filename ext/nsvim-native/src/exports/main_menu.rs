//! `MainMenu`: the application menu (`app::app_menu`) from the menu button of the title bar
//! (`src/main-menu.tsx`). On macOS the menu bar at the top of the screen holds it. On Windows
//! and Linux the platform shows the window's menu bar only with the system's title bar, and
//! nsvim draws its own title bar. So there a press on the button shows the menus in a row in
//! the title bar, and a press on a menu opens its items, as a menu bar does.

use nsh_core::prelude::*;

/// Whether the title bar shows the menu button: a window with no menu bar of its own. The
/// terminal and the web have no application menu.
pub fn main_menu_shown() -> bool {
    !cfg!(target_os = "macos") && !cfg!(target_arch = "wasm32") && !crate::terminal()
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

    /// The names of the menus, in the order of the menu bar: NsVim, File, Edit, View, Go, Help.
    pub fn menus() -> Vec<String> {
        crate::app::app_menu::MENUS.iter().map(|m| m.to_string()).collect()
    }

    /// Opens the items of menu `index` (`menus()`) at `x`, `y` in the window, with the recent
    /// projects in File › Open Recent. The keys go back to the editor when it closes. True when
    /// an item was picked, false when the menu closed with no pick.
    pub async fn open_menu(index: u32, x: f64, y: f64) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (done, picked) = tokio::sync::oneshot::channel();
            crate::bridge::send(move |s| {
                let entries = s.read(|a| a.app_menu(&crate::app::session::recent(&a.ed.root), false));
                let items = match entries.into_iter().nth(index as usize) {
                    Some(crate::app::app_menu::Entry::Menu(_, items)) => items,
                    _ => Vec::new(),
                };
                crate::ui::native_menu::popup_menu_items(s, items, x as f32, y as f32, done);
            });
            picked.await.unwrap_or(false)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (index, x, y);
            false
        }
    }
}
