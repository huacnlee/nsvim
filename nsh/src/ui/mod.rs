//! The views that Rust draws. Each is a function of the `Session` that makes memos of the part
//! of the app it shows (`Session::view`) and binds to them. `App.tsx` places them.

pub mod font;
pub mod icons;
pub mod overlays;
pub mod popups;
pub mod review_view;
pub mod style;
pub mod text_area;

use crate::app::{App, Screen};
use crate::theme::{self, Palette};

/// The palette to paint with: the setup's pick while it runs, else config.toml's theme.
pub fn palette(app: &App, system_dark: bool) -> Palette {
    let cfg = match &app.screen {
        Screen::Setup(s) => &s.cfg,
        Screen::Editor => &app.cfg,
    };
    theme::resolve(theme::Source::parse(&cfg.theme.mode), &cfg.theme.dark, &cfg.theme.light, system_dark, app.omarchy.as_ref()).0
}
