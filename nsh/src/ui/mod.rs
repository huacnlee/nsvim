//! The views. Each is a function of the `Session` that makes memos of the part of the app it
//! shows (`Session::view`) and binds to them. `shell` picks the screen: setup on first launch,
//! else the editor.

pub mod chrome;
pub mod icons;
pub mod overlays;
pub mod popups;
pub mod review_view;
pub mod settings_controls;
pub mod settings_preview;
pub mod settings_view;
pub mod setup_steps;
pub mod setup_view;
pub mod style;
pub mod text_area;
pub mod tree_view;

use nsh_core::prelude::*;

use crate::app::{App, Screen};
use crate::store::Session;
use crate::theme::{self, Palette};
use style::color;

/// The palette to paint with: the setup's pick while it runs, else config.toml's theme.
pub fn palette(app: &App, system_dark: bool) -> Palette {
    let cfg = match &app.screen {
        Screen::Setup(s) => &s.cfg,
        Screen::Editor => &app.cfg,
    };
    theme::resolve(theme::Source::parse(&cfg.theme.mode), &cfg.theme.dark, &cfg.theme.light, system_dark, app.omarchy.as_ref()).0
}

/// The window's content. `keys` is the view that takes every key.
pub fn shell(s: Session, keys: ElementRef) -> impl IntoElement {
    let in_setup = s.view(|a| matches!(a.screen, Screen::Setup(_)));
    when(in_setup, move || setup_view::setup(s, keys)).otherwise(move || editor(s, keys))
}

/// The editor screen: the buffer line, the tree beside the text area (or Settings), the status
/// line and the command line, with the overlays over them.
fn editor(s: Session, keys: ElementRef) -> impl IntoElement {
    let on_settings = s.view(|a| a.on_settings);
    let tree_shows = s.view(|a| a.tree.show && !a.on_settings);
    view()
        .flex_1()
        .bg(move || color("background"))
        .child(
            view()
                .flex_row()
                .flex_1()
                .min_h(px(0.0))
                .child(when(tree_shows, move || tree_view::tree(s, keys)))
                .child(when(on_settings, move || settings_view::settings(s, keys)).otherwise(move || text_area::text_area(s, keys))),
        )
        .child(chrome::status_line(s))
        .child(chrome::command_line(s))
        .child(overlays::overlays(s, keys))
}
