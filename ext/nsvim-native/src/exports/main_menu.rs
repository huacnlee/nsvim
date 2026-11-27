//! `MainMenu`: the application menu (`app::app_menu`) from the menu button of the title bar
//! (`src/main-menu.tsx`). On macOS the menu bar at the top of the screen holds it. On Windows
//! and Linux the platform shows the window's menu bar only with the system's title bar, and
//! nsvim draws its own title bar. So there the button opens the same menus, one submenu each.

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

    /// Opens the application menu at `x`, `y` in the window: a submenu for each menu, with the
    /// recent projects in File › Open Recent. The keys go back to the editor when it closes.
    pub fn open(x: f64, y: f64) {
        #[cfg(not(target_arch = "wasm32"))]
        crate::bridge::send(move |s| {
            let entries = s.read(|a| a.app_menu(&crate::app::session::recent(&a.ed.root), false));
            crate::ui::native_menu::popup_app_menu(s, entries, x as f32, y as f32);
        });
        #[cfg(target_arch = "wasm32")]
        let _ = (x, y);
    }
}
