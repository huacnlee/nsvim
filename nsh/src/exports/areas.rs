//! The parts of the screen that Rust draws, and the modules that give TSX the app's state.
//!
//! Each component reads the session of the `Workspace` around it.

use nsh_core::prelude::*;

use super::current;
use crate::app::Screen as AppScreen;
use crate::ui;

/// The code with its gutter, cursor and selections. Rust draws it, as it changes each key.
#[component]
pub fn TextArea() -> impl IntoElement {
    let (s, keys) = current();
    ui::text_area::text_area(s, keys)
}

/// The line of open buffers above the text.
#[component]
pub fn BufferLine() -> impl IntoElement {
    ui::tabs::buffer_line(current().0)
}

/// The file tree beside the text.
#[component]
pub fn Tree() -> impl IntoElement {
    let (s, keys) = current();
    ui::tree_view::tree(s, keys)
}

/// The Settings tab, in place of the text area.
#[component]
pub fn Settings() -> impl IntoElement {
    let (s, keys) = current();
    ui::settings_view::settings(s, keys)
}

/// The status line under the text.
#[component]
pub fn StatusLine() -> impl IntoElement {
    ui::chrome::status_line(current().0)
}

/// The command line under the status line.
#[component]
pub fn CommandLine() -> impl IntoElement {
    ui::chrome::command_line(current().0)
}

/// The leader menu, the pickers, the popups and the dialogs, over the editor.
#[component]
pub fn Overlays() -> impl IntoElement {
    let (s, keys) = current();
    ui::overlays::overlays(s, keys)
}

/// First-launch setup, in place of the editor.
#[component]
pub fn Setup() -> impl IntoElement {
    let (s, keys) = current();
    ui::setup_view::setup(s, keys)
}

/// What the window shows.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct ScreenState {
    /// First-launch setup shows, in place of the editor.
    pub setup: bool,
    /// The Settings tab shows, in place of the text area.
    pub settings: bool,
    /// The file tree shows beside the text.
    pub tree: bool,
}

/// Which screen and panels the window shows.
#[module]
pub struct Shell;

#[module]
impl Shell {
    /// Calls `listener` with what the window shows, now and after each change.
    pub fn on_change(#[args(screen)] listener: Callback<ScreenState>) -> Stop {
        crate::bridge::subscribe(
            |a| ScreenState {
                setup: matches!(a.screen, AppScreen::Setup(_)),
                settings: a.on_settings,
                tree: a.tree.show && !a.on_settings,
            },
            listener,
        )
    }

    /// The user asked to close the window. The app decides: it quits, or warns about unsaved
    /// edits first.
    pub fn close_requested() {
        crate::bridge::send(|s| s.close_requested());
    }
}
