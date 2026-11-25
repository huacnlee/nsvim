//! The interface font: the family and size from config.toml's `[ui]`, and the editor family.

use nsh_core::prelude::*;

use crate::app::App;
use crate::store::Session;

/// The interface font: the family and size from config.toml's `[ui]`, and the editor family
/// for code.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiFont {
    pub family: &'static str,
    pub size: f32,
    pub mono: &'static str,
}

impl UiFont {
    /// The font of `app`.
    pub fn of(a: &App) -> UiFont {
        UiFont { family: if a.cfg.ui.font == "system" { "system-ui" } else { a.font_family }, size: a.cfg.ui.size, mono: a.font_family }
    }
}

/// A memo of the interface font.
pub fn ui_font(s: Session) -> Memo<UiFont> {
    s.view(UiFont::of)
}

/// One line of interface text, `delta` points larger than the configured size. The chrome
/// has fixed heights, so its text follows config.toml's size and not the system text size.
pub fn ui_text(font: Memo<UiFont>, delta: f32, content: impl IntoReactive<SharedString>) -> Text {
    text(content)
        .font_family(move || SharedString::from(font.get().family))
        .text_size(move || px((font.get().size + delta).max(9.0)))
        .number_of_lines(1)
        .allow_font_scaling(false)
}
